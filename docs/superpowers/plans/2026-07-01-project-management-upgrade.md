# Project Management Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the per-project kanban into a proper PM tool — richer tasks (priority, size, description, checklist, manual order), per-project health stats, and a cross-project overdue/upcoming task view on the Dashboard.

**Architecture:** Add task fields via one migration; extend the existing full-replace task API to persist them; enrich the projects-list and dashboard read queries with `#[sqlx(default)]`-populated aggregate fields (same pattern as the existing `task_count`). Frontend adds a task-detail modal and fixes every minimal-payload PUT caller to send the full task object so drags/toggles don't wipe fields.

**Tech Stack:** Rust + Axum 0.8 + sqlx 0.8 (Postgres), Svelte 5 (runes) + svelte-spa-router + svelte-dnd-action, Vitest.

## Global Constraints

- `time` crate stays pinned `=0.3.41` in Cargo.toml — do not bump; no new dependencies.
- `PUT /api/tasks/:id` is a **full replace** — every caller must send the complete task object or unsent fields are overwritten with null/default.
- Auth: all API handlers take `_: AuthUser` as the first extractor (see existing routes).
- Migrations are append-only; next number is `0008`.
- New enum-like columns use a `text` + `check (... in (...))` constraint, mirroring the existing `status` columns, plus a matching const array in `models.rs`.
- Backend done-check: `cargo check` then `cargo test` must be clean before a task is complete. Frontend: `npm test` (in `frontend/`) clean.

---

### Task 1: Migration + models for new task fields and project health

**Files:**
- Create: `migrations/0008_task_fields.sql`
- Modify: `src/models.rs`

**Interfaces:**
- Produces: `ChecklistItem { title: String, done: bool }`; `Task` fields `priority: Option<String>`, `size: Option<String>`, `description: Option<String>`, `checklist: Vec<ChecklistItem>` (via `#[sqlx(json)]`), `position: i32`, `project_title: Option<String>` (`#[sqlx(default)]`); `TaskInput` fields `priority/size/description` (`Option`), `checklist: Vec<ChecklistItem>` (`#[serde(default)]`), `position: Option<i32>`; `Project` fields `done_count: i64`, `open_count: i64`, `overdue_count: i64`, `next_due: Option<NaiveDate>` (all `#[sqlx(default)]`); consts `PRIORITY_LEVELS: [&str; 3]`, `TASK_SIZES: [&str; 5]`.

- [ ] **Step 1: Write the migration**

Create `migrations/0008_task_fields.sql`:

```sql
alter table task add column priority    text check (priority in ('low','medium','high'));
alter table task add column size        text check (size in ('xs','s','m','l','xl'));
alter table task add column description text;
alter table task add column checklist   jsonb not null default '[]';
alter table task add column position    integer not null default 0;
```

- [ ] **Step 2: Add the model changes**

In `src/models.rs`, add above the `Task` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub title: String,
    pub done: bool,
}
```

Replace the `Task` struct with:

```rust
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
    #[sqlx(default)]
    pub project_title: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

Replace the `TaskInput` struct with:

```rust
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
}
```

Add after the existing `TASK_STATUSES` const:

```rust
pub const PRIORITY_LEVELS: [&str; 3] = ["low", "medium", "high"];
pub const TASK_SIZES: [&str; 5] = ["xs", "s", "m", "l", "xl"];
```

Replace the `Project` struct's `task_count` region — add the health fields alongside it:

