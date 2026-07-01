# Project Kanban (remove pipeline) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the lead Pipeline, add a Projects list that switches between each project's existing task kanban, and simplify project `status` to `active`/`archived`.

**Architecture:** Three tasks. Task 1 is all backend (migration + Rust + Rust tests) and must land together to compile. Task 2 removes the pipeline and adds the new Projects list + routing/nav. Task 3 fixes the remaining status/route consumers (ProjectBoard, ContactDetail, Dashboard). The per-project board (`ProjectBoard.svelte`) already exists and is not rebuilt.

**Tech Stack:** Rust + Axum 0.8 + sqlx 0.8 + Postgres; Svelte 5 (runes) SPA + svelte-spa-router + svelte-dnd-action; vitest.

## Global Constraints

- `time` crate stays pinned `=0.3.41` — do not touch `Cargo.toml`.
- Do NOT modify the `--color-st-lead/proposal/active/done/lost` tokens in `frontend/src/app.css`; they are a repo-wide palette (red errors via `st-lost`, etc.), not project-status-only.
- After any Rust change: `cargo test` must be green before commit.
- After any frontend change: `npm run build` and `npm test` (in `frontend/`) must be green before commit.
- Migration numbering continues from `0005`; next is `0006`.
- Project statuses after this plan: exactly `active`, `archived`. Task statuses unchanged (`todo`/`doing`/`done`).

---

### Task 1: Backend — status → active/archived, task counts, drop pipeline endpoint

**Files:**
- Create: `migrations/0006_project_status_active_archived.sql`
- Modify: `src/models.rs` (Project struct, remove ProjectMove, PROJECT_STATUSES)
- Modify: `src/routes/projects.rs` (list, create, update, remove move_)
- Modify: `src/app.rs` (remove /move route)
- Modify: `src/routes/dashboard.rs` (Counts, queries)
- Test: `tests/projects.rs`, `tests/dashboard.rs`

**Interfaces:**
- Produces:
  - `Project` gains `task_count: i64` (populated by the list query, defaults to 0 elsewhere); loses `board_order`.
  - `GET /api/projects[?status=active|archived]` returns objects with a `task_count` field.
  - `PUT /api/projects/{id}` now honors an optional `status` field (used to archive/restore); omitting it preserves the current status.
  - `POST /api/projects` defaults status to `active`.
  - `GET /api/dashboard` `counts` object is `{ projects, active, open_tasks }` (was `{ leads, active, open_tasks }`).
  - `PATCH /api/projects/{id}/move` and `ProjectMove` are removed.

- [ ] **Step 1: Write the migration**

Create `migrations/0006_project_status_active_archived.sql`:

```sql
-- Collapse the sales-funnel statuses onto active/archived and drop pipeline ordering.
alter table project drop constraint if exists project_status_check;

update project set status = case
    when status in ('done', 'lost') then 'archived'
    else 'active'
end;

alter table project alter column status set default 'active';
alter table project add constraint project_status_check
    check (status in ('active', 'archived'));

alter table project drop column board_order;
```

- [ ] **Step 2: Update `src/models.rs`**

Replace the `Project` struct (remove `board_order`, add `task_count`), delete `ProjectMove`, and set `PROJECT_STATUSES`.

