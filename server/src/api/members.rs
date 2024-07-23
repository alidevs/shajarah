use std::sync::Arc;

use axum::{extract::State, Json};

use crate::{AppError, AppState, Node};

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<AppState>>,
) -> anyhow::Result<Json<Node>, AppError> {
    Ok(Json(state.root_member.clone()))
}