```rust
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
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: clean (no errors).

- [ ] **Step 4: Commit**

```bash
git add migrations/0008_task_fields.sql src/models.rs
git commit -m "feat(tasks): schema + models for priority/size/description/checklist/position + project health fields"
```

---

### Task 2: Persist new task fields through create/update (+ drag-wipe regression test)

**Files:**
- Modify: `src/routes/tasks.rs`
- Test: `tests/tasks.rs`

**Interfaces:**
- Consumes: `Task`, `TaskInput`, `PRIORITY_LEVELS`, `TASK_SIZES`, `TASK_STATUSES` from Task 1.
- Produces: `POST /api/tasks` and `PUT /api/tasks/:id` accept and round-trip `priority`, `size`, `description`, `checklist`, `position`; invalid `priority`/`size` return 400.

- [ ] **Step 1: Write the failing tests**

Append to `tests/tasks.rs`:

```rust
#[sqlx::test]
async fn task_roundtrips_new_fields(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "Ship it",
            "priority": "high",
            "size": "m",
            "description": "the big one",
            "checklist": [{"title":"a","done":false},{"title":"b","done":true}],
            "position": 2
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["priority"], "high");
    assert_eq!(t["size"], "m");
    assert_eq!(t["description"], "the big one");
    assert_eq!(t["position"], 2);
    assert_eq!(t["checklist"].as_array().unwrap().len(), 2);
    assert_eq!(t["checklist"][1]["done"], true);
}

// Guards the drag-wipe trap: a full-object PUT that changes status must NOT
// drop priority/checklist. (The board's drag handler sends the full object.)
#[sqlx::test]
async fn task_move_preserves_fields(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "X", "priority": "low",
            "checklist": [{"title":"step","done":false}]
        })).with_cookie(&cookie),
    )
    .await;
    let id = t["id"].as_str().unwrap().to_string();

    // Full-object PUT with status flipped to doing (as the drag handler sends it).
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({
            "title": "X", "status": "doing", "priority": "low",
            "checklist": [{"title":"step","done":false}], "position": 0
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["status"], "doing");
    assert_eq!(upd["priority"], "low");
    assert_eq!(upd["checklist"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn task_rejects_bad_priority(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"X","priority":"urgent"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test tasks task_roundtrips_new_fields task_move_preserves_fields task_rejects_bad_priority`
Expected: FAIL — fields not persisted / bad priority not rejected.

- [ ] **Step 3: Implement the route changes**

In `src/routes/tasks.rs`, update the import line:

```rust
use crate::models::{Task, TaskInput, PRIORITY_LEVELS, TASK_SIZES, TASK_STATUSES};
```

Add validation helpers next to `check_status`:

```rust
fn check_optional<const N: usize>(v: &Option<String>, allowed: [&str; N], what: &str) -> Result<(), AppError> {
    match v {
        Some(s) if !allowed.contains(&s.as_str()) => {
            Err(AppError::BadRequest(format!("invalid task {what}")))
        }
        _ => Ok(()),
    }
}
```

Replace the `create` handler's body (from `let status = ...` through the `Ok(...)`) with:

```rust
    let status = input.status.clone().unwrap_or_else(|| "todo".to_string());
    check_status(&status)?;
    check_optional(&input.priority, PRIORITY_LEVELS, "priority")?;
    check_optional(&input.size, TASK_SIZES, "size")?;
    let row = sqlx::query_as::<_, Task>(
        "insert into task (project_id, title, status, due_on, priority, size, description, checklist, position) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9) returning *",
    )
    .bind(input.project_id)
    .bind(&input.title)
    .bind(&status)
    .bind(input.due_on)
    .bind(&input.priority)
    .bind(&input.size)
    .bind(&input.description)
    .bind(sqlx::types::Json(&input.checklist))
    .bind(input.position.unwrap_or(0))
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id does not exist".into())
        }
        other => AppError::Db(other),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
```

Replace the `update` handler's body (from `let status = ...` through the `Ok(...)`) with:

```rust
    let status = input.status.clone().unwrap_or_else(|| "todo".to_string());
    check_status(&status)?;
    check_optional(&input.priority, PRIORITY_LEVELS, "priority")?;
    check_optional(&input.size, TASK_SIZES, "size")?;
    let row = sqlx::query_as::<_, Task>(
        "update task set project_id=$2, title=$3, status=$4, due_on=$5, priority=$6, \
         size=$7, description=$8, checklist=$9, position=$10 where id=$1 returning *",
    )
    .bind(id)
    .bind(input.project_id)
    .bind(&input.title)
    .bind(&status)
    .bind(input.due_on)
    .bind(&input.priority)
    .bind(&input.size)
    .bind(&input.description)
    .bind(sqlx::types::Json(&input.checklist))
    .bind(input.position.unwrap_or(0))
    .fetch_optional(&s.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.is_foreign_key_violation() => {
            AppError::BadRequest("project_id does not exist".into())
        }
        other => AppError::Db(other),
    })?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
```

Change the `list` query ordering (both branches) from `order by due_on nulls last, created_at` to:

```rust
"select * from task where project_id = $1 order by position, created_at"
```
and (the no-filter branch):
```rust
"select * from task order by position, created_at"
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test tasks`
Expected: PASS (all task tests, old and new).

- [ ] **Step 5: Commit**

```bash
git add src/routes/tasks.rs tests/tasks.rs
git commit -m "feat(tasks): persist priority/size/description/checklist/position; guard drag-wipe with test"
```

---

### Task 3: Dashboard due-tasks carry project_title

**Files:**
- Modify: `src/routes/dashboard.rs`
- Test: `tests/dashboard.rs`

**Interfaces:**
- Consumes: `Task.project_title` from Task 1.
- Produces: `GET /api/dashboard` `due_tasks[].project_title` is the parent project's title (null for standalone tasks).

- [ ] **Step 1: Write the failing test**

Append to `tests/dashboard.rs` (follow the file's existing helper imports — it already uses `test_app`, `login`, `send`, `json_req`, `with_cookie`):

```rust
#[sqlx::test]
async fn dashboard_due_tasks_have_project_title(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"title":"Acme"})).with_cookie(&cookie)).await;
    let pid = p["id"].as_str().unwrap();
    send(&app, json_req("POST", "/api/tasks", json!({"title":"T","project_id":pid})).with_cookie(&cookie)).await;

    let (_, dash) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    let due = &dash["due_tasks"];
    assert_eq!(due[0]["project_title"], "Acme");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test dashboard dashboard_due_tasks_have_project_title`
Expected: FAIL — `project_title` is null.

- [ ] **Step 3: Update the due_tasks query**

In `src/routes/dashboard.rs`, replace the `due_tasks` query with:

```rust
    let due_tasks = sqlx::query_as::<_, Task>(
        "select t.*, p.title as project_title from task t \
         left join project p on p.id = t.project_id \
         where t.status <> 'done' order by t.due_on nulls last, t.created_at limit 20",
    )
    .fetch_all(&s.pool)
    .await?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test dashboard`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/dashboard.rs tests/dashboard.rs
git commit -m "feat(dashboard): due_tasks carry project_title"
```

