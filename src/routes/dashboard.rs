use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Project, Task};

#[derive(Serialize)]
pub struct Counts {
    pub leads: i64,
    pub active: i64,
    pub open_tasks: i64,
}

#[derive(Serialize)]
pub struct Dashboard {
    pub active_projects: Vec<Project>,
    pub due_tasks: Vec<Task>,
    pub counts: Counts,
}

pub async fn get(_: AuthUser, State(s): State<AppState>) -> Result<Json<Dashboard>, AppError> {
    let active_projects = sqlx::query_as::<_, Project>(
        "select * from project where status = 'active' order by board_order, created_at",
    )
    .fetch_all(&s.pool)
    .await?;

    let due_tasks = sqlx::query_as::<_, Task>(
        "select * from task where status <> 'done' order by due_on nulls last, created_at limit 20",
    )
    .fetch_all(&s.pool)
    .await?;

    let leads: i64 = sqlx::query_scalar("select count(*) from project where status = 'lead'")
        .fetch_one(&s.pool)
        .await?;
    let active: i64 = sqlx::query_scalar("select count(*) from project where status = 'active'")
        .fetch_one(&s.pool)
        .await?;
    let open_tasks: i64 = sqlx::query_scalar("select count(*) from task where status <> 'done'")
        .fetch_one(&s.pool)
        .await?;

    Ok(Json(Dashboard {
        active_projects,
        due_tasks,
        counts: Counts { leads, active, open_tasks },
    }))
}
