# Contacts Relationship Hub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the Contacts feature into a modern relationship hub (searchable card grid + per-contact timeline of notes & events) while keeping the existing terminal aesthetic, with no backend or schema changes.

**Architecture:** All new logic is pure JS in a new `frontend/src/lib/contacts.js` helper module (unit-tested with Vitest), consumed by two rewritten Svelte routes and one extracted shared form component. Data is aggregated client-side from existing endpoints (`/api/contacts`, `/api/projects`, `/api/notes?contact_id=`, `/api/events`) — matching the codebase's existing `ponytail:` client-filter pattern.

**Tech Stack:** Svelte 5 (runes: `$state`/`$derived`/`$effect`/`$props`), `svelte-spa-router` (hash routing, `push`), Tailwind v4 (theme tokens in `frontend/src/app.css`), Vite, Vitest.

## Global Constraints

- **No backend/schema changes.** Do not touch `src/`, `migrations/`, or any `.rs` file. Frontend only.
- **No new dependencies.** Use what's installed.
- **Design tokens only** (from `app.css`): `bg`, `surface`, `surface-2`, `border`, `border-2`, `ink`, `muted`, `faint`, `accent`, `accent-dim`, `on-accent`, `st-lead`, `st-active`, `st-lost`. Font is `font-mono` everywhere. Utilities: `glow-soft`, `glow-text`, `.rise`, `.label`.
- **Kind colors:** `company` → `text-accent` (mint); `person` → `text-st-lead` (blue). Badges render as `[ {kind} ]`, uppercase via existing classes.
- **API:** use the `api` client from `frontend/src/lib/api.js` (`api.get/post/put/del`). It throws `ApiError` with `.message`; `del` returns `null` on 204.
- **Contact create/update body shape** (unchanged): `{ kind, name, email|null, phone|null, company_id|null }`.
- **Test command:** `cd frontend && npm test` (Vitest, `vitest run`). Build check: `cd frontend && npm run build`.
- **Commit style:** conventional commits, e.g. `feat(contacts): …`. End messages with the `Co-Authored-By` trailer only if that's the repo norm (recent commits do not include one — match them: no trailer).

---

### Task 1: `contacts.js` pure helpers + tests

The testable core: filtering, project counts, timeline assembly, last-touch, roster/company resolution. Mirrors the existing `frontend/src/lib/links.js` + `links.test.js` pattern.

**Files:**
- Create: `frontend/src/lib/contacts.js`
- Test: `frontend/src/lib/contacts.test.js`

**Interfaces:**
- Produces (consumed by Tasks 3 & 4):
  - `filterContacts(contacts, query, kind) -> Contact[]` — `kind` ∈ `'all'|'person'|'company'`; matches `name`/`email`/resolved company name (case-insensitive).
  - `projectCountsByContact(projects) -> Map<contactId, {active, total}>`.
  - `eventsForContact(events, contactId) -> Event[]` — events whose `contact_ids` includes `contactId`.
  - `buildTimeline(notes, events, now) -> { upcoming: Item[], history: Item[] }` where `Item = { type:'note'|'event', id, when:ISOString, text, all_day? }`. `upcoming` = events with `starts_at > now`, ascending. `history` = notes + past events, descending.
  - `lastTouch(notes, events, now) -> ISOString|null` — most recent note `created_at` or event `starts_at` that is `<= now`.
  - `humanizeSince(dateStr, now) -> string` — `null`→`'—'`, same-or-future-day→`'today'`, else `'{n}d ago'`.
  - `companyRoster(contacts, companyId) -> Contact[]` — contacts whose `company_id === companyId`.
  - `companyName(contacts, companyId) -> string|null`.

- [ ] **Step 1: Write the failing test**

Create `frontend/src/lib/contacts.test.js`:

```js
import { describe, it, expect } from 'vitest'
import {
  filterContacts, projectCountsByContact, eventsForContact,
  buildTimeline, lastTouch, humanizeSince, companyRoster, companyName,
} from './contacts.js'

const NOW = '2026-07-02T12:00:00Z'

const contacts = [
  { id: 'co1', kind: 'company', name: 'Acme Corp', email: 'ops@acme.io', phone: null, company_id: null },
  { id: 'p1', kind: 'person', name: 'Jane Doe', email: 'jane@acme.io', phone: '555', company_id: 'co1' },
  { id: 'p2', kind: 'person', name: 'Bob Smith', email: 'bob@x.io', phone: null, company_id: null },
]

describe('filterContacts', () => {
  it('filters by kind', () => {
    expect(filterContacts(contacts, '', 'company').map((c) => c.id)).toEqual(['co1'])
    expect(filterContacts(contacts, '', 'person').map((c) => c.id)).toEqual(['p1', 'p2'])
    expect(filterContacts(contacts, '', 'all')).toHaveLength(3)
  })
  it('matches name, email, and resolved company name', () => {
    expect(filterContacts(contacts, 'jane', 'all').map((c) => c.id)).toEqual(['p1'])
    expect(filterContacts(contacts, 'bob@x', 'all').map((c) => c.id)).toEqual(['p2'])
    // Jane belongs to Acme → matches on company name even though her name doesn't contain "acme"
    expect(filterContacts(contacts, 'acme', 'all').map((c) => c.id)).toEqual(['co1', 'p1'])
  })
})

describe('projectCountsByContact', () => {
  it('counts active and total per contact, skips null contact_id', () => {
    const projects = [
      { id: 'a', contact_id: 'co1', status: 'active' },
      { id: 'b', contact_id: 'co1', status: 'archived' },
      { id: 'c', contact_id: 'p1', status: 'active' },
      { id: 'd', contact_id: null, status: 'active' },
    ]
    const m = projectCountsByContact(projects)
    expect(m.get('co1')).toEqual({ active: 1, total: 2 })
    expect(m.get('p1')).toEqual({ active: 1, total: 1 })
    expect(m.has('d')).toBe(false)
  })
})

describe('eventsForContact', () => {
  it('keeps events whose contact_ids includes the id', () => {
    const events = [
      { id: 'e1', contact_ids: ['p1', 'co1'] },
      { id: 'e2', contact_ids: ['p2'] },
      { id: 'e3', contact_ids: [] },
    ]
    expect(eventsForContact(events, 'p1').map((e) => e.id)).toEqual(['e1'])
  })
})

describe('buildTimeline', () => {
  const notes = [
    { id: 'n1', body: 'quote sent', created_at: '2026-07-01T09:00:00Z' },
    { id: 'n2', body: 'intro', created_at: '2026-06-25T09:00:00Z' },
  ]
  const events = [
    { id: 'e1', title: 'Kickoff', starts_at: '2026-07-04T10:00:00Z', all_day: false },
    { id: 'e2', title: 'Discovery', starts_at: '2026-06-28T10:00:00Z', all_day: false },
  ]
  it('splits upcoming (asc) from history (desc)', () => {
    const { upcoming, history } = buildTimeline(notes, events, NOW)
    expect(upcoming.map((i) => i.id)).toEqual(['e1'])
    expect(upcoming[0]).toMatchObject({ type: 'event', text: 'Kickoff' })
    // history: n1 (07-01) > e2 (06-28) > n2 (06-25)
    expect(history.map((i) => i.id)).toEqual(['n1', 'e2', 'n2'])
    expect(history[0]).toMatchObject({ type: 'note', text: 'quote sent' })
  })
  it('handles empty inputs', () => {
    expect(buildTimeline([], [], NOW)).toEqual({ upcoming: [], history: [] })
  })
})

describe('lastTouch', () => {
  it('returns most recent past date, ignoring future', () => {
    const notes = [{ id: 'n', body: 'x', created_at: '2026-06-30T00:00:00Z' }]
    const events = [
      { id: 'e', title: 'y', starts_at: '2026-07-01T00:00:00Z' },
      { id: 'f', title: 'z', starts_at: '2026-08-01T00:00:00Z' }, // future — ignored
    ]
    expect(lastTouch(notes, events, NOW)).toBe('2026-07-01T00:00:00Z')
  })
  it('returns null when nothing in the past', () => {
    expect(lastTouch([], [], NOW)).toBe(null)
  })
})

describe('humanizeSince', () => {
  it('formats relative days', () => {
    expect(humanizeSince(null, NOW)).toBe('—')
    expect(humanizeSince('2026-07-02T06:00:00Z', NOW)).toBe('today')
    expect(humanizeSince('2026-06-29T12:00:00Z', NOW)).toBe('3d ago')
  })
})

describe('companyRoster / companyName', () => {
  it('lists people of a company', () => {
    expect(companyRoster(contacts, 'co1').map((c) => c.id)).toEqual(['p1'])
  })
  it('resolves company name or null', () => {
    expect(companyName(contacts, 'co1')).toBe('Acme Corp')
    expect(companyName(contacts, 'nope')).toBe(null)
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd frontend && npx vitest run src/lib/contacts.test.js`
Expected: FAIL — `Failed to resolve import "./contacts.js"` / functions undefined.