---

### Task 4: Projects list returns health stats

**Files:**
- Modify: `src/routes/projects.rs`
- Test: `tests/projects.rs`

**Interfaces:**
- Consumes: `Project.done_count/open_count/overdue_count/next_due` from Task 1.
- Produces: `GET /api/projects` each project has `done_count`, `open_count`, `overdue_count`, `next_due` computed from its tasks.

- [ ] **Step 1: Write the failing test**

Append to `tests/projects.rs` (uses the same helpers as the other test files):

```rust
#[sqlx::test]
async fn project_list_reports_health(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"title":"P"})).with_cookie(&cookie)).await;
    let pid = p["id"].as_str().unwrap();
    send(&app, json_req("POST", "/api/tasks", json!({"title":"done","project_id":pid,"status":"done"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/tasks", json!({"title":"late","project_id":pid,"due_on":"2020-01-01"})).with_cookie(&cookie)).await;

    let (_, list) = send(&app, json_req("GET", "/api/projects", json!(null)).with_cookie(&cookie)).await;
    let row = &list[0];
    assert_eq!(row["task_count"], 2);
    assert_eq!(row["done_count"], 1);
    assert_eq!(row["open_count"], 1);
    assert_eq!(row["overdue_count"], 1);
    assert_eq!(row["next_due"], "2020-01-01");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test projects project_list_reports_health`
