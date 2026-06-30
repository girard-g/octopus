mod helpers;
use axum::http::StatusCode;
use helpers::{json_req, login, send, test_app, WithCookie};
use serde_json::json;

#[sqlx::test]
async fn dashboard_aggregates(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (_, c) = send(&app, json_req("POST", "/api/contacts", json!({"kind":"company","name":"Acme"})).with_cookie(&cookie)).await;
    let cid = c["id"].as_str().unwrap().to_string();
    send(&app, json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"Active one","status":"active"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"A lead","status":"lead"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/tasks", json!({"title":"Open task"})).with_cookie(&cookie)).await;

    let (status, d) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(d["active_projects"].as_array().unwrap().len(), 1);
    assert_eq!(d["counts"]["leads"], 1);
    assert_eq!(d["counts"]["active"], 1);
    assert_eq!(d["counts"]["open_tasks"], 1);
    assert!(d["upcoming_events"].is_array());
}
