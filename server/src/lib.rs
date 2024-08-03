use std::sync::Arc;

use axum::{
    extract::FromRef,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_cookies::Key;

pub mod api;
pub mod auth;
pub mod pages;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<Vec<String>>,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        serde_json::to_string(&self)
            .expect("ErrorResponse as json")
            .into_response()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub cookie_secret: String,
}

impl Config {
    pub fn load_config() -> Result<Self, ConfigError> {
        log::info!("getting config file");
        let config_file = std::fs::read_to_string("config.toml")?;
        toml::from_str::<Config>(&config_file).map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "gender")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

impl core::fmt::Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gender::Male => write!(f, "male"),
            Gender::Female => write!(f, "female"),
        }
    }
}

impl PartialEq<str> for Gender {
    fn eq(&self, other: &str) -> bool {
        self.to_string() == other
    }
}

impl<'a> PartialEq<&'a str> for Gender {
    fn eq(&self, other: &&'a str) -> bool {
        self.to_string() == *other
    }
}

pub type NodeId = usize;

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    id: NodeId,
    name: String,
    mother_name: String,
    father_name: String,
    children: Vec<Node>,
}

impl Node {
    pub fn new(
        id: NodeId,
        name: String,
        father_name: String,
        mother_name: String,
        children: Vec<Node>,
    ) -> Self {
        Self {
            id,
            name,
            children,
            mother_name,
            father_name,
        }
    }
}

pub struct InnerAppState {
    pub db_pool: PgPool,
    pub cookies_secret: Key,
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub inner: Arc<InnerAppState>,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