Expected: FAIL — health fields are 0/null.

- [ ] **Step 3: Update both list queries**

In `src/routes/projects.rs`, in the `list` handler, replace the `select` in **both** the `Some(st)` and `None` branches. The aggregate columns are identical; only the `where p.status = $1` clause differs.

`Some(st)` branch:

```rust
            sqlx::query_as::<_, Project>(
                "select p.*, \
                   count(t.id) as task_count, \
                   count(t.id) filter (where t.status = 'done') as done_count, \
                   count(t.id) filter (where t.status <> 'done') as open_count, \
                   count(t.id) filter (where t.status <> 'done' and t.due_on < current_date) as overdue_count, \
                   min(t.due_on) filter (where t.status <> 'done') as next_due \
                 from project p left join task t on t.project_id = p.id \
                 where p.status = $1 group by p.id order by p.created_at",
            )
            .bind(st)
            .fetch_all(&s.pool)
            .await?
```

`None` branch (same select, no `where`):

```rust
            sqlx::query_as::<_, Project>(
                "select p.*, \
                   count(t.id) as task_count, \
                   count(t.id) filter (where t.status = 'done') as done_count, \
                   count(t.id) filter (where t.status <> 'done') as open_count, \
                   count(t.id) filter (where t.status <> 'done' and t.due_on < current_date) as overdue_count, \
                   min(t.due_on) filter (where t.status <> 'done') as next_due \
                 from project p left join task t on t.project_id = p.id \
                 group by p.id order by p.created_at",
            )
            .fetch_all(&s.pool)
            .await?
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test projects`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/routes/projects.rs tests/projects.rs
git commit -m "feat(projects): list returns per-project health stats"
```

---

### Task 5: Frontend task move helper computes position + full-object PUT callers

**Files:**
- Modify: `frontend/src/lib/tasks.js`
- Modify: `frontend/src/routes/ProjectBoard.svelte:51-71` (drag `finalize`)
- Modify: `frontend/src/routes/Dashboard.svelte:56-61` (`toggleDone`)
- Test: `frontend/src/lib/tasks.test.js`

**Interfaces:**
- Consumes: task objects with `status` and `position`.
- Produces: `movesForTaskColumn(status, items)` returns `[{ id, status, position }]` for every item whose `status` or `position` changed; both PUT callers send the **full** task object with `status`/`position` overridden.

- [ ] **Step 1: Write the failing test**

Replace the two `movesForTaskColumn` tests in `frontend/src/lib/tasks.test.js` with:

```javascript
  it('emits moves with new position for reordered or moved items', () => {
    const moves = movesForTaskColumn('doing', [
      { id: 'x', status: 'todo', position: 0 },   // status changed
      { id: 'y', status: 'doing', position: 5 },   // position changed (now index 1)
    ])
    expect(moves).toEqual([
      { id: 'x', status: 'doing', position: 0 },
      { id: 'y', status: 'doing', position: 1 },
    ])
  })

  it('emits no moves when status and position are already correct', () => {
    const moves = movesForTaskColumn('done', [
      { id: 'a', status: 'done', position: 0 },
      { id: 'b', status: 'done', position: 1 },
    ])
    expect(moves).toEqual([])
  })
