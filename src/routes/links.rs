use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::collections::HashSet;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Link, LinkInput};

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub tag: Option<String>,
}

/// Host portion of an already-validated http(s) URL, for the default title.
fn host_of(url: &str) -> &str {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or(rest)
}

/// Validate + normalize input into the columns to store.
/// Returns (url, title, description, category, tags).
fn normalize(input: &LinkInput) -> Result<(String, String, Option<String>, Option<String>, Vec<String>), AppError> {
    let url = input.url.trim().to_string();
    if url.is_empty() {
        return Err(AppError::BadRequest("url is required".into()));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::BadRequest("url must start with http:// or https://".into()));
    }
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| host_of(&url).to_string());
    let clean = |o: &Option<String>| {
        o.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
    };
    let description = clean(&input.description);
    let category = clean(&input.category);
    let mut seen = HashSet::new();
    let tags: Vec<String> = input
        .tags
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect();
    Ok((url, title, description, category, tags))
}

pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Link>>, AppError> {
    let rows = sqlx::query_as::<_, Link>(
        "select * from link \
         where ($1::text is null or category = $1) \
           and ($2::text is null or tags @> array[$2]) \
         order by category nulls last, created_at desc",
    )
    .bind(q.category.as_deref())
    .bind(q.tag.as_deref())
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<LinkInput>,
) -> Result<(StatusCode, Json<Link>), AppError> {
    let (url, title, description, category, tags) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "insert into link (url, title, description, category, tags) \
         values ($1,$2,$3,$4,$5) returning *",
    )
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<LinkInput>,
) -> Result<Json<Link>, AppError> {
    let (url, title, description, category, tags) = normalize(&input)?;
    let row = sqlx::query_as::<_, Link>(
        "update link set url=$2, title=$3, description=$4, category=$5, tags=$6 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&category)
    .bind(&tags)
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

pub async fn delete(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let res = sqlx::query("delete from link where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
