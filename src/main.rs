use axum_extra::extract::cookie::Key;
use octopus::app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()),
    ).init();
    let _ = (Key::generate(), std::any::type_name::<app::AppState>());
    println!("octopus: run Task 2 to wire the database and serve");
}