```

- [ ] **Step 2: Run test to verify it fails**

Run (in `frontend/`): `npm test -- tasks.test.js`
Expected: FAIL — helper returns `{id, status}` without `position`.

- [ ] **Step 3: Update the helper**

In `frontend/src/lib/tasks.js`, replace `movesForTaskColumn` with:

```javascript
// After a dnd drop, compute deltas for a column: any item whose status or
// position (its index in the column) changed. Callers send the FULL task
// object with these fields overridden.
export function movesForTaskColumn(status, items) {
  return items
    .map((t, i) => ({ t, i }))
    .filter(({ t, i }) => t.status !== status || t.position !== i)
    .map(({ t, i }) => ({ id: t.id, status, position: i }))
}
```

- [ ] **Step 4: Fix the ProjectBoard drag handler to send full objects**

In `frontend/src/routes/ProjectBoard.svelte`, replace the `finalize` body of `dndHandlers` (the `for (const m of moves)` loop) with:

```javascript
      finalize: async (e) => {
        cols[status] = e.detail.items; cols = cols
        const moves = movesForTaskColumn(status, e.detail.items)
        try {
          for (const m of moves) {
            const t = e.detail.items.find((x) => x.id === m.id)
            await api.put('/api/tasks/' + m.id, { ...t, status: m.status, position: m.position, project_id: id })
          }
        } catch (err) { error = err.message }
        await load()
      },
```

- [ ] **Step 5: Fix the Dashboard toggleDone to send the full object**

In `frontend/src/routes/Dashboard.svelte`, replace `toggleDone` with:

```javascript
  async function toggleDone(t) {
    try {
      await api.put(`/api/tasks/${t.id}`, { ...t, status: 'done' })
      await load()
    } catch (e) { error = e.message }
  }
```

- [ ] **Step 6: Run tests to verify they pass**

Run (in `frontend/`): `npm test -- tasks.test.js`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/tasks.js frontend/src/lib/tasks.test.js frontend/src/routes/ProjectBoard.svelte frontend/src/routes/Dashboard.svelte
git commit -m "feat(web): task moves persist position; full-object PUTs prevent field wipe"
```

---

### Task 6: Task detail modal + richer card on the board

**Files:**
- Modify: `frontend/src/routes/ProjectBoard.svelte`

**Interfaces:**
- Consumes: task objects with `priority/size/description/checklist`; `api.put`.
- Produces: clicking a card opens an edit modal; card shows priority dot, size chip, checklist progress.

- [ ] **Step 1: Add modal state and helpers**

In `frontend/src/routes/ProjectBoard.svelte` `<script>`, add state near the other `$state` declarations:

```javascript
  let editingTask = $state(null)   // a shallow copy of the task being edited
  let newChecklistItem = $state('')
```

Add these helpers (after `deleteTask`):

```javascript
  const PRIORITY_DOT = { low: 'bg-st-done', medium: 'bg-st-proposal', high: 'bg-st-lost' }

  function openTask(t) {
    editingTask = { ...t, checklist: (t.checklist ?? []).map((c) => ({ ...c })) }
  }
  function addChecklistItem() {
    const title = newChecklistItem.trim()
    if (!title) return
    editingTask.checklist = [...editingTask.checklist, { title, done: false }]
    newChecklistItem = ''
  }
  function removeChecklistItem(i) {
    editingTask.checklist = editingTask.checklist.filter((_, idx) => idx !== i)
  }
  async function saveTask(e) {
    e.preventDefault()
    const t = editingTask
    try {
      await api.put('/api/tasks/' + t.id, {
        title: t.title,
        status: t.status,
        project_id: id,
        due_on: t.due_on || null,
        priority: t.priority || null,
        size: t.size || null,
        description: t.description || null,
        checklist: t.checklist,
        position: t.position ?? 0,
      })
      editingTask = null
      await load()
    } catch (err) { error = err.message }
  }
```

- [ ] **Step 2: Make the card open the modal + show flags**

In the board card markup, replace the task title/due block. The card `<div>` stays a dndzone item; wrap the text region in a click handler that opens the modal (leave the `×` delete button as-is, and stop its propagation).

Replace the inner content (the `<div class="min-w-0 flex-1">...</div>` and the delete button) with:

