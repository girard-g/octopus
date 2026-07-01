use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Contact {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ContactInput {
    pub kind: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub contact_id: Uuid,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
    pub board_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    pub contact_id: Uuid,
    pub title: String,
    pub status: Option<String>,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectMove {
    pub status: String,
    pub board_order: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub status: String,
    pub due_on: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TaskInput {
    pub project_id: Option<Uuid>,
    pub title: String,
    pub status: Option<String>,
    pub due_on: Option<NaiveDate>,
}

pub const CONTACT_KINDS: [&str; 2] = ["person", "company"];
pub const PROJECT_STATUSES: [&str; 5] = ["lead", "proposal", "active", "done", "lost"];
pub const TASK_STATUSES: [&str; 3] = ["todo", "doing", "done"];

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: bool,
    pub project_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub series_id: Option<Uuid>,
    #[sqlx(default)]
    pub contact_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EventInput {
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub all_day: Option<bool>,
    pub project_id: Option<Uuid>,
    pub notes: Option<String>,
    #[serde(default)]
    pub contact_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SeriesInput {
    pub occurrences: Vec<EventInput>,
}

#[derive(Debug, Deserialize)]
pub struct SeriesUpdateInput {
    pub title: String,
    pub notes: Option<String>,
    pub project_id: Option<Uuid>,
    pub all_day: Option<bool>,
    pub shift_seconds: i64,
    #[serde(default)]
    pub contact_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Note {
    pub id: Uuid,
    pub body: String,
    pub contact_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NoteInput {
    pub body: String,
    pub contact_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}
