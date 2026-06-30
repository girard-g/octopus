use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{Contact, ContactInput, CONTACT_KINDS};

fn validate(input: &ContactInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    if !CONTACT_KINDS.contains(&input.kind.as_str()) {
        return Err(AppError::BadRequest("kind must be person or company".into()));
    }
    Ok(())
}

pub async fn list(_: AuthUser, State(s): State<AppState>) -> Result<Json<Vec<Contact>>, AppError> {
    let rows = sqlx::query_as::<_, Contact>("select * from contact order by name")
        .fetch_all(&s.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn create(
    _: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<ContactInput>,
) -> Result<(StatusCode, Json<Contact>), AppError> {
    validate(&input)?;
    let row = sqlx::query_as::<_, Contact>(
        "insert into contact (kind, name, email, phone, company_id) \
         values ($1,$2,$3,$4,$5) returning *",
    )
    .bind(&input.kind)
    .bind(&input.name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(input.company_id)
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn get(
    _: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Contact>, AppError> {
    let row = sqlx::query_as::<_, Contact>("select * from contact where id = $1")
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
    Json(input): Json<ContactInput>,
) -> Result<Json<Contact>, AppError> {
    validate(&input)?;
    let row = sqlx::query_as::<_, Contact>(
        "update contact set kind=$2, name=$3, email=$4, phone=$5, company_id=$6 \
         where id=$1 returning *",
    )
    .bind(id)
    .bind(&input.kind)
    .bind(&input.name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(input.company_id)
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
    let res = sqlx::query("delete from contact where id = $1")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
