pub mod models;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use chrono::{Duration, Utc};
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{AppState, ErrorResponse, InnerAppState};

pub const SESSION_COOKIE_NAME: &str = "session_id";

pub struct UserSession {
    pub session_id: Option<Uuid>,
}

#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("something went wrong")]
    SomethingWentWrong,

    #[error("something went wrong")]
    Sqlx(#[from] sqlx::Error),

    #[error("invalid session")]
    InvalidSession,
}

impl IntoResponse for SessionError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:#?}", self);

        match self {
            SessionError::SomethingWentWrong => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            SessionError::InvalidSession => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            SessionError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for UserSession {
    type Rejection = SessionError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies =
            parts
                .extract::<Cookies>()
                .await
                .map_err(|(_error_status, error_message)| {
                    log::error!(
                        "session-extractor: failed to get private cookie jar: {error_message}"
                    );
                    SessionError::InvalidSession
                })?;

        if let Some(session_id) = cookies
            .private(&state.inner.cookies_secret)
            .get(SESSION_COOKIE_NAME)
        {
            Ok(Self {
                session_id: Some(Uuid::parse_str(session_id.value()).map_err(|e| {
                    log::error!("session-extractor: invalid session_id: {e}");
                    SessionError::InvalidSession
                })?),
            })
        } else {
            Ok(Self { session_id: None })
        }
    }
}

pub async fn refresh_session(
    session: UserSession,
    State(state): State<Arc<InnerAppState>>,
    request: Request,
    next: Next,
) -> Result<Response, SessionError> {
    log::info!("running refresh_session middleware");

    if let Some(session_id) = session.session_id {
        sqlx::query!(
            r#"
                UPDATE sessions
                SET expires_at = $1
                WHERE id = $2 AND expires_at = $3
            "#,
            Utc::now() + Duration::days(2),
            session_id,
            Utc::now(),
        )
        .execute(&state.db_pool)
        .await?;
    }

    Ok(next.run(request).await)
}
