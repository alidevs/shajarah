use argon2::{password_hash::SaltString, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Duration;
use chrono::Utc;
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
    // TODO: add Result<Json<UserLogin>> and handle error

    // payload.validate(&())?;

    if let Some(session_id) = cookies
        .private(&state.cookies_secret)
        .get(SESSION_COOKIE_NAME)
    {
        if let Some(session) = sqlx::query(
            r#"
SELECT id from sessions
WHERE sessions.id = $1
            "#,
        )
        .bind(Uuid::parse_str(session_id.value()).map_err(|e| {
            log::error!("{e}");
            UsersError::InternalServerError
        })?)
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

    let Some(user): Option<UserRow> = sqlx::query_as(
        r#"
SELECT users.id, users.password FROM users
WHERE users.email = $1
        "#,
    )
    .bind(payload.email)
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

    let session: SessionRow = sqlx::query_as(
        r#"
INSERT INTO sessions (id, user_id, created_at, expires_at)
VALUES ($1, $2, $3, $4)
RETURNING sessions.id
        "#,
    )
    .bind(new_session.id)
    .bind(new_session.user_id)
    .bind(new_session.created_at)
    .bind(new_session.expires_at)
    .fetch_one(&state.db_pool)
    .await?;

    #[allow(unused_mut)]
    let mut cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.to_string()))
        .path("/")
        .expires(time_now + time::Duration::days(2))
        .http_only(true);

    #[cfg(not(debug_assertions))]
    {
        cookie = cookie
            // TODO: use the actual domain
            .domain("salmanforgot.com")
            .secure(true);
    }

    #[cfg(debug_assertions)]
    {
        cookie = cookie.domain("localhost");
    }

    cookies.private(&state.cookies_secret).add(cookie.build());

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
    sqlx::query(
        r#"
DELETE FROM sessions WHERE sessions.id = $1
        "#,
    )
    .bind(auth.session_id)
    .execute(&state.db_pool)
    .await?;
    // diesel::delete(sessions::table.filter(sessions::id.eq(auth.session_id)))
    //     .execute(&mut db)
    //     .await?;

    let mut cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true);

    #[cfg(not(debug_assertions))]
    {
        cookie = cookie
            // TODO: use the actual musawarah domain
            .domain("salmanforgot.com")
            .secure(true);
    }

    #[cfg(debug_assertions)]
    {
        cookie = cookie.domain("localhost");
    }

    cookies.remove(cookie.build());

    Ok(())
}

pub async fn create_user(
    State(state): State<Arc<InnerAppState>>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<UserResponse>, UsersError> {
    // payload.validate(&())?;
    if payload.username.is_empty() || payload.password.is_empty() || payload.email.is_empty() {
        return Err(UsersError::BadRequest);
    }

    let salt = SaltString::generate(rand::thread_rng());

    let argon2 = Argon2::default();

    let hashed_password = argon2
        .hash_password(payload.password.as_bytes(), &salt)?
        .to_string();

    let user: UserResponse = sqlx::query_as(
        r#"
INSERT INTO users (id, first_name, last_name, username, email, password, role, created_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING id, username, email, role as "role: UserRole"
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.first_name)
    .bind(payload.last_name)
    .bind(payload.username.to_lowercase())
    .bind(payload.email)
    .bind(hashed_password)
    .bind(UserRole::Admin)
    .bind(Utc::now())
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(user))
}
