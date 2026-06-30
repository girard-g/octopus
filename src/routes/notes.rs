use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Note, NoteInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub contact_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

fn validate_input(input: &NoteInput) -> Result<(), AppError> {
    if input.body.trim().is_empty() {
        return Err(AppError::BadRequest("body is required".into()));
    }
    let parents = input.contact_id.is_some() as u8 + input.project_id.is_some() as u8;
    if parents != 1 {
        return Err(AppError::BadRequest(
            "exactly one of contact_id/project_id required".into(),
        ));
    }
    Ok(())
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Note>>, AppError> {
    let rows = match (q.contact_id, q.project_id) {
        (Some(cid), None) => sqlx::query_as::<_, Note>(
            "select * from note where contact_id = $1 order by created_at desc",
        )
        .bind(cid)
        .fetch_all(&s.pool)
        .await?,
        (None, Some(pid)) => sqlx::query_as::<_, Note>(
            "select * from note where project_id = $1 order by created_at desc",
        )
        .bind(pid)
        .fetch_all(&s.pool)
        .await?,
        _ => sqlx::query_as::<_, Note>("select * from note order by created_at desc")
            .fetch_all(&s.pool)
            .await?,
    };
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<NoteInput>,
) -> Result<(StatusCode, Json<Note>), AppError> {
    validate_input(&input)?;
    let row = sqlx::query_as::<_, Note>(
        "insert into note (body, contact_id, project_id) values ($1,$2,$3) returning *",
    )
    .bind(&input.body)
    .bind(input.contact_id)
    .bind(input.project_id)
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("contact_id or project_id does not exist".into())
        }
        other => AppError::Db(other),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<NoteInput>,
) -> Result<Json<Note>, AppError> {
    validate_input(&input)?;
    let row = sqlx::query_as::<_, Note>(
        "update note set body=$2, contact_id=$3, project_id=$4 where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.body)
    .bind(input.contact_id)
    .bind(input.project_id)
    .fetch_optional(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("contact_id or project_id does not exist".into())
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
    let res = sqlx::query("delete from note where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