```svelte
              <button
                type="button"
                onclick={() => openTask(t)}
                class="min-w-0 flex-1 text-left"
              >
                <div class="flex items-center gap-1.5">
                  {#if t.priority}<span class="h-1.5 w-1.5 shrink-0 rounded-full {PRIORITY_DOT[t.priority]}"></span>{/if}
                  <span class="truncate font-mono text-[13px] font-medium leading-snug text-ink">{t.title}</span>
                </div>
                <div class="mt-0.5 flex flex-wrap items-center gap-2 font-mono text-[11px] text-faint">
                  {#if t.due_on}<span>{t.due_on}</span>{/if}
                  {#if t.size}<span class="uppercase">[{t.size}]</span>{/if}
                  {#if t.checklist?.length}<span>{t.checklist.filter((c) => c.done).length}/{t.checklist.length}</span>{/if}
                </div>
              </button>
              <button
                onclick={(e) => { e.stopPropagation(); deleteTask(t.id) }}
                aria-label="Delete task"
                class="ml-2 shrink-0 font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
              >×</button>
```

- [ ] **Step 3: Add the task modal markup**

At the end of the file (after the edit-project modal `{/if}`), add:

```svelte
{#if editingTask}
  <Modal title="Edit task" onclose={() => (editingTask = null)}>
    <form onsubmit={saveTask} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={editingTask.title} required class={FIELD} />
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <p class="label mb-1.5">Priority</p>
          <select bind:value={editingTask.priority} class={FIELD}>
            <option value={null}>— none —</option>
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
          </select>
        </div>
        <div>
          <p class="label mb-1.5">Size</p>
          <select bind:value={editingTask.size} class={FIELD}>
            <option value={null}>— none —</option>
            <option value="xs">XS</option>
            <option value="s">S</option>
            <option value="m">M</option>
            <option value="l">L</option>
            <option value="xl">XL</option>
          </select>
        </div>
      </div>
      <div>
        <p class="label mb-1.5">Due date</p>
        <input type="date" bind:value={editingTask.due_on} class={FIELD} />
      </div>
      <div>
        <p class="label mb-1.5">Description</p>
        <textarea bind:value={editingTask.description} rows="3" class="{FIELD} resize-none"></textarea>
      </div>
      <div>
        <p class="label mb-1.5">Checklist</p>
        <div class="flex flex-col gap-1.5">
          {#each editingTask.checklist as item, i (i)}
            <div class="flex items-center gap-2">
              <input type="checkbox" bind:checked={item.done} class="h-3.5 w-3.5 shrink-0 rounded-sm accent-accent" />
              <span class="min-w-0 flex-1 truncate font-mono text-[13px] text-ink" class:line-through={item.done} class:text-faint={item.done}>{item.title}</span>
              <button type="button" onclick={() => removeChecklistItem(i)} aria-label="Remove item" class="shrink-0 font-mono text-[15px] leading-none text-faint transition hover:text-st-lost">×</button>
            </div>
          {/each}
          <div class="flex gap-2">
            <input bind:value={newChecklistItem} onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addChecklistItem() } }} placeholder="add item…" class="{FIELD} flex-1" />
            <button type="button" onclick={addChecklistItem} class="h-[38px] shrink-0 rounded-sm border border-border-2 px-3 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink">+</button>
          </div>
        </div>
      </div>
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
    </form>
  </Modal>
{/if}
```

- [ ] **Step 4: Verify build and manual click-vs-drag**

