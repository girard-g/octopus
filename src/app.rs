use axum::extract::FromRef;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::cookie::Key;
use serde_json::json;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
