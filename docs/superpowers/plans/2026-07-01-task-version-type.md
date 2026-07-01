# Task version + type Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional free-text `version` and an optional enum `type` (`feature`/`bug`/`enhancement`/`chore`/`docs`) to tasks, shown as badges on the board card and editable in the task detail modal.

**Architecture:** Two new nullable columns on the `task` table, threaded through the existing `Task`/`TaskInput` structs and the create/update SQL exactly like `priority`/`size`. Frontend adds two controls to the edit modal and two badges to the card. No new tables, views, or grouping layout.

**Tech Stack:** Rust + Axum + sqlx (Postgres), Svelte 5 frontend, Tailwind.

## Global Constraints

- `version` is free text (nullable, no DB constraint). `type` is nullable, check-constrained to exactly `feature`, `bug`, `enhancement`, `chore`, `docs`.
- Follow the existing `priority`/`size` pattern verbatim — same validation helper, same full-object PUT threading, same badge idiom.
- `type` is a Rust keyword: the struct field is `type_` with `#[serde(rename = "type")]`; the JSON/DB name is `type`.
- Run `cargo check` and `cargo test` before declaring the backend task done.

---

### Task 1: Backend — migration, model, route, validation

**Files:**
- Create: `migrations/0009_task_version_type.sql`
- Modify: `src/models.rs` (Task ~62-77, TaskInput ~80-91, consts ~95-97)
- Modify: `src/routes/tasks.rs` (imports line 10, create 65-86, update 102-125)
- Test: `tests/tasks.rs` (append)

**Interfaces:**
- Produces: `Task.version: Option<String>`, `Task.type_: Option<String>` (JSON key `type`); `TaskInput.version`, `TaskInput.type_`; `pub const TASK_TYPES: [&str; 5]`. The frontend (Task 2) sends/reads JSON keys `version` and `type`.

- [ ] **Step 1: Write the failing test**

Append to `tests/tasks.rs`:

```rust
#[sqlx::test]
async fn task_roundtrips_version_and_type(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;

    let (status, t) = send(
        &app,
        json_req("POST", "/api/tasks", json!({
            "title": "Ship v1", "version": "v1.0", "type": "feature"
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["version"], "v1.0");
    assert_eq!(t["type"], "feature");

    // Full-object PUT (drag path) must preserve both.
    let id = t["id"].as_str().unwrap().to_string();
    let (status, upd) = send(
        &app,
        json_req("PUT", &format!("/api/tasks/{id}"), json!({
            "title": "Ship v1", "status": "doing",
            "version": "v1.0", "type": "feature", "position": 0
        })).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["version"], "v1.0");
    assert_eq!(upd["type"], "feature");
}

#[sqlx::test]
async fn task_rejects_bad_type(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let (status, _) = send(
        &app,
        json_req("POST", "/api/tasks", json!({"title":"X","type":"wibble"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test tasks task_roundtrips_version_and_type task_rejects_bad_type`