- [ ] **Step 3: Write the implementation**

Create `frontend/src/lib/contacts.js`:

```js
// Pure helpers for the Contacts feature. No I/O — all aggregation client-side.
const ms = (d) => new Date(d).getTime()

export function filterContacts(contacts, query, kind) {
  const q = query.trim().toLowerCase()
  const nameById = new Map(contacts.map((c) => [c.id, c.name]))
  return contacts.filter((c) => {
    if (kind !== 'all' && c.kind !== kind) return false
    if (!q) return true
    const company = c.company_id ? nameById.get(c.company_id) || '' : ''
    return (
      c.name.toLowerCase().includes(q) ||
      (c.email || '').toLowerCase().includes(q) ||
      company.toLowerCase().includes(q)
    )
  })
}

export function projectCountsByContact(projects) {
  const m = new Map()
  for (const p of projects) {
    if (!p.contact_id) continue
    const c = m.get(p.contact_id) || { active: 0, total: 0 }
    c.total += 1
    if (p.status === 'active') c.active += 1
    m.set(p.contact_id, c)
  }
  return m
}

export function eventsForContact(events, contactId) {
  return events.filter((e) => (e.contact_ids || []).includes(contactId))
}

export function buildTimeline(notes, events, now) {
  const nowMs = ms(now)
  const noteItems = notes.map((n) => ({ type: 'note', id: n.id, when: n.created_at, text: n.body }))
  const eventItems = events.map((e) => ({ type: 'event', id: e.id, when: e.starts_at, text: e.title, all_day: e.all_day }))
  const upcoming = eventItems
    .filter((e) => ms(e.when) > nowMs)
    .sort((a, b) => ms(a.when) - ms(b.when))
  const past = eventItems.filter((e) => ms(e.when) <= nowMs)
  const history = [...noteItems, ...past].sort((a, b) => ms(b.when) - ms(a.when))
  return { upcoming, history }
}

export function lastTouch(notes, events, now) {
  const nowMs = ms(now)
  const dates = [
    ...notes.map((n) => n.created_at),
    ...events.map((e) => e.starts_at),
  ].filter((d) => ms(d) <= nowMs)
  if (!dates.length) return null
  return dates.reduce((a, b) => (ms(a) > ms(b) ? a : b))
}

export function humanizeSince(dateStr, now) {
  if (!dateStr) return '—'
  const days = Math.floor((ms(now) - ms(dateStr)) / 86400000)
  if (days <= 0) return 'today'
  return `${days}d ago`
}

export function companyRoster(contacts, companyId) {
  return contacts.filter((c) => c.company_id === companyId)
}

export function companyName(contacts, companyId) {
  return contacts.find((c) => c.id === companyId)?.name ?? null
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd frontend && npx vitest run src/lib/contacts.test.js`
Expected: PASS — all describe blocks green.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/contacts.js frontend/src/lib/contacts.test.js
git commit -m "feat(contacts): pure helpers for filtering, timeline, roster + tests"
```

---

### Task 2: Extract shared `ContactForm` component

Both the list and detail pages need the create/edit form. Extract it once (DRY) — the form currently lives inline in `Contacts.svelte`. The component wraps the existing `Modal` and renders the same fields; the parent owns state and persistence.

**Files:**
- Create: `frontend/src/lib/components/ContactForm.svelte`

**Interfaces:**
- Produces (consumed by Tasks 3 & 4):
  - `<ContactForm value companies onclose onsubmit />`
    - `value`: the editing object `{ id?, kind, name, email, phone, company_id }` (bound-into by the form fields).
    - `companies`: array of `kind === 'company'` contacts (for the person→company select).
    - `onclose`: `() => void` — close handler (Escape/backdrop/✕).
    - `onsubmit`: `(e) => void` — form submit handler; parent calls `e.preventDefault()` and persists.

- [ ] **Step 1: Create the component**

Create `frontend/src/lib/components/ContactForm.svelte`:

```svelte
<script>
  import Modal from './Modal.svelte'
  let { value, companies, onclose, onsubmit } = $props()
  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
</script>

