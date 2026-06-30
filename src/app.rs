use axum::extract::FromRef;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::Key;
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;

use crate::auth::{login, logout};
use crate::routes::contacts;
use crate::routes::dashboard;
use crate::routes::projects;
use crate::routes::tasks;

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
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/contacts", get(contacts::list).post(contacts::create))
        .route(
            "/api/contacts/{id}",
            get(contacts::get).put(contacts::update).delete(contacts::delete),
        )
        .route("/api/projects", get(projects::list).post(projects::create))
        .route(
            "/api/projects/{id}",
            get(projects::get).put(projects::update).delete(projects::delete),
        )
        .route("/api/projects/{id}/move", patch(projects::move_))
        .route("/api/tasks", get(tasks::list).post(tasks::create))
        .route("/api/tasks/{id}", axum::routing::put(tasks::update).delete(tasks::delete))
        .route("/api/dashboard", get(dashboard::get))
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