Expected: FAIL — compile error (fields/const don't exist) or column-missing DB error.

- [ ] **Step 3: Add the migration**

Create `migrations/0009_task_version_type.sql`:

```sql
alter table task add column version text;
alter table task add column type    text check (type in ('feature','bug','enhancement','chore','docs'));
```

- [ ] **Step 4: Add the model fields and const**

In `src/models.rs`, add to `struct Task` (after `pub description: Option<String>,` at line 70):

```rust
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
```

Add to `struct TaskInput` (after its `pub description: Option<String>,` at line 87):

```rust
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
```

No derive changes are needed: `Task` already derives `Serialize` (so `#[serde(rename = "type")]` emits the JSON key `type`) and `TaskInput` derives `Deserialize` (so the rename reads the JSON key `type`). Add the const after line 97:

```rust
pub const TASK_TYPES: [&str; 5] = ["feature", "bug", "enhancement", "chore", "docs"];
```

- [ ] **Step 5: Thread the fields through the route**

In `src/routes/tasks.rs`, extend the import on line 10:

```rust
use crate::models::{Task, TaskInput, PRIORITY_LEVELS, TASK_SIZES, TASK_STATUSES, TASK_TYPES};
```

In `create`, after the existing `check_optional(&input.size, ...)?;` (line 64) add:

```rust
    check_optional(&input.type_, TASK_TYPES, "type")?;
```

Replace the `create` INSERT (lines 65-77) with:

```rust
    let row = sqlx::query_as::<_, Task>(
        "insert into task (project_id, title, status, due_on, priority, size, description, checklist, position, version, type) \
         values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) returning *",
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
    .bind(&input.version)
    .bind(&input.type_)
```

In `update`, after its `check_optional(&input.size, ...)?;` (line 101) add:

```rust
    check_optional(&input.type_, TASK_TYPES, "type")?;
```

Replace the `update` UPDATE (lines 102-115) with:

```rust
    let row = sqlx::query_as::<_, Task>(
        "update task set project_id=$2, title=$3, status=$4, due_on=$5, priority=$6, \
         size=$7, description=$8, checklist=$9, position=$10, version=$11, type=$12 where id=$1 returning *",
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
    .bind(&input.version)
    .bind(&input.type_)
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test tasks`
Expected: PASS — all task tests including the two new ones.

- [ ] **Step 7: Full check**

Run: `cargo check && cargo test`
Expected: clean build, all tests pass.

- [ ] **Step 8: Commit**

```bash
git add migrations/0009_task_version_type.sql src/models.rs src/routes/tasks.rs tests/tasks.rs
git commit -m "feat(tasks): add version (free-text) and type (enum) fields"
```

---

### Task 2: Frontend — modal editor + card badges

**Files:**
- Modify: `frontend/src/routes/ProjectBoard.svelte` (const ~15, saveTask 113-123, modal 383-404, card 264-270)

**Interfaces:**
- Consumes: task objects from `/api/tasks` now carry `version` (string|null) and `type` (string|null), per Task 1.

- [ ] **Step 1: Add the type badge color map**

In `frontend/src/routes/ProjectBoard.svelte`, after the `PRIORITY_BAR` const (line 15) add:

```js
  const TYPE_BADGE = {
    feature:     'text-st-done',
    bug:         'text-st-lost',
    enhancement: 'text-accent',
    chore:       'text-faint',
    docs:        'text-st-proposal',
  }
```

- [ ] **Step 2: Send version + type in saveTask**

In `saveTask` (the `api.put` payload, lines 113-123), add after `description: t.description || null,`:

```js
        version: t.version || null,
        type: t.type || null,
```

- [ ] **Step 3: Add Version + Type controls to the modal**

In the edit-task modal, after the `grid grid-cols-2` block that holds Priority/Size (closes at line 404), insert:

```svelte
      <div class="grid grid-cols-2 gap-3">
        <div>
          <p class="label mb-1.5">Version</p>
          <input bind:value={editingTask.version} placeholder="v1.0" class={FIELD} />
        </div>
        <div>
          <p class="label mb-1.5">Type</p>
          <select bind:value={editingTask.type} class={FIELD}>
            <option value={null}>— none —</option>
            <option value="feature">feature</option>
            <option value="bug">bug</option>
            <option value="enhancement">enhancement</option>
            <option value="chore">chore</option>
            <option value="docs">docs</option>
          </select>
        </div>
      </div>
```

- [ ] **Step 4: Render badges on the card**

In the card meta row, extend the `{#if ...}` guard on line 264 to also fire on version/type, and add the two badges. Replace lines 264-270:

```svelte
                  {#if t.due_on || t.size || t.checklist?.length || t.type || t.version}
                    <div class="mt-1 flex flex-wrap items-center gap-x-2.5 gap-y-1 font-mono text-[11px]">
                      {#if t.type}<span class="uppercase {TYPE_BADGE[t.type] ?? 'text-faint'}">{t.type}</span>{/if}
                      {#if t.version}<span class="text-faint">{t.version}</span>{/if}
                      {#if t.due_on}<span class={isOverdue(t) ? 'text-st-lost' : 'text-faint'}>{t.due_on}</span>{/if}
                      {#if t.size}<span class="uppercase text-faint">[{t.size}]</span>{/if}
                      {#if t.checklist?.length}<span class="text-faint">✓ {t.checklist.filter((c) => c.done).length}/{t.checklist.length}</span>{/if}
                    </div>
                  {/if}
```

- [ ] **Step 5: Verify the build**

Run: `cd frontend && npm run build`
Expected: build succeeds, no Svelte errors.

- [ ] **Step 6: Manual smoke check**

Run the app, open a project board, open a task, set Version = `v1.0` and Type = `bug`, save. Expected: card shows a red `BUG` badge and a `v1.0` tag; reopening the modal shows both values retained.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/routes/ProjectBoard.svelte
git commit -m "feat(web): task version + type badges and modal editors"
```

---

## Notes

- The per-column quick-add (`submitAdd`) intentionally does not send version/type — they default to NULL and are set later via the modal. No change needed there.
- `openTask` already spreads `{ ...t }`, so `editingTask.version` / `.type` bind correctly with no change.