<Modal title={value.id ? 'Edit contact' : 'New contact'} {onclose}>
  <form onsubmit={onsubmit} class="flex flex-col gap-3">
    <div>
      <p class="label mb-1.5">Kind</p>
      <select bind:value={value.kind} class={FIELD}>
        <option value="person">Person</option>
        <option value="company">Company</option>
      </select>
    </div>
    <div>
      <p class="label mb-1.5">Name</p>
      <input bind:value={value.name} placeholder="Name" required class={FIELD} />
    </div>
    <div>
      <p class="label mb-1.5">Email</p>
      <input bind:value={value.email} placeholder="Email" class={FIELD} />
    </div>
    <div>
      <p class="label mb-1.5">Phone</p>
      <input bind:value={value.phone} placeholder="Phone" class={FIELD} />
    </div>
    {#if value.kind === 'person'}
      <div>
        <p class="label mb-1.5">Company</p>
        <select bind:value={value.company_id} class={FIELD}>
          <option value={null}>— No company —</option>
          {#each companies as co}
            {#if co.id !== value.id}<option value={co.id}>{co.name}</option>{/if}
          {/each}
        </select>
      </div>
    {/if}
    <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
  </form>
</Modal>
```

- [ ] **Step 2: Verify it builds**

Run: `cd frontend && npm run build`
Expected: build succeeds (component compiles; not yet imported anywhere).

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/ContactForm.svelte
git commit -m "refactor(contacts): extract ContactForm modal for reuse across list + detail"
```

---

### Task 3: Rewrite `Contacts.svelte` as a searchable card grid

Replaces the flat table with a command bar (search + kind toggle) and a responsive card grid. Cards show kind badge, name, company line, email/phone, project-count stat, and inline quick actions (mailto / tel / copy). Edit/delete reveal on hover. Uses the Task 1 helpers and Task 2 form.

**Files:**
- Modify (full rewrite): `frontend/src/routes/Contacts.svelte`

**Interfaces:**
- Consumes: `filterContacts`, `projectCountsByContact` (Task 1); `ContactForm` (Task 2); `api` (`api.get/post/put/del`).

- [ ] **Step 1: Replace the file contents**

Overwrite `frontend/src/routes/Contacts.svelte`:

```svelte
<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import ContactForm from '../lib/components/ContactForm.svelte'
  import { filterContacts, projectCountsByContact } from '../lib/contacts.js'

  const KINDS = ['all', 'person', 'company']

  let contacts = $state([])
  let projects = $state([])
  let error = $state('')
  let editing = $state(null) // null=closed; {}=new; {...}=edit
  let query = $state('')
  let kind = $state('all')

  const companies = $derived(contacts.filter((c) => c.kind === 'company'))
  const nameById = $derived(new Map(contacts.map((c) => [c.id, c.name])))
  const counts = $derived(projectCountsByContact(projects))
  const shown = $derived(filterContacts(contacts, query, kind))

  async function load() {
    error = ''
    try {
      const [cs, ps] = await Promise.all([api.get('/api/contacts'), api.get('/api/projects')])
      contacts = cs
      projects = ps
    } catch (e) { error = e.message }
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
    if (!confirm(`Delete ${c.name}? This also deletes their projects and tasks.`)) return
    try { await api.del(`/api/contacts/${c.id}`); await load() } catch (e) { error = e.message }
  }

  function copy(text) { navigator.clipboard?.writeText(text) }

  $effect(() => { load() })
</script>

<div class="rise mb-5 flex items-center justify-between gap-3">
  <p class="font-mono text-[12px] text-faint"><span class="text-accent-dim">//</span> manage clients and contacts</p>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  ><span class="text-[15px] leading-none">+</span> New contact</button>
</div>

<!-- Command bar: search + kind toggle -->
<div class="rise mb-5 flex flex-wrap items-center gap-3" style="animation-delay:30ms">
  <div class="relative min-w-[220px] flex-1">
    <span class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 font-mono text-[13px] text-accent">&gt;</span>
    <input
      bind:value={query}
      placeholder="search name, email, company…"
      class="w-full rounded-sm border border-border bg-surface-2 py-2 pl-7 pr-2.5 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none"
    />
  </div>
  <div class="flex rounded-sm border border-border">
    {#each KINDS as k}
      <button
        onclick={() => (kind = k)}
        class="h-8 px-3 font-mono text-[12px] transition-colors {kind === k ? 'bg-surface-2 text-accent' : 'text-faint hover:text-ink'}"
      >{k}</button>
    {/each}
  </div>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

{#if shown.length === 0}
  <p class="rise font-mono text-[12px] text-faint" style="animation-delay:60ms">no contacts{query || kind !== 'all' ? ' match' : ' yet'}</p>
{:else}
  <div class="rise grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3" style="animation-delay:60ms">
    {#each shown as c (c.id)}
      {@const ct = counts.get(c.id) ?? { active: 0, total: 0 }}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        onclick={() => push('/contacts/' + c.id)}
        class="group relative cursor-pointer rounded-sm border border-border bg-surface p-4 transition hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
      >
        <div class="absolute right-2 top-2 flex gap-1 opacity-0 transition group-hover:opacity-100">
          <button onclick={(e) => { e.stopPropagation(); openEdit(c) }} class="h-6 rounded-sm border border-border-2 px-2 font-mono text-[11px] text-muted transition hover:border-accent-dim hover:text-ink">edit</button>
          <button onclick={(e) => { e.stopPropagation(); remove(c) }} class="h-6 rounded-sm border border-st-lost/40 px-2 font-mono text-[11px] text-st-lost transition hover:bg-st-lost/10">del</button>
        </div>

        <span class="font-mono text-[11px] font-bold uppercase tracking-wider {c.kind === 'company' ? 'text-accent' : 'text-st-lead'}">[ {c.kind} ]</span>
        <p class="mt-1.5 font-mono text-[14px] font-medium text-ink">{c.name}</p>
        {#if c.kind === 'person'}
          <p class="mt-0.5 font-mono text-[12px] text-faint">⌂ {c.company_id ? (nameById.get(c.company_id) ?? '—') : '—'}</p>
        {/if}

        <div class="mt-2.5 flex flex-col gap-0.5 font-mono text-[12px]">
          <span class="text-muted">{c.email ?? '—'}</span>
          <span class="text-faint">{c.phone ?? '—'}</span>
        </div>

        <div class="mt-3 flex items-center justify-between border-t border-border/60 pt-2.5">
          <span class="font-mono text-[11px] text-faint">◈ {ct.active} active · {ct.total} total</span>
          <div class="flex items-center gap-1.5">
            {#if c.email}
              <a href={'mailto:' + c.email} onclick={(e) => e.stopPropagation()} aria-label="Email" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">✉</a>
            {/if}
            {#if c.phone}
              <a href={'tel:' + c.phone} onclick={(e) => e.stopPropagation()} aria-label="Call" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">☏</a>
            {/if}
            {#if c.email}
              <button onclick={(e) => { e.stopPropagation(); copy(c.email) }} aria-label="Copy email" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">⧉</button>
            {/if}
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

{#if editing}
  <ContactForm value={editing} {companies} onclose={() => (editing = null)} onsubmit={save} />
{/if}
```

- [ ] **Step 2: Verify it builds and existing tests pass**

Run: `cd frontend && npm run build && npm test`
Expected: build succeeds; Vitest suite (incl. `contacts.test.js`) passes.

- [ ] **Step 3: Manual smoke check**

Run: `cd frontend && npm run dev`, log in, open `/#/contacts`. Verify:
- Cards render 3-up on desktop; search filters live; `all/people/company` toggle filters by kind.
- Hover reveals edit/del; `+ New contact` and `edit` open the form; save persists.
- `✉`/`☏`/`⧉` do not navigate into the card (they stop propagation); clicking the card body opens detail.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/Contacts.svelte
git commit -m "feat(contacts): searchable card grid with quick actions + project counts"
```

---

### Task 4: Rewrite `ContactDetail.svelte` as the relationship hub

Adds quick-action header, derived stat strip, restyled projects panel, company-only roster, and the merged notes+events timeline with an inline note composer (⌘⏎ to submit). Uses Task 1 helpers and Task 2 form.

**Files:**
- Modify (full rewrite): `frontend/src/routes/ContactDetail.svelte`

**Interfaces:**
- Consumes: `eventsForContact`, `buildTimeline`, `lastTouch`, `humanizeSince`, `companyRoster`, `companyName` (Task 1); `ContactForm` (Task 2); `api`.
- Notes:
  - `↗` on events navigates to `/calendar` only — the Calendar route has no day-param deep-link. Marked `ponytail:`.
  - `log ▸ event`/`project` navigate to `/calendar`/`/projects` with **no** contact prefill. Marked `ponytail:`.

- [ ] **Step 1: Replace the file contents**

Overwrite `frontend/src/routes/ContactDetail.svelte`:

```svelte
<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import ContactForm from '../lib/components/ContactForm.svelte'
  import { eventsForContact, buildTimeline, lastTouch, humanizeSince, companyRoster, companyName } from '../lib/contacts.js'

  const PROJECT_STATUS_TEXT = { active: 'text-st-active', archived: 'text-muted' }

  let { params } = $props()
  const id = $derived(params.id)

  let contact = $state(null)
  let contacts = $state([])
  let projects = $state([])
  let notes = $state([])
  let events = $state([])
  let error = $state('')
  let newNote = $state('')
  let noteBusy = $state(false)
  let editing = $state(null)
  let composer // textarea ref (bind:this)

  const myProjects = $derived(projects.filter((p) => p.contact_id === id))
  const myEvents = $derived(eventsForContact(events, id))
  const timeline = $derived(buildTimeline(notes, myEvents, new Date()))
  const activeCount = $derived(myProjects.filter((p) => p.status === 'active').length)
  const touch = $derived(humanizeSince(lastTouch(notes, myEvents, new Date()), new Date()))
  const roster = $derived(contact?.kind === 'company' ? companyRoster(contacts, id) : [])
  const company = $derived(contact?.company_id ? companyName(contacts, contact.company_id) : null)
  const companies = $derived(contacts.filter((c) => c.kind === 'company'))

  async function load() {
    error = ''
    try {
      // ponytail: no ?contact_id filters on /api/projects or /api/events — filter client-side.
      const [c, cs, ps, ns, es] = await Promise.all([
        api.get('/api/contacts/' + id),
        api.get('/api/contacts'),
        api.get('/api/projects'),
        api.get('/api/notes?contact_id=' + id),
        api.get('/api/events'),
      ])
      contact = c; contacts = cs; projects = ps; notes = ns; events = es
    } catch (e) { error = e.message }
  }

  $effect(() => { if (id) load() })

  function copy(text) { navigator.clipboard?.writeText(text) }

  function openEdit() { editing = { ...contact } }
  async function saveEdit(e) {
    e.preventDefault()
    const body = {
      kind: editing.kind,
      name: editing.name,
      email: editing.email || null,
      phone: editing.phone || null,
      company_id: editing.company_id || null,
    }
    try { await api.put('/api/contacts/' + id, body); editing = null; await load() }
    catch (err) { error = err.message }
  }

  async function deleteContact() {
    if (!confirm(`Delete ${contact?.name ?? 'this contact'}? This also deletes their projects and tasks.`)) return
    try { await api.del('/api/contacts/' + id); push('/contacts') } catch (e) { error = e.message }
  }

  async function addNote(e) {
    e?.preventDefault()
    const b = newNote.trim()
    if (!b) return
    noteBusy = true
    try { await api.post('/api/notes', { body: b, contact_id: id }); newNote = ''; await load() }
    catch (err) { error = err.message } finally { noteBusy = false }
  }

  function composerKey(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') { e.preventDefault(); addNote() }
  }

  async function deleteNote(noteId) {
    try { await api.del('/api/notes/' + noteId); await load() } catch (e) { error = e.message }
  }

  const fmtDay = (iso) => new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  const fmtWhen = (iso) => new Date(iso).toLocaleString(undefined, { weekday: 'short', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="mb-2 flex flex-wrap items-center gap-3">
    <button onclick={() => push('/contacts')} class="font-mono text-[12px] text-faint transition hover:text-accent">&lt; contacts</button>
    <span class="font-mono text-[12px] text-faint">/</span>
    <h2 class="font-mono text-[15px] font-bold text-ink">{contact?.name ?? '…'}</h2>
    {#if contact}
      <span class="font-mono text-[11px] font-bold uppercase tracking-wider {contact.kind === 'company' ? 'text-accent' : 'text-st-lead'}">[ {contact.kind} ]</span>
    {/if}
    <div class="ml-auto flex flex-wrap gap-1.5">
      {#if contact?.email}
        <a href={'mailto:' + contact.email} class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">✉ email</a>
      {/if}
      {#if contact?.phone}
        <a href={'tel:' + contact.phone} class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">☏ call</a>
      {/if}
      <button onclick={openEdit} class="h-8 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink">edit</button>
      <button onclick={deleteContact} class="h-8 rounded-sm border border-st-lost/40 px-2.5 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10">delete</button>
    </div>
  </div>

  {#if contact}
    <p class="font-mono text-[12px]">
      {#if contact.kind === 'person'}
        {#if company}
          <button onclick={() => push('/contacts/' + contact.company_id)} class="text-muted transition hover:text-accent">⌂ {company}</button>
        {:else}<span class="text-faint">⌂ —</span>{/if}
        <span class="mx-1.5 text-border-2">·</span>
      {/if}
      <span class="text-muted">{contact.email ?? '—'}</span>
      <span class="mx-1.5 text-border-2">·</span>
      <span class="text-faint">{contact.phone ?? '—'}</span>
    </p>
  {/if}

  {#if error}
    <p class="mt-3 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Stat strip -->
{#if contact}
  <div class="rise mb-8 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-sm border border-border bg-surface px-4 py-2.5 font-mono text-[12px]" style="animation-delay:30ms">
    <span class="text-accent">◈</span>
    <span class="text-ink">{activeCount}<span class="text-faint"> active</span></span>
    <span class="text-border-2">·</span>
    <span class="text-ink">{myProjects.length}<span class="text-faint"> projects</span></span>
    <span class="text-border-2">·</span>
    <span class="text-ink">{notes.length}<span class="text-faint"> notes</span></span>
    <span class="text-border-2">·</span>
    <span class="text-faint">last touch {touch}</span>
  </div>
{/if}

<div class="grid gap-8 lg:grid-cols-[1fr_260px]">
  <!-- Timeline -->
  <div class="rise order-2 lg:order-1" style="animation-delay:80ms">
    <div class="mb-3 flex items-center justify-between gap-2">
      <p class="font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> timeline</p>
      <div class="flex items-center gap-2 font-mono text-[11px]">
        <span class="text-faint">log ▸</span>
        <button onclick={() => composer?.focus()} class="text-muted transition hover:text-accent">note</button>
        <!-- ponytail: navigate-only, no contact prefill -->
        <button onclick={() => push('/calendar')} class="text-muted transition hover:text-accent">event</button>
        <button onclick={() => push('/projects')} class="text-muted transition hover:text-accent">project</button>
      </div>
    </div>

    <form onsubmit={addNote} class="mb-4 flex flex-col gap-2">
      <textarea
        bind:this={composer}
        bind:value={newNote}
        onkeydown={composerKey}
        placeholder="log a note…  (⌘⏎ to add)"
        rows="2"
        class="w-full resize-none rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none"
      ></textarea>
      <button type="submit" disabled={noteBusy} class="h-8 self-start rounded-sm bg-accent px-3 font-mono text-[12px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50">add note</button>
    </form>

    {#if timeline.upcoming.length === 0 && timeline.history.length === 0}
      <p class="font-mono text-[12px] text-faint">· start of history ·</p>
    {:else}
      <div class="flex flex-col gap-2">
        {#if timeline.upcoming.length}
          <p class="label mt-1 text-accent">⧗ upcoming</p>
          {#each timeline.upcoming as it (it.id)}
            <div class="flex items-start gap-3 rounded-sm border border-accent-dim/40 bg-surface px-3 py-2">
              <span class="mt-0.5 text-[12px] text-accent">◆</span>
              <div class="min-w-0 flex-1">
                <p class="font-mono text-[13px] text-ink">{it.text}</p>
                <p class="mt-0.5 font-mono text-[11px] text-faint">{fmtWhen(it.when)}</p>
              </div>
              <button onclick={() => push('/calendar')} aria-label="Open calendar" class="shrink-0 font-mono text-[13px] text-faint transition hover:text-accent">↗</button>
            </div>
          {/each}
          <div class="my-1 border-t border-border/60"></div>
        {/if}

        {#each timeline.history as it (it.id)}
          <div class="group flex items-start gap-3 rounded-sm border border-border bg-surface px-3 py-2">
            <span class="mt-0.5 text-[12px] {it.type === 'event' ? 'text-st-lead' : 'text-accent'}">{it.type === 'event' ? '◆' : '●'}</span>
            <div class="min-w-0 flex-1">
              <pre class="whitespace-pre-wrap break-words font-mono text-[13px] text-ink">{it.text}</pre>
              <p class="mt-0.5 font-mono text-[11px] text-faint">{fmtDay(it.when)}</p>
            </div>
            {#if it.type === 'note'}
              <button onclick={() => deleteNote(it.id)} aria-label="Delete note" class="shrink-0 font-mono text-[15px] leading-none text-faint transition hover:text-st-lost">×</button>
            {:else}
              <button onclick={() => push('/calendar')} aria-label="Open calendar" class="shrink-0 font-mono text-[13px] text-faint transition hover:text-accent">↗</button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Projects + roster -->
  <div class="order-1 flex flex-col gap-8 lg:order-2">
    <div class="rise" style="animation-delay:50ms">
      <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> projects</p>
      {#if myProjects.length === 0}
        <p class="font-mono text-[12px] text-faint">no projects</p>
      {:else}
        <div class="flex flex-col gap-2">
          {#each myProjects as p (p.id)}
            <button onclick={() => push('/projects/' + p.id)} class="flex items-center gap-2.5 rounded-sm border border-border bg-surface px-3 py-2 text-left transition hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
              <span class="font-mono text-[10px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[p.status] ?? 'text-muted'}">[ {p.status} ]</span>
              <span class="min-w-0 flex-1 truncate font-mono text-[13px] text-ink">{p.title}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    {#if contact?.kind === 'company'}
      <div class="rise" style="animation-delay:70ms">
        <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> roster</p>
        {#if roster.length === 0}
          <p class="font-mono text-[12px] text-faint">no people yet</p>
        {:else}
          <div class="flex flex-col gap-2">
            {#each roster as person (person.id)}
              <button onclick={() => push('/contacts/' + person.id)} class="flex flex-col items-start rounded-sm border border-border bg-surface px-3 py-2 text-left transition hover:border-accent-dim">
                <span class="font-mono text-[13px] text-ink">{person.name}</span>
                {#if person.email}<span class="font-mono text-[11px] text-faint">{person.email}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if editing}
  <ContactForm value={editing} {companies} onclose={() => (editing = null)} onsubmit={saveEdit} />
{/if}
```

- [ ] **Step 2: Verify it builds and tests pass**

Run: `cd frontend && npm run build && npm test`
Expected: build succeeds; Vitest suite passes.

- [ ] **Step 3: Manual smoke check**

Run `npm run dev`, open a contact from the grid. Verify:
- Header shows quick actions; `⌂ company` links to the company (for a person); stat strip shows correct active/total/notes/last-touch.
- Timeline merges notes + this contact's events; upcoming events appear under `⧗ upcoming`; note composer adds a note (button and ⌘⏎); note `×` deletes; event `↗` opens `/calendar`.
- For a **company** contact, the roster panel lists its people and links through; for a **person**, no roster panel renders.
- `edit` opens the shared form and saves.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/ContactDetail.svelte
git commit -m "feat(contacts): relationship-hub detail — timeline, stats, roster, quick actions"
```

---

## Self-Review

**Spec coverage:**
- No backend/schema change → Global Constraints + Task 4 client-side filter comment. ✅
- Card grid + search + kind toggle + per-card counts + quick actions + hover edit/delete → Task 3. ✅
- Quick-action header (mailto/tel/copy/edit/delete) + company link → Task 4. ✅
- Derived stat strip (active/total/notes/last-touch) → Task 1 (`lastTouch`,`humanizeSince`) + Task 4. ✅
- Projects panel restyled → Task 4. ✅
- Company-only roster → Task 1 (`companyRoster`) + Task 4. ✅
- Merged notes+events timeline, upcoming-first, inline composer (⌘⏎), note delete, event ↗ → Task 1 (`buildTimeline`,`eventsForContact`) + Task 4. ✅
- `log ▸` shortcuts navigate-only (ponytail) → Task 4. ✅
- Shared create/edit form reused → Task 2. ✅
- Tests on pure helpers → Task 1. ✅

**Deviations from spec (intentional, noted):**
- Spec said event `↗` "deep-link to the calendar day"; the Calendar route has no day-param support, so `↗` opens `/calendar`. Marked `ponytail:` in Task 4. Adding day-preselection would require editing Calendar (out of scope).
- Edit-from-detail added via the shared `ContactForm` (Task 2) rather than duplicating the form — DRY, both pages consume it.

**Placeholder scan:** none — all steps contain full code/commands.

**Type consistency:** helper names/signatures in Task 1 Interfaces match their call sites in Tasks 3–4 (`filterContacts`, `projectCountsByContact`, `eventsForContact`, `buildTimeline` returning `{upcoming, history}` with `Item.{type,id,when,text}`, `lastTouch`, `humanizeSince`, `companyRoster`, `companyName`). `ContactForm` prop names (`value`,`companies`,`onclose`,`onsubmit`) match both consumers. ✅
