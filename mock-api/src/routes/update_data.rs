use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::{AppError, AppState};

pub async fn handle(state: State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    match state.db.update() {
        Ok(()) => Ok(Json(json!({"status": "database updated."}))),
        Err(_e) => Err(AppError::InternalServerError(
            "failed to update database".to_string(),
        )),
    }
}
