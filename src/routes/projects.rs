use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Project, ProjectInput, PROJECT_STATUSES};

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

fn check_status(status: &str) -> Result<(), AppError> {
    if PROJECT_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::BadRequest("invalid project status".into()))
    }
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Project>>, AppError> {
    let rows = match q.status {
        Some(st) => {
            check_status(&st)?;
            sqlx::query_as::<_, Project>(
                "select p.*, count(t.id) as task_count from project p \
                 left join task t on t.project_id = p.id \
                 where p.status = $1 group by p.id order by p.created_at",
            )
            .bind(st)
            .fetch_all(&s.pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, Project>(
                "select p.*, count(t.id) as task_count from project p \
                 left join task t on t.project_id = p.id \
                 group by p.id order by p.created_at",
            )
            .fetch_all(&s.pool)
            .await?
        }
    };
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<ProjectInput>,
) -> Result<(StatusCode, Json<Project>), AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let status = input.status.clone().unwrap_or_else(|| "active".to_string());
    check_status(&status)?;
    let row = sqlx::query_as::<_, Project>(
        "insert into project (contact_id, title, status, description, invoice_url) \
         values ($1,$2,$3,$4,$5) returning *",
    )
    .bind(input.contact_id)
    .bind(&input.title)
    .bind(&status)
    .bind(&input.description)
    .bind(&input.invoice_url)
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("contact_id does not exist".into())
        }
        other => AppError::Db(other),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, AppError> {
    let row = sqlx::query_as::<_, Project>("select * from project where id = $1")
        .bind(id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ProjectInput>,
) -> Result<Json<Project>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if let Some(ref st) = input.status {
        check_status(st)?;
    }
    let row = sqlx::query_as::<_, Project>(
        "update project set contact_id=$2, title=$3, description=$4, invoice_url=$5, \
         status=coalesce($6, status) where id=$1 returning *",
    )
    .bind(id)
    .bind(input.contact_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.invoice_url)
    .bind(&input.status)
    .fetch_optional(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("contact_id does not exist".into())
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
    let res = sqlx::query("delete from project where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
