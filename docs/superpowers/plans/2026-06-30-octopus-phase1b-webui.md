# Octopus Phase 1B — Web UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Svelte web UI for Octopus on top of the Phase 1A JSON API — login, a navigable app shell, a dashboard, contacts management, and a drag-and-drop pipeline kanban — served as static files by the existing Axum server.

**Architecture:** A Vite + Svelte 5 single-page app in `frontend/`, built into the repo's `static/` directory (which Axum already serves with an index fallback). The SPA talks only to the `/api` JSON endpoints via a single fetch wrapper that sends the session cookie. Local dev uses `vite dev` with a proxy to the running Rust API; production builds the SPA in a Node stage of the Dockerfile before the Rust build.

**Tech Stack:** Svelte 5 (runes), Vite 8, Tailwind CSS 4 (via `@tailwindcss/vite`), `svelte-spa-router` (hash routing), `svelte-dnd-action` (kanban). Vitest for a few focused unit tests; primary verification is a manual end-to-end run.

## Global Constraints

- **Verified toolchain (compile-tested on this machine before this plan):** Node 25 / npm 11; `svelte@^5.56`, `vite@^8.1`, `@sveltejs/vite-plugin-svelte@^7.1`, `tailwindcss@^4.3`, `@tailwindcss/vite@^4.3`, `svelte-spa-router@^5.1`, `svelte-dnd-action@^0.9.70`. A minimal app with Tailwind classes + `dndzone` + `$state` + `mount()` builds to a configurable `outDir`. Do not change these major versions.
- **Svelte 5 idioms:** mount with `import { mount } from 'svelte'; mount(App, { target })` — NOT `new App()`. State via `$state`/`$derived` runes. Props via `$props()`. Events on DOM/components use attribute syntax `onclick={...}`, `onconsider={...}` (NOT `on:click`).
- **Tailwind 4:** no `tailwind.config.js` and no PostCSS config — just `@import "tailwindcss";` in the CSS entry and the `@tailwindcss/vite` plugin.
- **API base path is `/api`.** All calls go through the fetch wrapper in `src/lib/api.js`; every call sends `credentials: 'include'` so the signed session cookie rides along. A `401` from any call means "not logged in" → redirect to the login route.
- **The API is Phase 1A as built.** Endpoints and shapes (do not invent others):
  - `POST /api/login {password}` → 204 + cookie | 401; `POST /api/logout` → 204
  - `GET /api/dashboard` → `{active_projects:[Project], due_tasks:[Task], counts:{leads,active,open_tasks}}`
  - `GET/POST /api/contacts`, `GET/PUT/DELETE /api/contacts/{id}` — Contact `{id,kind,name,email,phone,company_id,created_at}`; kind ∈ {person,company}
  - `GET /api/projects` (optional `?status=`), `POST /api/projects`, `GET/PUT/DELETE /api/projects/{id}`, `PATCH /api/projects/{id}/move {status,board_order}` — Project `{id,contact_id,title,status,description,invoice_url,board_order,created_at}`; status ∈ {lead,proposal,active,done,lost}
  - `GET /api/tasks` (optional `?project_id=`), `POST /api/tasks`, `PUT/DELETE /api/tasks/{id}` — Task `{id,project_id,title,status,due_on,created_at}`; status ∈ {todo,doing,done}
- **PUT is full-replace** (send the whole object). For projects, `PUT` does NOT change `status`/`board_order` (those are owned by `/move`) — so editing a project's title/description/contact via PUT is safe and preserves pipeline position.
- **No new API endpoints.** Where the UI needs "projects for a contact," the API has no `?contact_id` filter — fetch all projects and filter client-side (single-user scale; mark with a comment).
- **`static/` becomes a build artifact** — git-ignored, produced by `vite build`. The committed placeholder `static/index.html` is removed.

## File Structure

```
frontend/
  package.json
  vite.config.js
  svelte.config.js
  index.html
  src/
    main.js              # mount App, install router
    app.css              # @import "tailwindcss";
    App.svelte           # router outlet + auth gate + nav shell
    lib/
      api.js             # fetch wrapper (get/post/put/patch/del) + 401 handling
      session.js         # $state-based auth store + helpers
      contacts.js        # client-side cache/helpers for joining contact names (optional)
    routes/
      Login.svelte
      Dashboard.svelte
      Contacts.svelte
      Pipeline.svelte
    lib/components/
      Nav.svelte
      Modal.svelte       # small reusable dialog for forms
    lib/api.test.js      # vitest: wrapper behavior
    lib/pipeline.test.js # vitest: column/move payload logic
Dockerfile               # +Node build stage (modified)
.dockerignore            # +frontend/node_modules, frontend/dist (modified)
.gitignore               # +/static, frontend/node_modules (modified)
README.md                # frontend dev/build notes (modified)
```

---

### Task 1: Frontend scaffold + build pipeline wired to `static/`

**Files:**
- Create: `frontend/package.json`, `frontend/vite.config.js`, `frontend/svelte.config.js`, `frontend/index.html`, `frontend/src/main.js`, `frontend/src/app.css`, `frontend/src/App.svelte`
- Modify: `.gitignore` (root), `README.md`
- Delete: `static/index.html` (placeholder, now a build artifact)

