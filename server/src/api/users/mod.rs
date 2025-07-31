use axum::{http::StatusCode, response::IntoResponse};

use crate::ErrorResponse;

pub mod models;
pub mod routes;

#[derive(thiserror::Error, Debug)]
pub enum UsersError {
    #[error("internal server error")]
    InternalServerError,

    #[error("something went wrong")]
    Sqlx(#[from] sqlx::Error),

    #[error("user not found")]
    UserNotFound,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("bad request")]
    BadRequest,

    #[error("user has no posts")]
    HasNoPosts,

    #[error("already logged in")]
    AlreadyLoggedIn,

    #[error(transparent)]
    Argon2(#[from] argon2::password_hash::Error),

    #[error("{0}")]
    Conflict(String),

    #[error(transparent)]
    Garde(#[from] garde::Report),
}

impl IntoResponse for UsersError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:#?}", self);

        match self {
            UsersError::UserNotFound => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::HasNoPosts => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::BadRequest => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::Conflict(_) => (
                StatusCode::CONFLICT,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            UsersError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            UsersError::Argon2(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            UsersError::AlreadyLoggedIn => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: self.to_string(),
                    ..Default::default()
                },
            )
                .into_response(),
            UsersError::Garde(_) => (StatusCode::BAD_REQUEST).into_response(),
        }
    }
}
