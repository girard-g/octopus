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
    pub contact_id: Option<Uuid>,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
    #[sqlx(default)]
    pub task_count: i64,
    #[sqlx(default)]
    pub done_count: i64,
    #[sqlx(default)]
    pub open_count: i64,
    #[sqlx(default)]
    pub overdue_count: i64,
    #[sqlx(default)]
    pub next_due: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    pub contact_id: Option<Uuid>,
    pub title: String,
    pub status: Option<String>,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub status: String,
    pub due_on: Option<NaiveDate>,
    pub priority: Option<String>,
    pub size: Option<String>,
    pub description: Option<String>,
    #[sqlx(json)]
    pub checklist: Vec<ChecklistItem>,
    pub position: i32,
    pub version: Option<String>,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[sqlx(default)]
    pub project_title: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TaskInput {
    pub project_id: Option<Uuid>,
    pub title: String,
    pub status: Option<String>,
    pub due_on: Option<NaiveDate>,
    pub priority: Option<String>,
    pub size: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub checklist: Vec<ChecklistItem>,
    pub position: Option<i32>,
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

pub const CONTACT_KINDS: [&str; 2] = ["person", "company"];
pub const PROJECT_STATUSES: [&str; 2] = ["active", "archived"];
pub const TASK_STATUSES: [&str; 3] = ["todo", "doing", "done"];
pub const PRIORITY_LEVELS: [&str; 3] = ["low", "medium", "high"];
pub const TASK_SIZES: [&str; 5] = ["xs", "s", "m", "l", "xl"];
pub const TASK_TYPES: [&str; 5] = ["feature", "bug", "enhancement", "chore", "docs"];

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