**Interfaces:**
- Produces: `npm --prefix frontend run build` emits the SPA into repo `static/`; `npm --prefix frontend run dev` serves on :5173 proxying `/api` → :8090. `App.svelte` renders a visible heading (router added in Task 4).

- [ ] **Step 1: Write `frontend/package.json`**

```json
{
  "name": "octopus-frontend",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run"
  },
  "dependencies": {
    "svelte-spa-router": "^5.1.1",
    "svelte-dnd-action": "^0.9.70"
  },
  "devDependencies": {
    "svelte": "^5.56.4",
    "vite": "^8.1.1",
    "@sveltejs/vite-plugin-svelte": "^7.1.2",
    "tailwindcss": "^4.3.2",
    "@tailwindcss/vite": "^4.3.2",
    "vitest": "^3",
    "jsdom": "^25"
  }
}
```

- [ ] **Step 2: Write `frontend/vite.config.js`**

```js
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  // Build straight into the dir Axum serves (../static at repo root).
  build: { outDir: '../static', emptyOutDir: true },
  server: {
    port: 5173,
    proxy: { '/api': 'http://localhost:8090' }, // Rust API in local dev
  },
  test: { environment: 'jsdom' },
})
```

- [ ] **Step 3: Write `frontend/svelte.config.js`**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'
export default { preprocess: vitePreprocess() }
```

- [ ] **Step 4: Write `frontend/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Octopus</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
```

- [ ] **Step 5: Write `frontend/src/app.css`**

```css
@import "tailwindcss";
```

- [ ] **Step 6: Write `frontend/src/main.js`**

```js
import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'

