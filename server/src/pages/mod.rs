use std::sync::Arc;

use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};

use crate::{
    api::{
        members::{routes::get_members_flat, MemberResponseBrief, MembersError},
        users::models::UserRole,
    },
    auth::{AuthError, AuthExtractor},
    InnerAppState,
};

mod filters {
    pub fn deref_i32(s: &i32) -> ::askama::Result<i32> {
        Ok(*s)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PagesError {
    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error(transparent)]
    Members(#[from] MembersError),
}

impl IntoResponse for PagesError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:#?}", self);

        match self {
            PagesError::Auth(e) => e.into_response(),
            PagesError::Members(e) => e.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    name: String,
    members: Vec<MemberResponseBrief>,
}

pub async fn admin_page(
    auth: Result<AuthExtractor<{ UserRole::Admin as u8 }>, AuthError>,
    state: State<Arc<InnerAppState>>,
) -> Result<impl IntoResponse, PagesError> {
    match auth {
        Ok(auth) => {
            let members = get_members_flat(state).await?;
            Ok(AdminTemplate {
                name: auth.current_user.username,
                members: members.0.into_iter().rev().collect(),
            }
            .into_response())
        }
        Err(e) => match e {
            AuthError::InvalidSession | AuthError::SessionError(_) => {
                return Ok(Redirect::to("/login").into_response());
            }
            e => return Err(e.into()),
        },
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate;

pub async fn login_page() -> LoginTemplate {
    LoginTemplate
}
