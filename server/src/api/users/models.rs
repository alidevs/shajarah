use chrono::DateTime;
use garde::Validate;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "UserRole", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin = 0,
    User = 1,
}

#[derive(Debug)]
pub struct ProfileImage {
    pub id: Uuid,
    pub path: String,
    pub content_type: String,
    pub user_id: Uuid,
    pub updated_at: Option<DateTime<chrono::Utc>>,
}

#[derive(Deserialize, Validate)]
pub struct CreateUser {
    #[garde(skip)]
    pub username: String,
    #[garde(skip)]
    pub first_name: String,
    #[garde(skip)]
    pub last_name: String,
    #[garde(email)]
    pub email: String,
    // TODO: add password rules
    #[garde(skip)]
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct CreateUserReponse {
    pub user_id: Uuid,
}

#[derive(Deserialize, Validate)]
pub struct UserLogin {
    #[garde(email)]
    pub email: String,
    // TODO: add password rules
    #[garde(skip)]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponseBrief {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserClaims {
    pub user: UserResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserToken {
    pub access_token: String,
    pub r#type: String,
}
