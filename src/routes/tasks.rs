use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Task, TaskInput, PRIORITY_LEVELS, TASK_SIZES, TASK_STATUSES, TASK_TYPES};

#[derive(Deserialize)]
pub struct ListQuery {
    pub project_id: Option<Uuid>,
}

fn check_status(status: &str) -> Result<(), AppError> {
    if TASK_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::BadRequest("invalid task status".into()))
    }
}

fn check_optional<const N: usize>(v: &Option<String>, allowed: [&str; N], what: &str) -> Result<(), AppError> {
    match v {
        Some(s) if !allowed.contains(&s.as_str()) => {
            Err(AppError::BadRequest(format!("invalid task {what}")))
        }
        _ => Ok(()),
    }
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Task>>, AppError> {
    let rows = match q.project_id {
        Some(pid) => sqlx::query_as::<_, Task>(
            "select * from task where project_id = $1 order by position, created_at",
        )
        .bind(pid)
        .fetch_all(&s.pool)
        .await?,
        None => sqlx::query_as::<_, Task>("select * from task order by position, created_at")
            .fetch_all(&s.pool)
            .await?,
    };
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<TaskInput>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let status = input.status.clone().unwrap_or_else(|| "todo".to_string());
    check_status(&status)?;
    check_optional(&input.priority, PRIORITY_LEVELS, "priority")?;
    check_optional(&input.size, TASK_SIZES, "size")?;
    check_optional(&input.type_, TASK_TYPES, "type")?;
    let row = sqlx::query_as::<_, Task>(
        "insert into task (project_id, title, status, due_on, priority, size, description, checklist, position, version, type) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) returning *",
    )
    .bind(input.project_id)
    .bind(&input.title)
    .bind(&status)
    .bind(input.due_on)
    .bind(&input.priority)
    .bind(&input.size)
    .bind(&input.description)
    .bind(sqlx::types::Json(&input.checklist))
    .bind(input.position.unwrap_or(0))
    .bind(&input.version)
    .bind(&input.type_)
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id does not exist".into())
        }
        other => AppError::Db(other),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<TaskInput>,
) -> Result<Json<Task>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let status = input.status.clone().unwrap_or_else(|| "todo".to_string());
    check_status(&status)?;
    check_optional(&input.priority, PRIORITY_LEVELS, "priority")?;
    check_optional(&input.size, TASK_SIZES, "size")?;
    check_optional(&input.type_, TASK_TYPES, "type")?;
    let row = sqlx::query_as::<_, Task>(
        "update task set project_id=$2, title=$3, status=$4, due_on=$5, priority=$6, \
         size=$7, description=$8, checklist=$9, position=$10, version=$11, type=$12 where id=$1 returning *",
    )
    .bind(id)
    .bind(input.project_id)
    .bind(&input.title)
    .bind(&status)
    .bind(input.due_on)
    .bind(&input.priority)
    .bind(&input.size)
    .bind(&input.description)
    .bind(sqlx::types::Json(&input.checklist))
    .bind(input.position.unwrap_or(0))
    .bind(&input.version)
    .bind(&input.type_)
    .fetch_optional(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id does not exist".into())
        }
        other => AppError::Db(other),
    })?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn delete(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let res = sqlx::query("delete from task where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