Change the `Project` struct to:

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub contact_id: Uuid,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
    #[sqlx(default)]
    pub task_count: i64,
    pub created_at: DateTime<Utc>,
}
```

Delete the entire `ProjectMove` struct:

```rust
#[derive(Debug, Deserialize)]
pub struct ProjectMove {
    pub status: String,
    pub board_order: i32,
}
```

Change the constant:

```rust
pub const PROJECT_STATUSES: [&str; 2] = ["active", "archived"];
```

- [ ] **Step 3: Update `src/routes/projects.rs`**

Fix the import (drop `ProjectMove`):

```rust
use crate::models::{Project, ProjectInput, PROJECT_STATUSES};
```

Replace the `list` handler body (add task_count join, order by created_at):

```rust
pub async fn list(
    _: AuthUser,
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Project>>, AppError> {
    let rows = match q.status {
        Some(st) => {
            check_status(&st)?;
            sqlx::query_as::<_, Project>(
                "select p.*, count(t.id) as task_count from project p \
                 left join task t on t.project_id = p.id \
                 where p.status = $1 group by p.id order by p.created_at",
            )
            .bind(st)
            .fetch_all(&s.pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, Project>(
                "select p.*, count(t.id) as task_count from project p \
                 left join task t on t.project_id = p.id \
                 group by p.id order by p.created_at",
            )
            .fetch_all(&s.pool)
            .await?
        }
    };
    Ok(Json(rows))
}
```

In `create`, change the default status:

```rust
    let status = input.status.clone().unwrap_or_else(|| "active".to_string());
```

In `update`, honor an optional status via COALESCE. Add a status check near the top of the fn (after the title check):

```rust
    if let Some(ref st) = input.status {
        check_status(st)?;
    }
```

and change the update query + add the bind:

```rust
    let row = sqlx::query_as::<_, Project>(
        "update project set contact_id=$2, title=$3, description=$4, invoice_url=$5, \
         status=coalesce($6, status) where id=$1 returning *",
    )
    .bind(id)
    .bind(input.contact_id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.invoice_url)
    .bind(&input.status)
```

(the `.map_err(...).ok_or(AppError::NotFound)?` tail stays unchanged.)

Delete the entire `move_` function (lines 122-139 in the current file).

- [ ] **Step 4: Update `src/app.rs`**

Remove the pipeline move route line:

```rust
        .route("/api/projects/{id}/move", patch(projects::move_))
```

Leave the `patch` import — it is still used by `/api/events/{id}/series`.

- [ ] **Step 5: Update `src/routes/dashboard.rs`**

Rename the count field and fix the two queries. Change the `Counts` struct:

```rust
#[derive(Serialize)]
pub struct Counts {
    pub projects: i64,
    pub active: i64,
    pub open_tasks: i64,
}
```

Change the active-projects query ordering (drop `board_order`):

```rust
    let active_projects = sqlx::query_as::<_, Project>(
        "select * from project where status = 'active' order by created_at",
    )
```

Replace the `leads` scalar with a total `projects` scalar:

```rust
    let projects: i64 = sqlx::query_scalar("select count(*) from project")
        .fetch_one(&s.pool)
        .await?;
```

Update the struct construction:

```rust
        counts: Counts { projects, active, open_tasks },
```

- [ ] **Step 6: Update `tests/projects.rs`**

Rename the defaults test and assert `active`:

```rust
#[sqlx::test]
async fn project_create_defaults_to_active(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;

    let (status, p) = send(
        &app,
        json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"Website"})).with_cookie(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(p["status"], "active");
}
```

Delete the whole `project_move_changes_status_and_order` test (the `/move` endpoint is gone).

In `project_list_filters_by_status`, change the second project's status from `"lead"` to `"archived"`, and assert the returned `task_count`:

```rust
#[sqlx::test]
async fn project_list_filters_by_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"A","status":"active"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"B","status":"archived"})).with_cookie(&cookie)).await;

    let (status, list) = send(&app, json_req("GET", "/api/projects?status=active", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "A");
    assert_eq!(list[0]["task_count"], 0);
}
```

Add a new test asserting archive-via-PUT sets status:

```rust
#[sqlx::test]
async fn project_update_can_set_status(pool: sqlx::PgPool) {
    std::env::set_var("APP_PASSWORD", "secret");
    let app = test_app(pool);
    let cookie = login(&app, "secret").await;
    let contact_id = make_contact(&app, &cookie).await;
    let (_, p) = send(&app, json_req("POST", "/api/projects", json!({"contact_id": contact_id, "title":"A","status":"active"})).with_cookie(&cookie)).await;
    let id = p["id"].as_str().unwrap().to_string();

    let (status, upd) = send(&app, json_req("PUT", &format!("/api/projects/{id}"), json!({"contact_id": contact_id, "title":"A", "status":"archived"})).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upd["status"], "archived");
}
```

(`project_rejects_bad_status`, `project_update_preserves_status`, `project_update_rejects_bad_contact_id` stay as-is — the first still rejects `"wat"`, the second now also verifies COALESCE preserves status on a status-less PUT.)

- [ ] **Step 7: Update `tests/dashboard.rs`**

In `dashboard_aggregates`, change the second project to `archived` and swap the `leads` assertion for a total `projects` one:

```rust
    send(&app, json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"Active one","status":"active"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/projects", json!({"contact_id":cid,"title":"Archived one","status":"archived"})).with_cookie(&cookie)).await;
    send(&app, json_req("POST", "/api/tasks", json!({"title":"Open task"})).with_cookie(&cookie)).await;

    let (status, d) = send(&app, json_req("GET", "/api/dashboard", json!(null)).with_cookie(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(d["active_projects"].as_array().unwrap().len(), 1);
    assert_eq!(d["counts"]["projects"], 2);
    assert_eq!(d["counts"]["active"], 1);
    assert_eq!(d["counts"]["open_tasks"], 1);
    assert!(d["upcoming_events"].is_array());
```

- [ ] **Step 8: Run the backend tests**

Run: `cargo test`
Expected: PASS — all project/dashboard tests green, no reference to `move_`/`ProjectMove`/`board_order` remains (compiler confirms). If the DB constraint name differs, the `drop constraint if exists` no-ops and the new constraint still applies.

- [ ] **Step 9: Commit**

```bash
git add migrations/0006_project_status_active_archived.sql src/models.rs src/routes/projects.rs src/app.rs src/routes/dashboard.rs tests/projects.rs tests/dashboard.rs
git commit -m "feat(projects): status active/archived, per-project task counts; drop pipeline move endpoint"
```

---

### Task 2: Frontend — delete pipeline, add Projects list, wire routing/nav

**Files:**
- Delete: `frontend/src/routes/Pipeline.svelte`, `frontend/src/lib/pipeline.js`, `frontend/src/lib/pipeline.test.js`
- Create: `frontend/src/routes/Projects.svelte`
- Modify: `frontend/src/App.svelte` (import, route, title)
- Modify: `frontend/src/lib/components/Sidebar.svelte` (nav item)

**Interfaces:**
- Consumes: `GET /api/projects?status=…` (with `task_count`) and `GET /api/contacts` from Task 1.
- Produces: route `/projects` → `Projects.svelte`; sidebar item "Projects". Cards navigate to `/projects/:id` (existing `ProjectBoard`).

- [ ] **Step 1: Delete the pipeline files**

```bash
git rm frontend/src/routes/Pipeline.svelte frontend/src/lib/pipeline.js frontend/src/lib/pipeline.test.js
```

- [ ] **Step 2: Create `frontend/src/routes/Projects.svelte`**

```svelte
<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let filter = $state('active')            // 'active' | 'archived'
  let projects = $state([])
  let contacts = $state([])
  let error = $state('')
  let creating = $state(null)              // {contact_id, title} | null

  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))

  async function load() {
    error = ''
    try {
      ;[projects, contacts] = await Promise.all([
        api.get('/api/projects?status=' + filter),
        api.get('/api/contacts'),
      ])
    } catch (e) { error = e.message }
  }

  function openNew() { creating = { contact_id: contacts[0]?.id ?? '', title: '' } }
  async function createProject(e) {
    e.preventDefault()
    if (!creating.contact_id || !creating.title.trim()) return
    try {
      await api.post('/api/projects', { contact_id: creating.contact_id, title: creating.title.trim() })
      creating = null
      await load()
    } catch (err) { error = err.message }
  }

  $effect(() => { filter; load() })
</script>

<div class="rise mb-5 flex items-center justify-between">
  <div class="flex gap-1.5">
    {#each ['active', 'archived'] as f}
      <button
        onclick={() => (filter = f)}
        class="h-8 rounded-sm border px-3 font-mono text-[12px] lowercase transition"
        class:border-accent={filter === f}
        class:text-accent={filter === f}
        class:glow-soft={filter === f}
        class:border-border={filter !== f}
        class:text-faint={filter !== f}
      >{f}</button>
    {/each}
  </div>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> New project
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<div class="rise grid grid-cols-2 gap-3 md:grid-cols-3" style="animation-delay:40ms">
  {#each projects as p (p.id)}
    <button
      onclick={() => push('/projects/' + p.id)}
      class="group relative overflow-hidden rounded-sm border border-border bg-surface p-4 text-left transition-all duration-100 hover:-translate-y-px hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
    >
      <div class="font-mono text-[13px] font-medium leading-snug text-ink">{p.title}</div>
      <div class="mt-1 truncate font-mono text-[11px] text-faint">{contactsById[p.contact_id] ?? '—'}</div>
      <div class="mt-3 font-mono text-[11px] tabular-nums text-faint"><span class="text-accent-dim">&gt;</span> {p.task_count} task{p.task_count === 1 ? '' : 's'}</div>
    </button>
  {:else}
    <p class="col-span-full py-10 text-center font-mono text-[13px] text-faint">// no {filter} projects</p>
  {/each}
</div>

{#if creating}
  <Modal title="New project" onclose={() => (creating = null)}>
    <form onsubmit={createProject} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Contact</p>
        <select bind:value={creating.contact_id} required class={FIELD}>
          {#each contacts as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </div>
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={creating.title} placeholder="Project title" required class={FIELD} />
      </div>
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Create project</button>
    </form>
  </Modal>
{/if}
```

- [ ] **Step 3: Update `frontend/src/App.svelte`**

Replace the Pipeline import (line 7):

```js
  import Projects from './routes/Projects.svelte'
```

In the `routes` object, replace the `'/pipeline': Pipeline,` entry with:

```js
    '/projects': Projects,
```

In the `TITLES` object, replace `'/pipeline': 'Pipeline',` with:

```js
    '/projects': 'Projects',
```

- [ ] **Step 4: Update `frontend/src/lib/components/Sidebar.svelte`**

Replace the Pipeline nav item (line 11) and fix the stale comment on line 7.

Change the comment block (lines 6-7) to:

```js
  // Numbered terminal nav. Source labels stay natural-case (e2e matches
  // <a> textContent on 'Contacts'/'Projects'); CSS lowercases for the look.
```

Change the nav item:

```js
    { href: '/projects', n: '03', label: 'Projects' },
```

- [ ] **Step 5: Build and test**

Run: `cd frontend && npm run build && npm test`
Expected: build succeeds; vitest PASS (`tasks.test.js`, `calendar.test.js`, `guard`/session tests remain; `pipeline.test.js` is gone). No import of `pipeline.js` remains.

- [ ] **Step 6: Commit**

```bash
git add -A frontend/src
git commit -m "feat(web): Projects list page + nav; remove Pipeline board"
```

---

### Task 3: Frontend — fix status/route consumers (board, contact detail, dashboard)

**Files:**
- Modify: `frontend/src/routes/ProjectBoard.svelte` (back link, delete redirect, status map, archive button)
- Modify: `frontend/src/routes/ContactDetail.svelte` (status color map)
- Modify: `frontend/src/routes/Dashboard.svelte` (stat tile → Projects count)

**Interfaces:**
- Consumes: `counts.projects` from Task 1's dashboard; `PUT /api/projects/{id}` honoring `status` from Task 1.

- [ ] **Step 1: `ProjectBoard.svelte` — status color map**

Replace `PROJECT_STATUS_TEXT` (lines 14-17) with the two live statuses:

```js
  const PROJECT_STATUS_TEXT = {
    active: 'text-st-active', archived: 'text-muted',
  }
```

- [ ] **Step 2: `ProjectBoard.svelte` — add the archive/restore action**

Add this function after `deleteProject` (around line 109):

```js
  async function toggleArchive() {
    const next = project.status === 'archived' ? 'active' : 'archived'
    try {
      await api.put('/api/projects/' + id, {
        contact_id: project.contact_id,
        title: project.title,
        description: project.description || null,
        invoice_url: project.invoice_url || null,
        status: next,
      })
      await load()
    } catch (e) { error = e.message }
  }
```

In the header actions (the `<div class="ml-auto flex gap-2">` block, lines 143-152), add an archive button before the `edit` button:

```svelte
      <button
        onclick={toggleArchive}
        class="h-8 rounded-sm border border-border-2 px-3 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
      >{project?.status === 'archived' ? 'restore' : 'archive'}</button>
```

- [ ] **Step 3: `ProjectBoard.svelte` — repoint pipeline references to /projects**

Change the post-delete redirect (line 107) from `push('/pipeline')` to:

```js
    try { await api.del('/api/projects/' + id); push('/projects') }
```

Change the back button (lines 133-136):

```svelte
    <button
      onclick={() => push('/projects')}
      class="font-mono text-[12px] text-faint transition hover:text-accent"
    >&lt; projects</button>
```

- [ ] **Step 4: `ContactDetail.svelte` — status color map**

Replace `PROJECT_STATUS_TEXT` (lines 6-9) with:

```js
  const PROJECT_STATUS_TEXT = {
    active: 'text-st-active', archived: 'text-muted',
  }
```

- [ ] **Step 5: `Dashboard.svelte` — Projects stat tile**

Change the `counts` initial state (line 5):

```js
  let counts = $state({ projects: 0, active: 0, open_tasks: 0 })
```

Change the first tile (line 24) from `Leads` to `Projects`:

```js
    { label: 'Projects', value: counts.projects, accent: false },
```

- [ ] **Step 6: Build and test**

Run: `cd frontend && npm run build && npm test`
Expected: build + vitest PASS. Manually confirm (dev server) that a project board's `archive` button flips the badge to `[ archived ]` and the card then shows only under the Archived filter.

- [ ] **Step 7: Commit**

```bash
git add -A frontend/src
git commit -m "fix(web): repoint board/contact/dashboard to active-archived + /projects"
```

---

## Self-Review

**Spec coverage:**
- Delete pipeline (frontend files, routes, nav, backend move endpoint/struct/board_order) → Task 1 (backend) + Task 2 (frontend). ✅
- Add Projects list (cards, task count, active/archived filter, New project modal) → Task 2. ✅
- Sidebar Pipeline→Projects, App route/title → Task 2. ✅
- ProjectBoard back link/redirect + archive button → Task 3. ✅
- Status active/archived migration + models + PROJECT_STATUSES → Task 1. ✅
- Task count per project (backend list) → Task 1 (Project.task_count + join). ✅
- Dashboard leads→projects, order by created_at → Task 1 + Task 3. ✅
- ContactDetail status map → Task 3. ✅
- Tests (delete pipeline.test, update projects/dashboard rust tests, migration remap covered by dashboard/list tests using archived) → Task 1 (rust) + Task 2 (vitest). ✅

**Placeholder scan:** No TBD/TODO; every code step shows full code. ✅

**Type consistency:** `task_count: i64` (Rust) ↔ `p.task_count` (Svelte). `Counts.projects` (Rust) ↔ `counts.projects` (Svelte). `toggleArchive` defined and referenced in the same file. Route `/projects` used in App, Sidebar, ProjectBoard, Projects card navigation. ✅

**Note on the one unavoidable large task:** Task 1 groups all backend edits because removing `ProjectMove`/`board_order` and the `/move` route must compile together; it is split into bite-sized steps rather than sub-tasks to keep the tree compiling at the commit boundary.
