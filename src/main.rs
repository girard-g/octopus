use axum_extra::extract::cookie::Key;
use octopus::app::{connect_and_migrate, serve, AppState};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let secret = std::env::var("SESSION_SECRET").expect("SESSION_SECRET must be set");
    assert!(secret.len() >= 64, "SESSION_SECRET must be at least 64 bytes");

    let pool = connect_and_migrate(&database_url).await.expect("db connect/migrate failed");
    let key = Key::from(secret.as_bytes());

    serve(AppState { pool, key }, port).await;
}
