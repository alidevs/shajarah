use askama::Template;
use axum::response::{IntoResponse, Redirect};

use crate::{
    api::users::models::UserRole,
    auth::{AuthError, AuthExtractor},
};

#[derive(thiserror::Error, Debug)]
pub enum PagesError {
    #[error(transparent)]
    Auth(#[from] AuthError),
}

impl IntoResponse for PagesError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:#?}", self);

        match self {
            PagesError::Auth(e) => e.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct AdminTemplate {
    name: String,
}

pub async fn admin_page(
    auth: Result<AuthExtractor<{ UserRole::Admin as u8 }>, AuthError>,
) -> Result<impl IntoResponse, PagesError> {
    match auth {
        Ok(auth) => Ok(AdminTemplate {
            name: auth.current_user.username,
        }
        .into_response()),
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