export default mount(App, { target: document.getElementById('app') })
```

- [ ] **Step 7: Write a placeholder `frontend/src/App.svelte`** (replaced in Task 4)

```svelte
<h1 class="p-6 text-2xl font-bold text-slate-800">Octopus — scaffold OK</h1>
```

- [ ] **Step 8: Update root `.gitignore`**

Append:

```
/static
frontend/node_modules
```

- [ ] **Step 9: Remove the committed placeholder**

```bash
git rm static/index.html
```

(`static/` is now produced by the build and ignored. The Axum `ServeDir` still points at `static/`; it serves whatever the build emits.)

- [ ] **Step 10: Install and build to verify**

```bash
cd frontend && npm install --no-fund --no-audit
npm run build
ls ../static/index.html ../static/assets    # both should exist
```
Expected: build succeeds; `../static/index.html` + `../static/assets/*` produced.

- [ ] **Step 11: Update `README.md` frontend notes**

Under "Local development", add:

````markdown
### Frontend (web UI)

- One-time: `cd frontend && npm install`.
- Dev with hot reload: run the Rust API (`PORT=8090 cargo run`) in one terminal,
  then `cd frontend && npm run dev` → open http://localhost:5173 (it proxies
  `/api` to the Rust server on :8090).
- Production build: `cd frontend && npm run build` emits the SPA into `static/`,
  which the Rust server serves on its own port. `static/` is git-ignored.
````

- [ ] **Step 12: Commit**

```bash
git add frontend/ .gitignore README.md
git commit -m "feat(web): scaffold svelte+vite+tailwind frontend building into static/"
```

---

### Task 2: API client (`api.js`) + auth session store

**Files:**
- Create: `frontend/src/lib/api.js`, `frontend/src/lib/session.js`, `frontend/src/lib/api.test.js`

**Interfaces:**
- Produces:
  - `api.get(path)`, `api.post(path, body)`, `api.put(path, body)`, `api.patch(path, body)`, `api.del(path)` — all async, return parsed JSON (or `null` for 204), throw `ApiError {status, message}` on non-2xx. On `401` they also call `session.markLoggedOut()`.
  - `session` store: `session.authed` (a `$state` boolean accessor via `getAuthed()`), `markLoggedIn()`, `markLoggedOut()`.

- [ ] **Step 1: Write `frontend/src/lib/session.js`**

```js
// Tiny auth flag shared across the app. Runes work in .svelte.js modules.
let authed = $state(false)

export function getAuthed() { return authed }
export function markLoggedIn() { authed = true }
export function markLoggedOut() { authed = false }
```

Note: rune state in a plain `.js` module requires the file to be `.svelte.js`. Rename to `session.svelte.js` and import accordingly. (Vite's Svelte plugin compiles `*.svelte.js` with runes.)

So: create `frontend/src/lib/session.svelte.js` with the content above.

- [ ] **Step 2: Write `frontend/src/lib/api.js`**

```js
import { markLoggedOut } from './session.svelte.js'

export class ApiError extends Error {
  constructor(status, message) {
    super(message)
    this.status = status
  }
}

async function request(method, path, body) {
  const opts = {
    method,
    credentials: 'include',
    headers: {},
  }
  if (body !== undefined) {
    opts.headers['content-type'] = 'application/json'
    opts.body = JSON.stringify(body)
  }
  const res = await fetch(path, opts)
  if (res.status === 401) {
    markLoggedOut()
    throw new ApiError(401, 'unauthorized')
  }
  if (res.status === 204) return null
  let data = null
  const text = await res.text()
  if (text) {
    try { data = JSON.parse(text) } catch { data = text }
  }
  if (!res.ok) {
    const msg = (data && data.error) || `request failed (${res.status})`
    throw new ApiError(res.status, msg)
  }
  return data
}

export const api = {
  get: (p) => request('GET', p),
  post: (p, b) => request('POST', p, b),
  put: (p, b) => request('PUT', p, b),
  patch: (p, b) => request('PATCH', p, b),
  del: (p) => request('DELETE', p),
}
```

- [ ] **Step 3: Write `frontend/src/lib/api.test.js` (Vitest)**

```js
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api, ApiError } from './api.js'

function mockFetch(status, payload) {
  globalThis.fetch = vi.fn().mockResolvedValue({
    status,
    ok: status >= 200 && status < 300,
    text: async () => (payload === undefined ? '' : JSON.stringify(payload)),
  })
}

describe('api wrapper', () => {
  beforeEach(() => { vi.restoreAllMocks() })

  it('returns parsed JSON on 200', async () => {
    mockFetch(200, { name: 'Acme' })
    expect(await api.get('/api/contacts/1')).toEqual({ name: 'Acme' })
  })

  it('returns null on 204', async () => {
    mockFetch(204)
    expect(await api.post('/api/login', { password: 'x' })).toBeNull()
  })

  it('throws ApiError with server message on 400', async () => {
    mockFetch(400, { error: 'name is required' })
    await expect(api.post('/api/contacts', {})).rejects.toMatchObject({
      status: 400, message: 'name is required',
    })
  })

  it('throws ApiError(401) on unauthorized', async () => {
    mockFetch(401, { error: 'unauthorized' })
    await expect(api.get('/api/dashboard')).rejects.toBeInstanceOf(ApiError)
  })

  it('sends credentials and JSON content-type on POST', async () => {
    mockFetch(201, { id: '1' })
    await api.post('/api/contacts', { name: 'A' })
    const [, opts] = globalThis.fetch.mock.calls[0]
    expect(opts.credentials).toBe('include')
    expect(opts.headers['content-type']).toBe('application/json')
  })
})
```

- [ ] **Step 4: Run the tests**

```bash
cd frontend && npm run test
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/
git commit -m "feat(web): api fetch wrapper + auth session store with tests"
```

---

### Task 3: Login route

**Files:**
- Create: `frontend/src/routes/Login.svelte`

**Interfaces:**
- Consumes: `api.post`, `session.markLoggedIn`.
- Produces: a `Login` component that POSTs `/api/login`, on success calls `markLoggedIn()` and navigates to `/` (`push('/')` from `svelte-spa-router`), on 401 shows an error.

- [ ] **Step 1: Write `frontend/src/routes/Login.svelte`**

```svelte
<script>
  import { push } from 'svelte-spa-router'
  import { api, ApiError } from '../lib/api.js'
  import { markLoggedIn } from '../lib/session.svelte.js'

  let password = $state('')
  let error = $state('')
  let busy = $state(false)

  async function submit(e) {
    e.preventDefault()
    error = ''
    busy = true
    try {
      await api.post('/api/login', { password })
      markLoggedIn()
      push('/')
    } catch (err) {
      error = err instanceof ApiError && err.status === 401
        ? 'Wrong password.'
        : 'Login failed.'
    } finally {
      busy = false
    }
  }
</script>

<div class="flex min-h-screen items-center justify-center bg-slate-100">
  <form onsubmit={submit} class="w-80 rounded-lg bg-white p-6 shadow">
    <h1 class="mb-4 text-xl font-bold text-slate-800">Octopus</h1>
    <input
      type="password"
      bind:value={password}
      placeholder="Password"
      autocomplete="current-password"
      class="mb-3 w-full rounded border border-slate-300 px-3 py-2"
    />
    {#if error}<p class="mb-3 text-sm text-red-600">{error}</p>{/if}
    <button
      type="submit"
      disabled={busy}
      class="w-full rounded bg-blue-600 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-50"
    >
      {busy ? 'Signing in…' : 'Sign in'}
    </button>
  </form>
</div>
```

- [ ] **Step 2: Verify build still passes**

```bash
cd frontend && npm run build
```
Expected: build succeeds (component compiles). (Visual check happens in Task 9.)

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/Login.svelte
git commit -m "feat(web): login screen"
```

---

### Task 4: App shell, router, auth gate, nav

**Files:**
- Create: `frontend/src/lib/components/Nav.svelte`
- Replace: `frontend/src/App.svelte`

**Interfaces:**
- Consumes: `svelte-spa-router` `Router`, the route components, `session` store, `api.post` (logout).
- Produces: hash routes `'/'`→Dashboard, `'/contacts'`→Contacts, `'/pipeline'`→Pipeline, `'/login'`→Login. Unauthenticated access to any non-login route redirects to `/login`. `Nav` shows links + logout.

- [ ] **Step 1: Write `frontend/src/lib/components/Nav.svelte`**

```svelte
<script>
  import { link, location, push } from 'svelte-spa-router'
  import { api } from '../api.js'
  import { markLoggedOut } from '../session.svelte.js'

  const items = [
    { href: '/', label: 'Dashboard' },
    { href: '/contacts', label: 'Contacts' },
    { href: '/pipeline', label: 'Pipeline' },
  ]

  async function logout() {
    try { await api.post('/api/logout') } catch { /* ignore */ }
    markLoggedOut()
    push('/login')
  }
</script>

