use argon2::{password_hash::SaltString, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Duration;
use chrono::Utc;
use garde::Validate;
use sqlx::prelude::FromRow;
use std::sync::Arc;
use tower_cookies::cookie::time::OffsetDateTime;
use tower_cookies::Cookie;
use uuid::Uuid;

use argon2::Argon2;
use axum::{extract::State, Json};
use tower_cookies::Cookies;

use crate::{
    api::sessions::{models::CreateSession, SESSION_COOKIE_NAME},
    api::users::{
        models::{CreateUser, UserLogin, UserResponse, UserResponseBrief, UserRole},
        UsersError,
    },
    auth::AuthExtractor,
    InnerAppState,
};

#[axum::debug_handler]
pub async fn login(
    State(state): State<Arc<InnerAppState>>,
    cookies: Cookies,
    Json(payload): Json<UserLogin>,
) -> Result<(), UsersError> {
    payload.validate()?;

    if let Some(session_id) = cookies
        .private(&state.cookies_secret)
        .get(SESSION_COOKIE_NAME)
    {
        if let Some(session) = sqlx::query!(
            r#"
SELECT id from sessions
WHERE sessions.id = $1
            "#,
            Uuid::parse_str(session_id.value()).map_err(|e| {
                log::error!("{e}");
                UsersError::InternalServerError
            })?
        )
        .fetch_optional(&state.db_pool)
        .await?
        {
            log::error!("user already logged in with session: {:?}", session);
            return Err(UsersError::AlreadyLoggedIn);
        }
    }

    let argon2 = Argon2::default();

    #[derive(FromRow)]
    pub struct UserRow {
        pub id: Uuid,
        pub password: String,
    }

    let Some(user) = sqlx::query_as!(
        UserRow,
        r#"
SELECT users.id, users.password FROM users
WHERE users.email = $1
        "#,
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await?
    else {
        return Err(UsersError::UserNotFound);
    };

    let parsed_password = PasswordHash::new(&user.password)?;

    if argon2
        .verify_password(payload.password.as_bytes(), &parsed_password)
        .is_err()
    {
        return Err(UsersError::InvalidCredentials);
    }

    let now = Utc::now();
    let time_now = OffsetDateTime::now_utc();

    let new_session = CreateSession {
        id: Uuid::new_v4(),
        user_id: user.id,
        created_at: now,
        expires_at: now + Duration::days(2),
    };

    #[derive(FromRow)]
    struct SessionRow {
        id: Uuid,
    }

    let session = sqlx::query_as!(
        SessionRow,
        r#"
INSERT INTO sessions (id, user_id, created_at, expires_at)
VALUES ($1, $2, $3, $4)
RETURNING sessions.id
        "#,
        new_session.id,
        new_session.user_id,
        new_session.created_at,
        new_session.expires_at,
    )
    .fetch_one(&state.db_pool)
    .await?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.to_string()))
        .path("/")
        .expires(time_now + time::Duration::days(2))
        .http_only(true);

    #[cfg(not(debug_assertions))]
    let cookie = cookie.secure(true);

    let cookie = cookie.build();

    cookies.private(&state.cookies_secret).add(cookie);

    Ok(())
}

pub async fn me(auth: AuthExtractor<{ UserRole::User as u8 }>) -> Json<UserResponseBrief> {
    Json(auth.current_user)
}

pub async fn logout(
    cookies: Cookies,
    State(state): State<Arc<InnerAppState>>,
    auth: AuthExtractor<{ UserRole::User as u8 }>,
) -> Result<(), UsersError> {
    sqlx::query!(
        r#"
DELETE FROM sessions WHERE sessions.id = $1
        "#,
        auth.session_id,
    )
    .execute(&state.db_pool)
    .await?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true);

    #[cfg(not(debug_assertions))]
    let cookie = cookie.secure(true);

    cookies.remove(cookie.build());

    Ok(())
}

pub async fn create_user(
    State(state): State<Arc<InnerAppState>>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, UsersError> {
    payload.validate()?;

    if sqlx::query!(
        r#"
SELECT id, role as "role: UserRole" FROM users
WHERE role = $1
        "#,
        UserRole::Admin as _,
    )
    .fetch_optional(&state.db_pool)
    .await?
    .is_some()
    {
        return Err(UsersError::BadRequest);
    }

    if payload.username.is_empty() || payload.password.is_empty() || payload.email.is_empty() {
        return Err(UsersError::BadRequest);
    }

    let salt = SaltString::generate(rand::thread_rng());

    let argon2 = Argon2::default();

    let hashed_password = argon2
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    let user = sqlx::query_as!(
        UserResponse,
        r#"
INSERT INTO users (id, first_name, last_name, username, email, password, role, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING id, username, email, role as "role: UserRole"
        "#,
        Uuid::new_v4(),
        payload.first_name,
        payload.last_name,
        payload.username.to_lowercase(),
        payload.email,
        hashed_password,
        UserRole::Admin as _,
        Utc::now(),
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(user))
}
