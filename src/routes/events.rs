use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Event, EventInput, SeriesInput, SeriesUpdateInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

// A FK violation on project_id (event insert) or contact_id (junction insert) → 400.
fn fk_400(e: sqlx::Error) -> AppError {
    match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id or contact_id does not exist".into())
        }
        other => AppError::Db(other),
    }
}

// Replace an event's contact links with `contact_ids` (delete-then-insert).
async fn set_event_contacts(
    conn: &mut PgConnection,
    event_id: Uuid,
    contact_ids: &[Uuid],
) -> Result<(), AppError> {
    sqlx::query("delete from event_contact where event_id = $1")
        .bind(event_id)
        .execute(&mut *conn)
        .await?;
    for cid in contact_ids {
        sqlx::query("insert into event_contact (event_id, contact_id) values ($1, $2)")
            .bind(event_id)
            .bind(cid)
            .execute(&mut *conn)
            .await
            .map_err(fk_400)?;
    }
    Ok(())
}

// Read query fragment: aggregate each event's contact_ids into a uuid[].
const AGG: &str = "select e.*, coalesce(array_agg(ec.contact_id) filter (where ec.contact_id is not null), '{}') as contact_ids \
     from event e left join event_contact ec on ec.event_id = e.id";

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    let rows = match (q.from, q.to) {
        (Some(from), Some(to)) => sqlx::query_as::<_, Event>(&format!(
            "{AGG} where e.ends_at >= $1 and e.starts_at < $2 group by e.id order by e.starts_at"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&s.pool)
        .await?,
        _ => sqlx::query_as::<_, Event>(&format!("{AGG} group by e.id order by e.starts_at"))
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
    let mut tx = s.pool.begin().await?;
    let mut row = sqlx::query_as::<_, Event>(
        "insert into event (title, starts_at, ends_at, all_day, project_id, notes) \
         values ($1,$2,$3,$4,$5,$6) returning *",
    )
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(&input.notes)
    .fetch_one(&mut *tx)
    .await
    .map_err(fk_400)?;
    set_event_contacts(&mut *tx, row.id, &input.contact_ids).await?;
    tx.commit().await?;
    row.contact_ids = input.contact_ids;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, AppError> {
    let row = sqlx::query_as::<_, Event>(&format!("{AGG} where e.id = $1 group by e.id"))
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
    let mut tx = s.pool.begin().await?;
    let row = sqlx::query_as::<_, Event>(
        "update event set title=$2, starts_at=$3, ends_at=$4, all_day=$5, project_id=$6, notes=$7 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.title)
    .bind(input.starts_at)
    .bind(input.ends_at)
    .bind(all_day)
    .bind(input.project_id)
    .bind(&input.notes)
    .fetch_optional(&mut *tx)
    .await
    .map_err(fk_400)?;
    let mut row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };
    set_event_contacts(&mut *tx, id, &input.contact_ids).await?;
    tx.commit().await?;
    row.contact_ids = input.contact_ids;
    Ok(Json(row))
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
        _ => sqlx::query("delete from event where id = $1")
            .bind(id)
            .execute(&s.pool)
            .await?,
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
        let mut row = sqlx::query_as::<_, Event>(
            "insert into event (title, starts_at, ends_at, all_day, project_id, notes, series_id) \
             values ($1,$2,$3,$4,$5,$6,$7) returning *",
        )
        .bind(&occ.title)
        .bind(occ.starts_at)
        .bind(occ.ends_at)
        .bind(all_day)
        .bind(occ.project_id)
        .bind(&occ.notes)
        .bind(series_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(fk_400)?;
        set_event_contacts(&mut *tx, row.id, &occ.contact_ids).await?;
        row.contact_ids = occ.contact_ids.clone();
        rows.push(row);
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(rows)))
}

pub async fn update_series(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ScopeQuery>,
    Json(input): Json<SeriesUpdateInput>,
) -> Result<Json<Vec<Event>>, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    let mut tx = s.pool.begin().await?;
    let target = sqlx::query_as::<_, Event>("select * from event where id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    let sid = target
        .series_id
        .ok_or_else(|| AppError::BadRequest("event is not part of a series".into()))?;
    let all_day = input.all_day.unwrap_or(false);

    let mut rows = match q.scope.as_deref() {
        Some("series") => sqlx::query_as::<_, Event>(
            "update event set title=$2, notes=$3, project_id=$4, all_day=$5, \
             starts_at = starts_at + ($6::bigint * interval '1 second'), \
             ends_at   = ends_at   + ($6::bigint * interval '1 second') \
             where series_id=$1 returning *",
        )
        .bind(sid)
        .bind(&input.title)
        .bind(&input.notes)
        .bind(input.project_id)
        .bind(all_day)
        .bind(input.shift_seconds)
        .fetch_all(&mut *tx)
        .await
        .map_err(fk_400)?,

        Some("following") => {
            let new_sid = Uuid::new_v4();
            sqlx::query_as::<_, Event>(
                "update event set title=$2, notes=$3, project_id=$4, all_day=$5, \
                 starts_at = starts_at + ($6::bigint * interval '1 second'), \
                 ends_at   = ends_at   + ($6::bigint * interval '1 second'), \
                 series_id = $7 \
                 where series_id=$1 and starts_at >= $8 returning *",
            )
            .bind(sid)
            .bind(&input.title)
            .bind(&input.notes)
            .bind(input.project_id)
            .bind(all_day)
            .bind(input.shift_seconds)
            .bind(new_sid)
            .bind(target.starts_at)
            .fetch_all(&mut *tx)
            .await
            .map_err(fk_400)?
        }

        _ => return Err(AppError::BadRequest("scope must be 'following' or 'series'".into())),
    };

    for row in rows.iter_mut() {
        set_event_contacts(&mut *tx, row.id, &input.contact_ids).await?;
        row.contact_ids = input.contact_ids.clone();
    }
    tx.commit().await?;
    Ok(Json(rows))
}
