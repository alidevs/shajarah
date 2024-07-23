use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub mod api;

#[derive(Debug, Clone, Copy, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "gender")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
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

pub struct AppState {
    pub db_pool: PgPool,
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
