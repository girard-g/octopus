use axum::extract::FromRef;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::cookie::Key;
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;

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

pub async fn connect_and_migrate(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub async fn serve(state: AppState, port: u16) {
    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}
