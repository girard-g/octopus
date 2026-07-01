mod helpers;
use axum::http::StatusCode;
use chrono::{Duration, Utc};
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
    send(&app, json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"Archived one","status":"archived"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/tasks", json!({"title":"Open task"})).with_cookie(&cookie)).await;

    let (status, d) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(d["active_projects"].as_array().unwrap().len(), 1);
    assert_eq!(d["counts"]["projects"], 2);
    assert_eq!(d["counts"]["active"], 1);
    assert_eq!(d["counts"]["open_tasks"], 1);
    assert!(d["upcoming_events"].is_array());
}

#[sqlx::test]
async fn dashboard_upcoming_events_excludes_past(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let fmt = |dt: chrono::DateTime<Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let now = Utc::now();
    let future_start = fmt(now + Duration::days(1));
    let future_end = fmt(now + Duration::days(1) + Duration::hours(1));
    let past_start = fmt(now - Duration::days(2));
    let past_end = fmt(now - Duration::days(2) + Duration::hours(1));

    send(
        &app,
        json_req("POST", "/api/events", json!({"title":"FutureEvent","starts_at":future_start,"ends_at":future_end}))
            .with_cookie(&cookie),
    )
    .await;
    send(
        &app,
        json_req("POST", "/api/events", json!({"title":"PastEvent","starts_at":past_start,"ends_at":past_end}))
            .with_cookie(&cookie),
    )
    .await;

    let (status, d) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let upcoming = d["upcoming_events"].as_array().unwrap();
    assert!(upcoming.iter().any(|e| e["title"] == "FutureEvent"), "future event missing from upcoming_events");
    assert!(!upcoming.iter().any(|e| e["title"] == "PastEvent"), "past event must not appear in upcoming_events");
}

#[sqlx::test]
async fn dashboard_due_tasks_have_project_title(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"title":"Acme"})).with_cookie(&cookie)).await;
    let pid = p["id"].as_str().unwrap();
    send(&app, json_req("POST", "/api/tasks", json!({"title":"T","project_id":pid})).with_cookie(&cookie)).await;

    let (_, dash) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    let due = &dash["due_tasks"];
    assert_eq!(due[0]["project_title"], "Acme");
}
