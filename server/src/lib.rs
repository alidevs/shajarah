use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

pub mod api;

pub type NodeId = usize;

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    id: NodeId,
    name: String,
    children: Vec<Node>,
}

impl Node {
    pub fn new(id: NodeId, name: String, children: Vec<Node>) -> Self {
        Self { id, name, children }
    }
}

pub struct AppState {
    pub root_member: Node,
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