Run (in `frontend/`): `npm run build`
Expected: build succeeds.
Then manually (or note for the reviewer): dragging a card still reorders; a plain click opens the modal; the `×` deletes without opening the modal.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/ProjectBoard.svelte
git commit -m "feat(web): task detail modal (priority/size/desc/checklist) + richer board card"
```

---

### Task 7: Per-project health on the Projects list

**Files:**
- Modify: `frontend/src/routes/Projects.svelte:65-77` (project card)

**Interfaces:**
- Consumes: `p.task_count/done_count/open_count/overdue_count/next_due` from Task 4.

- [ ] **Step 1: Enrich the project card**

In `frontend/src/routes/Projects.svelte`, replace the task-count line (the `<div class="mt-3 ...">` showing `{p.task_count} task…`) with a health row:

```svelte
      <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-[11px] tabular-nums text-faint">
        <span><span class="text-accent-dim">&gt;</span> {p.done_count}/{p.task_count} done</span>
        {#if p.overdue_count > 0}<span class="text-st-lost">{p.overdue_count} overdue</span>{/if}
        {#if p.next_due}<span>next {p.next_due}</span>{/if}
      </div>
```

- [ ] **Step 2: Verify build**

Run (in `frontend/`): `npm run build`
Expected: build succeeds.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/Projects.svelte
git commit -m "feat(web): per-project health on Projects list"
```

---

### Task 8: Dashboard overdue/upcoming split with project tags

**Files:**
- Modify: `frontend/src/routes/Dashboard.svelte:123-139` (tasks_due list)

**Interfaces:**
- Consumes: `dueTasks[].due_on`, `dueTasks[].project_title` from Task 3.

- [ ] **Step 1: Add derived overdue/upcoming split**

In `frontend/src/routes/Dashboard.svelte` `<script>`, add after the `tiles` derived:

```javascript
  const today = new Date().toISOString().slice(0, 10)   // YYYY-MM-DD, matches due_on
  const overdue = $derived(dueTasks.filter((t) => t.due_on && t.due_on < today))
  const upcoming = $derived(dueTasks.filter((t) => !t.due_on || t.due_on >= today))
```

- [ ] **Step 2: Render overdue first (red), then upcoming**

Replace the `tasks_due` `<ul>` body (the `{#each dueTasks as t}...{/each}` block) with a helper snippet reused for both groups. Replace the whole `<ul class="px-4 py-2">...</ul>` with:

```svelte
    <ul class="px-4 py-2">
      {#snippet taskRow(t, isOverdue)}
        <li class="flex items-center gap-2.5 border-b border-border py-2 last:border-0">
          <input
            type="checkbox"
            checked={t.status === 'done'}
            onchange={() => toggleDone(t)}
            aria-label="Mark {t.title} done"
            class="h-3.5 w-3.5 shrink-0 rounded-sm accent-accent"
          />
          <span class="truncate font-mono text-[13px]" class:text-ink={!isOverdue} class:text-st-lost={isOverdue}>{t.title}</span>
          {#if t.project_title}<span class="shrink-0 font-mono text-[11px] text-faint">/{t.project_title}</span>{/if}
          {#if t.due_on}<span class="ml-auto shrink-0 font-mono text-[11px] tabular-nums" class:text-st-lost={isOverdue} class:text-faint={!isOverdue}>{t.due_on}</span>{/if}
        </li>
      {/snippet}
      {#each overdue as t}{@render taskRow(t, true)}{/each}
      {#each upcoming as t}{@render taskRow(t, false)}{/each}
      {#if dueTasks.length === 0}
        <li class="py-6 text-center font-mono text-[13px] text-faint">// nothing due</li>
      {/if}
    </ul>
```

- [ ] **Step 3: Verify build and full frontend test suite**

Run (in `frontend/`): `npm run build && npm test`
Expected: build succeeds; all tests pass.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/Dashboard.svelte
git commit -m "feat(web): dashboard splits overdue/upcoming tasks with project tags"
```

---

### Task 9: Full verification pass

**Files:** none (verification only).

- [ ] **Step 1: Backend suite**

Run: `cargo test`
Expected: all tests pass (54 existing + new task/dashboard/project tests).

- [ ] **Step 2: Frontend suite + build**

Run (in `frontend/`): `npm test && npm run build`
Expected: all tests pass; build succeeds.

- [ ] **Step 3: Manual smoke (note for reviewer)**

- Create a task, open it, set priority/size/due/description, add checklist items, save → card shows dot/size/progress.
- Drag the task to another column → priority/checklist survive (not wiped).
- Projects list shows done/overdue/next-due.
- Dashboard lists overdue (red) above upcoming, each tagged with its project.

- [ ] **Step 4: Commit (if any doc/lint fixups)**

```bash
git commit --allow-empty -m "chore: project management upgrade verification pass"
```