<nav class="flex items-center gap-1 border-b bg-white px-4 py-2">
  <span class="mr-4 font-bold text-blue-600">🐙 Octopus</span>
  {#each items as it}
    <a
      href={it.href}
      use:link
      class="rounded px-3 py-1 text-sm hover:bg-slate-100"
      class:bg-slate-200={$location === it.href}
    >{it.label}</a>
  {/each}
  <button onclick={logout} class="ml-auto rounded px-3 py-1 text-sm text-slate-600 hover:bg-slate-100">
    Logout
  </button>
</nav>
```

- [ ] **Step 2: Replace `frontend/src/App.svelte`**

```svelte
<script>
  import Router, { push, location } from 'svelte-spa-router'
  import Nav from './lib/components/Nav.svelte'
  import Login from './routes/Login.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import Contacts from './routes/Contacts.svelte'
  import Pipeline from './routes/Pipeline.svelte'
  import { getAuthed, markLoggedIn } from './lib/session.svelte.js'
  import { api } from './lib/api.js'

  const routes = {
    '/': Dashboard,
    '/contacts': Contacts,
    '/pipeline': Pipeline,
    '/login': Login,
  }

  // On first load, probe the session: a successful /api/dashboard means the
  // cookie is still valid; a 401 leaves us logged out and the gate redirects.
  let ready = $state(false)
  $effect(() => {
    api.get('/api/dashboard')
      .then(() => markLoggedIn())
      .catch(() => {})
      .finally(() => { ready = true })
  })

  // Auth gate: if not authed and not already on /login, redirect.
  $effect(() => {
    if (ready && !getAuthed() && $location !== '/login') push('/login')
  })
</script>

{#if !ready}
  <div class="p-6 text-slate-500">Loading…</div>
{:else}
  {#if getAuthed() && $location !== '/login'}
    <Nav />
  {/if}
  <main class="mx-auto max-w-5xl p-4">
    <Router {routes} />
  </main>
{/if}
```

- [ ] **Step 3: Add stub route components so the build compiles**

Create minimal stubs (replaced in Tasks 5–7):

`frontend/src/routes/Dashboard.svelte`:
```svelte
<h1 class="text-xl font-bold">Dashboard</h1>
```
`frontend/src/routes/Contacts.svelte`:
```svelte
<h1 class="text-xl font-bold">Contacts</h1>
```
`frontend/src/routes/Pipeline.svelte`:
```svelte
<h1 class="text-xl font-bold">Pipeline</h1>
```

- [ ] **Step 4: Build to verify**

```bash
cd frontend && npm run build
```
Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/App.svelte frontend/src/lib/components/Nav.svelte frontend/src/routes/
git commit -m "feat(web): app shell, hash router, auth gate, nav"
```

---

### Task 5: Dashboard view

**Files:**
- Replace: `frontend/src/routes/Dashboard.svelte`

**Interfaces:**
- Consumes: `api.get('/api/dashboard')`, `api.get('/api/contacts')` (to join contact names), `api.post('/api/tasks')`, `api.put('/api/tasks/{id}')`.
- Produces: dashboard showing the three counts, active projects (with client name + status), due tasks (with a done toggle), and a quick "add task" input.

- [ ] **Step 1: Write `frontend/src/routes/Dashboard.svelte`**

```svelte
<script>
  import { api } from '../lib/api.js'

  let counts = $state({ leads: 0, active: 0, open_tasks: 0 })
  let activeProjects = $state([])
  let dueTasks = $state([])
  let contactsById = $state({})
  let newTask = $state('')
  let error = $state('')

  async function load() {
    error = ''
    try {
      const [dash, contacts] = await Promise.all([
        api.get('/api/dashboard'),
        api.get('/api/contacts'),
      ])
      counts = dash.counts
      activeProjects = dash.active_projects
      dueTasks = dash.due_tasks
      contactsById = Object.fromEntries(contacts.map((c) => [c.id, c.name]))
    } catch (e) { error = e.message }
  }

  async function addTask() {
    const title = newTask.trim()
    if (!title) return
    await api.post('/api/tasks', { title })
    newTask = ''
    await load()
  }

  async function toggleDone(t) {
    await api.put(`/api/tasks/${t.id}`, { title: t.title, status: 'done', project_id: t.project_id, due_on: t.due_on })
    await load()
  }

  $effect(() => { load() })
</script>

<h1 class="mb-4 text-2xl font-bold text-slate-800">Dashboard</h1>
{#if error}<p class="mb-3 text-red-600">{error}</p>{/if}

<div class="mb-6 grid grid-cols-3 gap-4">
  <div class="rounded-lg bg-white p-4 shadow"><div class="text-3xl font-bold">{counts.leads}</div><div class="text-sm text-slate-500">Leads</div></div>
  <div class="rounded-lg bg-white p-4 shadow"><div class="text-3xl font-bold">{counts.active}</div><div class="text-sm text-slate-500">Active projects</div></div>
  <div class="rounded-lg bg-white p-4 shadow"><div class="text-3xl font-bold">{counts.open_tasks}</div><div class="text-sm text-slate-500">Open tasks</div></div>
</div>

<div class="grid grid-cols-2 gap-6">
  <section>
    <h2 class="mb-2 font-semibold text-slate-700">Active projects</h2>
    <ul class="space-y-1">
      {#each activeProjects as p}
        <li class="rounded bg-white px-3 py-2 shadow-sm">
          <span class="font-medium">{p.title}</span>
          <span class="text-sm text-slate-500"> · {contactsById[p.contact_id] ?? '—'}</span>
        </li>
      {:else}
        <li class="text-sm text-slate-400">No active projects.</li>
      {/each}
    </ul>
  </section>

  <section>
    <h2 class="mb-2 font-semibold text-slate-700">Tasks due</h2>
    <form onsubmit={(e) => { e.preventDefault(); addTask() }} class="mb-2 flex gap-2">
      <input bind:value={newTask} placeholder="Quick add task…" class="flex-1 rounded border border-slate-300 px-2 py-1 text-sm" />
      <button class="rounded bg-blue-600 px-3 text-sm text-white">Add</button>
    </form>
    <ul class="space-y-1">
      {#each dueTasks as t}
        <li class="flex items-center gap-2 rounded bg-white px-3 py-2 shadow-sm">
          <input type="checkbox" checked={t.status === 'done'} onchange={() => toggleDone(t)} />
          <span class:line-through={t.status === 'done'}>{t.title}</span>
          {#if t.due_on}<span class="ml-auto text-xs text-slate-400">{t.due_on}</span>{/if}
        </li>
      {:else}
        <li class="text-sm text-slate-400">Nothing due.</li>
      {/each}
    </ul>
  </section>
</div>
```

- [ ] **Step 2: Build to verify**

```bash
cd frontend && npm run build
```
Expected: build succeeds.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/Dashboard.svelte
git commit -m "feat(web): dashboard view"
```

---

### Task 6: Contacts view (list + create + edit + delete)

**Files:**
- Create: `frontend/src/lib/components/Modal.svelte`
- Replace: `frontend/src/routes/Contacts.svelte`

**Interfaces:**
- Consumes: `api.get/post/put/del` on `/api/contacts`.
- Produces: a contacts list with a "New contact" button opening a form modal (create), per-row Edit (PUT, full object) and Delete. Fields: kind (person/company), name (required), email, phone, company (optional dropdown of companies).

- [ ] **Step 1: Write `frontend/src/lib/components/Modal.svelte`**

```svelte
<script>
  let { title, onclose, children } = $props()
</script>

<div class="fixed inset-0 z-10 flex items-center justify-center bg-black/30" onclick={onclose}>
  <div class="w-96 rounded-lg bg-white p-5 shadow-xl" onclick={(e) => e.stopPropagation()}>
    <div class="mb-3 flex items-center justify-between">
      <h2 class="font-semibold text-slate-800">{title}</h2>
      <button onclick={onclose} class="text-slate-400 hover:text-slate-700">✕</button>
    </div>
    {@render children()}
  </div>
</div>
```

- [ ] **Step 2: Write `frontend/src/routes/Contacts.svelte`**

```svelte
<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

  let contacts = $state([])
  let error = $state('')
  let editing = $state(null) // null = closed; {} = new; {...} = edit existing

  const companies = $derived(contacts.filter((c) => c.kind === 'company'))

  async function load() {
    error = ''
    try { contacts = await api.get('/api/contacts') } catch (e) { error = e.message }
  }

  function openNew() { editing = { kind: 'person', name: '', email: '', phone: '', company_id: null } }
  function openEdit(c) { editing = { ...c } }

  async function save(e) {
    e.preventDefault()
    const body = {
      kind: editing.kind,
      name: editing.name,
      email: editing.email || null,
      phone: editing.phone || null,
      company_id: editing.company_id || null,
    }
    try {
      if (editing.id) await api.put(`/api/contacts/${editing.id}`, body)
      else await api.post('/api/contacts', body)
      editing = null
      await load()
    } catch (e) { error = e.message }
  }

  async function remove(c) {
    if (!confirm(`Delete ${c.name}?`)) return
    await api.del(`/api/contacts/${c.id}`)
    await load()
  }

  $effect(() => { load() })
</script>

<div class="mb-4 flex items-center justify-between">
  <h1 class="text-2xl font-bold text-slate-800">Contacts</h1>
  <button onclick={openNew} class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white">New contact</button>
</div>
{#if error}<p class="mb-3 text-red-600">{error}</p>{/if}

<table class="w-full bg-white text-sm shadow-sm">
  <thead class="border-b text-left text-slate-500">
    <tr><th class="p-2">Name</th><th class="p-2">Kind</th><th class="p-2">Email</th><th class="p-2">Phone</th><th class="p-2"></th></tr>
  </thead>
  <tbody>
    {#each contacts as c}
      <tr class="border-b last:border-0">
        <td class="p-2 font-medium">{c.name}</td>
        <td class="p-2 text-slate-500">{c.kind}</td>
        <td class="p-2">{c.email ?? ''}</td>
        <td class="p-2">{c.phone ?? ''}</td>
        <td class="p-2 text-right">
          <button onclick={() => openEdit(c)} class="text-blue-600 hover:underline">Edit</button>
          <button onclick={() => remove(c)} class="ml-2 text-red-600 hover:underline">Delete</button>
        </td>
      </tr>
    {:else}
      <tr><td colspan="5" class="p-3 text-slate-400">No contacts yet.</td></tr>
    {/each}
  </tbody>
</table>

{#if editing}
  <Modal title={editing.id ? 'Edit contact' : 'New contact'} onclose={() => (editing = null)}>
    <form onsubmit={save} class="space-y-3">
      <select bind:value={editing.kind} class="w-full rounded border border-slate-300 px-2 py-1.5">
        <option value="person">Person</option>
        <option value="company">Company</option>
      </select>
      <input bind:value={editing.name} placeholder="Name" required class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <input bind:value={editing.email} placeholder="Email" class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <input bind:value={editing.phone} placeholder="Phone" class="w-full rounded border border-slate-300 px-2 py-1.5" />
      {#if editing.kind === 'person'}
        <select bind:value={editing.company_id} class="w-full rounded border border-slate-300 px-2 py-1.5">
          <option value={null}>— No company —</option>
          {#each companies as co}
            {#if co.id !== editing.id}<option value={co.id}>{co.name}</option>{/if}
          {/each}
        </select>
      {/if}
      <button class="w-full rounded bg-blue-600 py-2 text-white">Save</button>
    </form>
  </Modal>
{/if}
```

- [ ] **Step 3: Build to verify**

```bash
cd frontend && npm run build
```
Expected: build succeeds.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/Contacts.svelte frontend/src/lib/components/Modal.svelte
git commit -m "feat(web): contacts list with create/edit/delete"
```

---

### Task 7: Pipeline kanban (drag → move) + new-lead + edit project

**Files:**
- Create: `frontend/src/lib/pipeline.js`, `frontend/src/lib/pipeline.test.js`
- Replace: `frontend/src/routes/Pipeline.svelte`

**Interfaces:**
- Consumes: `api.get('/api/projects')`, `api.get('/api/contacts')`, `api.post('/api/projects')`, `api.put('/api/projects/{id}')`, `api.del`, `api.patch('/api/projects/{id}/move')`.
- Produces: a 5-column board (lead/proposal/active/done/lost). Cards show project title + client name. Dragging a card (within or across columns) calls `/move` with the card's new `status` (target column) and `board_order` (its index in that column). A "New lead" form (client dropdown + title) creates a project (defaults to status `lead`). Clicking a card opens an edit modal (title/description/invoice_url/contact via PUT — preserves status; plus a Delete).

- [ ] **Step 1: Write `frontend/src/lib/pipeline.js` (pure helpers)**

```js
export const STATUSES = ['lead', 'proposal', 'active', 'done', 'lost']
export const STATUS_LABELS = {
  lead: 'Lead', proposal: 'Proposal', active: 'Active', done: 'Done', lost: 'Lost',
}

// Group a flat project list into { status: [projects sorted by board_order] }.
export function groupByStatus(projects) {
  const cols = Object.fromEntries(STATUSES.map((s) => [s, []]))
  for (const p of projects) (cols[p.status] ??= []).push(p)
  for (const s of STATUSES) cols[s].sort((a, b) => a.board_order - b.board_order)
  return cols
}

// After a dnd drop, compute the /move payloads for a column's items:
// each item that changed status or position gets {id, status, board_order:index}.
export function movesForColumn(status, items) {
  return items.map((p, i) => ({ id: p.id, status, board_order: i }))
    .filter((m, i) => items[i].status !== status || items[i].board_order !== i)
}
```

- [ ] **Step 2: Write `frontend/src/lib/pipeline.test.js` (Vitest)**

```js
import { describe, it, expect } from 'vitest'
import { groupByStatus, movesForColumn, STATUSES } from './pipeline.js'

describe('pipeline helpers', () => {
  it('groups and sorts by board_order', () => {
    const cols = groupByStatus([
      { id: 'a', status: 'lead', board_order: 1 },
      { id: 'b', status: 'lead', board_order: 0 },
      { id: 'c', status: 'active', board_order: 0 },
    ])
    expect(cols.lead.map((p) => p.id)).toEqual(['b', 'a'])
    expect(cols.active.map((p) => p.id)).toEqual(['c'])
    expect(STATUSES.every((s) => Array.isArray(cols[s]))).toBe(true)
  })

  it('emits moves only for items whose status or order changed', () => {
    // Item dragged into 'active' at index 0; previously lead@5.
    const moves = movesForColumn('active', [{ id: 'x', status: 'lead', board_order: 5 }])
    expect(moves).toEqual([{ id: 'x', status: 'active', board_order: 0 }])
  })

  it('no moves when nothing changed', () => {
    const moves = movesForColumn('lead', [{ id: 'x', status: 'lead', board_order: 0 }])
    expect(moves).toEqual([])
  })
})
```

- [ ] **Step 3: Run helper tests**

```bash
cd frontend && npm run test
```
Expected: pipeline tests pass (plus the api tests from Task 2).

- [ ] **Step 4: Write `frontend/src/routes/Pipeline.svelte`**

```svelte
<script>
  import { dndzone } from 'svelte-dnd-action'
  import { api } from '../lib/api.js'
  import { STATUSES, STATUS_LABELS, groupByStatus, movesForColumn } from '../lib/pipeline.js'
  import Modal from '../lib/components/Modal.svelte'

  let projects = $state([])
  let contacts = $state([])
  let cols = $state(Object.fromEntries(STATUSES.map((s) => [s, []])))
  let error = $state('')
  let creating = $state(null)  // {contact_id, title} | null
  let editing = $state(null)   // {...project} | null

  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))

  async function load() {
    error = ''
    try {
      ;[projects, contacts] = await Promise.all([api.get('/api/projects'), api.get('/api/contacts')])
      cols = groupByStatus(projects)
    } catch (e) { error = e.message }
  }

  // dnd handlers: status is captured per column via the factory below.
  function dndHandlers(status) {
    return {
      consider: (e) => { cols[status] = e.detail.items; cols = cols },
      finalize: async (e) => {
        cols[status] = e.detail.items; cols = cols
        const moves = movesForColumn(status, e.detail.items)
        try {
          for (const m of moves) await api.patch(`/api/projects/${m.id}/move`, { status: m.status, board_order: m.board_order })
        } catch (err) { error = err.message }
        await load()
      },
    }
  }

  function openNew() { creating = { contact_id: contacts[0]?.id ?? '', title: '' } }
  async function createLead(e) {
    e.preventDefault()
    if (!creating.contact_id || !creating.title.trim()) return
    try {
      await api.post('/api/projects', { contact_id: creating.contact_id, title: creating.title.trim() })
      creating = null
      await load()
    } catch (err) { error = err.message }
  }

  function openEdit(p) { editing = { ...p } }
  async function saveEdit(e) {
    e.preventDefault()
    try {
      // PUT is full-replace but does NOT change status/board_order (server-owned via /move).
      await api.put(`/api/projects/${editing.id}`, {
        contact_id: editing.contact_id,
        title: editing.title,
        description: editing.description || null,
        invoice_url: editing.invoice_url || null,
      })
      editing = null
      await load()
    } catch (err) { error = err.message }
  }
  async function removeProject(p) {
    if (!confirm(`Delete ${p.title}?`)) return
    await api.del(`/api/projects/${p.id}`)
    editing = null
    await load()
  }

  $effect(() => { load() })
</script>

<div class="mb-4 flex items-center justify-between">
  <h1 class="text-2xl font-bold text-slate-800">Pipeline</h1>
  <button onclick={openNew} class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white">New lead</button>
</div>
{#if error}<p class="mb-3 text-red-600">{error}</p>{/if}

<div class="flex gap-3 overflow-x-auto pb-2">
  {#each STATUSES as s}
    <div class="w-56 shrink-0 rounded-lg bg-slate-100 p-2">
      <h2 class="mb-2 px-1 text-sm font-semibold text-slate-600">{STATUS_LABELS[s]} <span class="text-slate-400">({cols[s].length})</span></h2>
      <div
        class="min-h-12 space-y-2"
        use:dndzone={{ items: cols[s], flipDurationMs: 150 }}
        onconsider={dndHandlers(s).consider}
        onfinalize={dndHandlers(s).finalize}
      >
        {#each cols[s] as p (p.id)}
          <button onclick={() => openEdit(p)} class="block w-full rounded bg-white p-2 text-left shadow-sm hover:shadow">
            <div class="font-medium text-slate-800">{p.title}</div>
            <div class="text-xs text-slate-500">{contactsById[p.contact_id] ?? '—'}</div>
            {#if p.invoice_url}<div class="mt-1 text-xs text-blue-600">invoice ↗</div>{/if}
          </button>
        {/each}
      </div>
    </div>
  {/each}
</div>

{#if creating}
  <Modal title="New lead" onclose={() => (creating = null)}>
    <form onsubmit={createLead} class="space-y-3">
      <select bind:value={creating.contact_id} required class="w-full rounded border border-slate-300 px-2 py-1.5">
        {#each contacts as c}<option value={c.id}>{c.name}</option>{/each}
      </select>
      <input bind:value={creating.title} placeholder="Project title" required class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <button class="w-full rounded bg-blue-600 py-2 text-white">Create lead</button>
    </form>
  </Modal>
{/if}

{#if editing}
  <Modal title="Edit project" onclose={() => (editing = null)}>
    <form onsubmit={saveEdit} class="space-y-3">
      <input bind:value={editing.title} placeholder="Title" required class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <select bind:value={editing.contact_id} class="w-full rounded border border-slate-300 px-2 py-1.5">
        {#each contacts as c}<option value={c.id}>{c.name}</option>{/each}
      </select>
      <textarea bind:value={editing.description} placeholder="Description" class="w-full rounded border border-slate-300 px-2 py-1.5"></textarea>
      <input bind:value={editing.invoice_url} placeholder="Indy invoice URL" class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <p class="text-xs text-slate-400">Status: {editing.status} (change by dragging on the board)</p>
      <div class="flex gap-2">
        <button class="flex-1 rounded bg-blue-600 py-2 text-white">Save</button>
        <button type="button" onclick={() => removeProject(editing)} class="rounded bg-red-100 px-3 py-2 text-red-700">Delete</button>
      </div>
    </form>
  </Modal>
{/if}
```

Note (`pipeline.js:movesForColumn`): `ponytail:` we PATCH each changed card one-by-one; fine for a single user's handful of cards. If a board ever holds hundreds, add a batch move endpoint.

- [ ] **Step 5: Build to verify**

```bash
cd frontend && npm run build
```
Expected: build succeeds.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/routes/Pipeline.svelte frontend/src/lib/pipeline.js frontend/src/lib/pipeline.test.js
git commit -m "feat(web): pipeline kanban with drag-to-move, new lead, edit project"
```

---

### Task 8: Dockerfile — Node build stage for the frontend

**Files:**
- Modify: `Dockerfile`, `.dockerignore`

**Interfaces:**
- Produces: a Docker image whose runtime contains both the compiled Rust binary AND the built SPA in `static/`, so a single container serves the API and the UI.

- [ ] **Step 1: Update `.dockerignore`**

Ensure it contains (append if missing):

```
frontend/node_modules
static
```

(`static/` is git-ignored anyway; excluding it from the build context avoids shipping any stale local build — the image rebuilds it.)

- [ ] **Step 2: Rewrite `Dockerfile` with a frontend stage**

```dockerfile
# ---- frontend build ----
FROM node:22-bookworm-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install --no-fund --no-audit
COPY frontend/ ./
# vite.config.js emits to ../static (i.e. /app/static)
RUN npm run build

# ---- rust build ----
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release ; rm -rf src
COPY . .
RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/octopus /usr/local/bin/octopus
COPY --from=build /app/migrations ./migrations
COPY --from=frontend /app/static ./static
ENV PORT=8080
EXPOSE 8080
CMD ["octopus"]
```

Key change: `static/` now comes from the `frontend` stage (the built SPA), not from the repo. The rust `build` stage's `COPY . .` brings in source but not `static/` (git-ignored / dockerignored), which is correct — the runtime takes `static/` from the frontend stage.

- [ ] **Step 3: Build the image to verify (compiles frontend + backend)**

```bash
docker build -t octopus .
```
Expected: build succeeds through all three stages; image `octopus` created. (Takes several minutes.)

- [ ] **Step 4: Commit**

```bash
git add Dockerfile .dockerignore
git commit -m "build(web): add node frontend stage; image serves API + SPA"
```

---

### Task 9: End-to-end manual verification

**Files:** none (verification task).

**Interfaces:** confirms the full stack works against a real Postgres + browser.

- [ ] **Step 1: Run the API**

```bash
export DATABASE_URL=postgres://octopus:octopus@localhost:5432/octopus
export SESSION_SECRET=$(openssl rand -hex 48)
export APP_PASSWORD=dev-password
export PORT=8090
cargo run
```
(Use the local `octopus` role/db from the README. Keep this running.)

- [ ] **Step 2: Run the frontend dev server**

```bash
cd frontend && npm run dev
```
Open http://localhost:5173.

- [ ] **Step 3: Walk the flow and confirm each**

- [ ] Visiting any page while logged out redirects to the login screen.
- [ ] Wrong password shows "Wrong password."; correct password (`dev-password`) lands on the dashboard.
- [ ] Dashboard shows three count cards (0s initially).
- [ ] Contacts → New contact → create a company and a person; both appear; edit one (rename) and confirm it persists; the person's company dropdown lists the company.
- [ ] Pipeline → New lead → pick the contact, give a title → a card appears in the **Lead** column; the counts on the dashboard update (leads +1).
- [ ] Drag the card from Lead to Active → it stays in Active after the list refreshes (the `/move` persisted); dashboard "Active projects" now lists it.
- [ ] Click a card → edit title + add an Indy invoice URL → save; the "invoice ↗" marker appears; the card's status is unchanged by the edit.
- [ ] Dashboard quick-add a task; tick its checkbox → it shows done (strike-through) and open-task count drops.
- [ ] Logout returns to the login screen; reloading stays logged out.

- [ ] **Step 4: Confirm the production build path**

```bash
cd frontend && npm run build      # emits into ../static
# stop vite; with the API still running on 8090, browse http://localhost:8090
```
- [ ] The SPA loads and works when served by Axum itself (not just via vite dev) — confirms the `static/` serving path.

- [ ] **Step 5: Record the result**

Write a short note of what passed / any issues into the task report. No commit (no file changes), unless a fix was needed (then commit the fix).

---

## Self-Review

**Spec coverage (1B scope):**
- Svelte SPA built into `static/`, served by Axum → Tasks 1, 8. ✓
- API client sending the cookie, 401 handling → Task 2. ✓
- Login + auth gate → Tasks 3, 4. ✓
- Dashboard (counts + active projects + due tasks + quick task) → Task 5. ✓
- Contacts CRUD UI → Task 6. ✓
- Pipeline kanban with drag→`/move`, new lead, edit project (PUT preserves status), invoice_url → Task 7. ✓
- Tailwind styling throughout; svelte-spa-router; svelte-dnd-action → all tasks. ✓
- Dockerfile builds + ships the SPA → Task 8. ✓
- End-to-end verification → Task 9. ✓
- Calendar/notes deliberately deferred to Phase 2/3 (out of 1B scope). ✓
- Deeper per-project task drawer / contact-detail page deferred to a possible 1B.2 — the core entities are all reachable/editable.

**Placeholder scan:** No "TBD"/vague steps. Route-component stubs in Task 4 are explicit and replaced in Tasks 5–7. The only client-side-filter shortcut (projects-by-contact via fetch-all) and the per-card move PATCH are marked with `ponytail:` notes.

**Type/contract consistency:** All API paths, payload field names, and status enums match the Phase 1A endpoints in Global Constraints. `session.svelte.js` (runes module) is imported consistently (not `session.js`). `mount()` (Svelte 5), `onconsider/onfinalize` (dnd-action), and `use:link`/`push` (svelte-spa-router) match the verified library APIs. The project edit PUT omits `status`/`board_order` deliberately (server preserves them), consistent with the documented PUT contract.

**Risk note:** `static/` is git-ignored and produced by the build — a fresh `cargo run` without first running `npm run build` (or `vite dev`) serves no UI (API still works). Documented in README (Task 1) and the verify task (Task 9).
