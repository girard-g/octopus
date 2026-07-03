use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Note, NoteInput};

fn map_fk(e: sqlx::Error) -> AppError {
    match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("contact_id, project_id, or folder_id does not exist".into())
        }
        other => AppError::Db(other),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub contact_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub q: Option<String>,
}

fn validate_input(input: &NoteInput) -> Result<(), AppError> {
    let has_title = input
        .title
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    let has_body = !input.body.trim().is_empty();
    if !has_title && !has_body {
        return Err(AppError::BadRequest("title or body is required".into()));
    }
    if input.contact_id.is_some() && input.project_id.is_some() {
        return Err(AppError::BadRequest(
            "link to a contact or a project, not both".into(),
        ));
    }
    Ok(())
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Note>>, AppError> {
    // priority: search > folder > contact > project > all
    if let Some(term) = q.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let rows = sqlx::query_as::<_, Note>(
            "select * from note where title ilike $1 or body ilike $1 order by updated_at desc",
        )
        .bind(format!("%{term}%"))
        .fetch_all(&s.pool)
        .await?;
        return Ok(Json(rows));
    }
    let rows = match (q.folder_id, q.contact_id, q.project_id) {
        (Some(fid), _, _) => sqlx::query_as::<_, Note>(
            "select * from note where folder_id = $1 order by pinned desc, updated_at desc",
        )
        .bind(fid)
        .fetch_all(&s.pool)
        .await?,
        (None, Some(cid), None) => sqlx::query_as::<_, Note>(
            "select * from note where contact_id = $1 order by created_at desc",
        )
        .bind(cid)
        .fetch_all(&s.pool)
        .await?,
        (None, None, Some(pid)) => sqlx::query_as::<_, Note>(
            "select * from note where project_id = $1 order by created_at desc",
        )
        .bind(pid)
        .fetch_all(&s.pool)
        .await?,
        _ => sqlx::query_as::<_, Note>("select * from note order by updated_at desc")
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
    let title = input.title.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let row = sqlx::query_as::<_, Note>(
        "insert into note (body, title, contact_id, project_id, folder_id, pinned) \
         values ($1,$2,$3,$4,$5,$6) returning *",
    )
    .bind(&input.body)
    .bind(title)
    .bind(input.contact_id)
    .bind(input.project_id)
    .bind(input.folder_id)
    .bind(input.pinned.unwrap_or(false))
    .fetch_one(&s.pool)
    .await
    .map_err(map_fk)?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<NoteInput>,
) -> Result<Json<Note>, AppError> {
    validate_input(&input)?;
    let title = input.title.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let row = sqlx::query_as::<_, Note>(
        "update note set body=$2, title=$3, contact_id=$4, project_id=$5, \
         folder_id=$6, pinned=$7, updated_at=now() where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.body)
    .bind(title)
    .bind(input.contact_id)
    .bind(input.project_id)
    .bind(input.folder_id)
    .bind(input.pinned.unwrap_or(false))
    .fetch_optional(&s.pool)
    .await
    .map_err(map_fk)?
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
