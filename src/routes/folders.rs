use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Folder, FolderInput};

fn map_fk(e: sqlx::Error) -> AppError {
    match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("parent_id does not exist".into())
        }
        other => AppError::Db(other),
    }
}

fn clean_name(name: &str) -> Result<String, AppError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    Ok(n.to_string())
}

pub async fn list(_: AuthUser, State(s): State<AppState>) -> Result<Json<Vec<Folder>>, AppError> {
    let rows = sqlx::query_as::<_, Folder>(
        "select * from folder order by parent_id nulls first, position, created_at",
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<FolderInput>,
) -> Result<(StatusCode, Json<Folder>), AppError> {
    let name = clean_name(&input.name)?;
    let row = sqlx::query_as::<_, Folder>(
        "insert into folder (name, parent_id, position) values ($1,$2,$3) returning *",
    )
    .bind(&name)
    .bind(input.parent_id)
    .bind(input.position.unwrap_or(0))
    .fetch_one(&s.pool)
    .await
    .map_err(map_fk)?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn would_cycle(s: &AppState, id: Uuid, new_parent: Uuid) -> Result<bool, AppError> {
    let rows: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as("select id, parent_id from folder")
        .fetch_all(&s.pool)
        .await?;
    let parent_of: std::collections::HashMap<Uuid, Option<Uuid>> = rows.into_iter().collect();

    let mut current = Some(new_parent);
    let mut visited = std::collections::HashSet::new();
    while let Some(cur) = current {
        if cur == id {
            return Ok(true);
        }
        if !visited.insert(cur) {
            break; // pre-existing cycle unrelated to this move; stop walking
        }
        current = parent_of.get(&cur).copied().flatten();
    }
    Ok(false)
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<FolderInput>,
) -> Result<Json<Folder>, AppError> {
    let name = clean_name(&input.name)?;
    if let Some(new_parent) = input.parent_id {
        if new_parent == id || would_cycle(&s, id, new_parent).await? {
            return Err(AppError::BadRequest(
                "cannot move a folder under its own descendant".into(),
            ));
        }
    }
    let row = sqlx::query_as::<_, Folder>(
        "update folder set name=$2, parent_id=$3, position=coalesce($4, position) \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&name)
    .bind(input.parent_id)
    .bind(input.position)
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
    let res = sqlx::query("delete from folder where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
