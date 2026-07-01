use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Event, EventInput, SeriesInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    let rows = match (q.from, q.to) {
        (Some(from), Some(to)) => sqlx::query_as::<_, Event>(
            "select * from event where ends_at >= $1 and starts_at < $2 order by starts_at",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&s.pool)
        .await?,
        _ => sqlx::query_as::<_, Event>("select * from event order by starts_at")
            .fetch_all(&s.pool)
            .await?,
    };
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<EventInput>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if input.ends_at < input.starts_at {
        return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
    }
    let all_day = input.all_day.unwrap_or(false);
    let row = sqlx::query_as::<_, Event>(
        "insert into event (title, starts_at, ends_at, all_day, project_id, contact_id, notes) \
         values ($1,$2,$3,$4,$5,$6,$7) returning *",
    )
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(input.contact_id)
    .bind(&input.notes)
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id or contact_id does not exist".into())
        }
        other => AppError::Db(other),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, AppError> {
    let row = sqlx::query_as::<_, Event>("select * from event where id = $1")
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
    Json(input): Json<EventInput>,
) -> Result<Json<Event>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if input.ends_at < input.starts_at {
        return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
    }
    let all_day = input.all_day.unwrap_or(false);
    let row = sqlx::query_as::<_, Event>(
        "update event set title=$2, starts_at=$3, ends_at=$4, all_day=$5, \
         project_id=$6, contact_id=$7, notes=$8 where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(input.contact_id)
    .bind(&input.notes)
    .fetch_optional(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id or contact_id does not exist".into())
        }
        other => AppError::Db(other),
    })?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

pub async fn delete(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ScopeQuery>,
) -> Result<StatusCode, AppError> {
    let res = match q.scope.as_deref() {
        Some("following") | Some("series") => {
            let target = sqlx::query_as::<_, Event>("select * from event where id = $1")
                .bind(id)
                .fetch_optional(&s.pool)
                .await?
                .ok_or(AppError::NotFound)?;
            let sid = target
                .series_id
                .ok_or_else(|| AppError::BadRequest("event is not part of a series".into()))?;
            if q.scope.as_deref() == Some("following") {
                sqlx::query("delete from event where series_id = $1 and starts_at >= $2")
                    .bind(sid)
                    .bind(target.starts_at)
                    .execute(&s.pool)
                    .await?
            } else {
                sqlx::query("delete from event where series_id = $1")
                    .bind(sid)
                    .execute(&s.pool)
                    .await?
            }
        }
        _ => {
            sqlx::query("delete from event where id = $1")
                .bind(id)
                .execute(&s.pool)
                .await?
        }
    };
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_series(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<SeriesInput>,
) -> Result<(StatusCode, Json<Vec<Event>>), AppError> {
    if input.occurrences.is_empty() {
        return Err(AppError::BadRequest("occurrences must not be empty".into()));
    }
    if input.occurrences.len() > 366 {
        return Err(AppError::BadRequest("too many occurrences (max 366)".into()));
    }
    for occ in &input.occurrences {
        if occ.title.trim().is_empty() {
            return Err(AppError::BadRequest("title is required".into()));
        }
        if occ.ends_at < occ.starts_at {
            return Err(AppError::BadRequest("ends_at must be >= starts_at".into()));
        }
    }

    let series_id = Uuid::new_v4();
    let mut tx = s.pool.begin().await?;
    let mut rows = Vec::with_capacity(input.occurrences.len());
    for occ in &input.occurrences {
        let all_day = occ.all_day.unwrap_or(false);
        let row = sqlx::query_as::<_, Event>(
            "insert into event (title, starts_at, ends_at, all_day, project_id, contact_id, notes, series_id) \
             values ($1,$2,$3,$4,$5,$6,$7,$8) returning *",
        )
        .bind(&occ.title)
        .bind(occ.starts_at)
        .bind(occ.ends_at)
        .bind(all_day)
        .bind(occ.project_id)
        .bind(occ.contact_id)
        .bind(&occ.notes)
        .bind(series_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
                AppError::BadRequest("project_id or contact_id does not exist".into())
            }
            other => AppError::Db(other),
        })?;
        rows.push(row);
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(rows)))
}
