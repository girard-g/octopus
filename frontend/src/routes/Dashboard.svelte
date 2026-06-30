<script>
  import { api } from '../lib/api.js'

  let counts = $state({ leads: 0, active: 0, open_tasks: 0 })
  let activeProjects = $state([])
  let dueTasks = $state([])
  let contactsById = $state({})
  let newTask = $state('')
  let error = $state('')

  const tiles = $derived([
    { label: 'Leads', value: counts.leads, accent: false },
    { label: 'Active projects', value: counts.active, accent: true },
    { label: 'Open tasks', value: counts.open_tasks, accent: false },
  ])

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
    try {
      await api.post('/api/tasks', { title })
      newTask = ''
      await load()
    } catch (e) { error = e.message }
  }

  async function toggleDone(t) {
    try {
      await api.put(`/api/tasks/${t.id}`, { title: t.title, status: 'done', project_id: t.project_id, due_on: t.due_on })
      await load()
    } catch (e) { error = e.message }
  }

  $effect(() => { load() })
</script>

<header class="rise mb-6 flex items-end justify-between">
  <div>
    <h1 class="text-[22px] font-semibold tracking-[-0.01em] text-ink">Dashboard</h1>
    <p class="mt-0.5 text-[13px] text-muted">Pipeline health and what's due.</p>
  </div>
</header>

{#if error}<p class="rise mb-4 rounded-md border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">{error}</p>{/if}

<div class="rise mb-6 grid grid-cols-3 gap-3" style="animation-delay:40ms">
  {#each tiles as t}
    <div class="rounded-lg border border-border bg-surface p-4 shadow-[0_1px_2px_rgba(0,0,0,0.4)]">
      <div
        class="font-mono text-[28px] font-medium leading-none"
        class:text-accent={t.accent}
        class:text-ink={!t.accent}
      >{t.value}</div>
      <!-- Visible label is CSS-uppercased; innerText reflects that transform, so a
           natural-case sr-only twin keeps the text matchable (e2e) and SR-friendly. -->
      <div class="label mt-2.5" aria-hidden="true">{t.label}</div>
      <span class="sr-only">{t.label}</span>
    </div>
  {/each}
</div>

<div class="rise grid grid-cols-2 gap-4" style="animation-delay:80ms">
  <section class="rounded-lg border border-border bg-surface shadow-[0_1px_2px_rgba(0,0,0,0.4)]">
    <div class="flex items-center justify-between border-b border-border px-4 py-3">
      <h2 class="label">Active projects</h2>
      <span class="font-mono text-[12px] text-faint">{activeProjects.length}</span>
    </div>
    <ul>
      {#each activeProjects as p}
        <li class="flex items-center gap-3 border-b border-border px-4 py-2.5 last:border-0">
          <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-st-active"></span>
          <span class="truncate text-[13px] font-medium text-ink">{p.title}</span>
          <span class="ml-auto truncate font-mono text-[12px] text-faint">{contactsById[p.contact_id] ?? '—'}</span>
        </li>
      {:else}
        <li class="px-4 py-6 text-center text-[13px] text-faint">No active projects.</li>
      {/each}
    </ul>
  </section>

  <section class="rounded-lg border border-border bg-surface shadow-[0_1px_2px_rgba(0,0,0,0.4)]">
    <div class="flex items-center justify-between border-b border-border px-4 py-3">
      <h2 class="label">Tasks due</h2>
      <span class="font-mono text-[12px] text-faint">{dueTasks.length}</span>
    </div>
    <div class="px-4 pt-3">
      <form onsubmit={(e) => { e.preventDefault(); addTask() }} class="flex gap-2">
        <input
          bind:value={newTask}
          placeholder="Quick add task…"
          class="h-8 flex-1 rounded-md border border-border bg-surface-2 px-2.5 text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(69,214,196,0.12)] focus:outline-none"
        />
        <button class="h-8 shrink-0 rounded-md bg-accent px-3 text-[13px] font-medium text-on-accent transition hover:brightness-110">Add</button>
      </form>
    </div>
    <ul class="px-4 py-2">
      {#each dueTasks as t}
        <li class="flex items-center gap-2.5 border-b border-border py-2 last:border-0">
          <input
            type="checkbox"
            checked={t.status === 'done'}
            onchange={() => toggleDone(t)}
            aria-label="Mark {t.title} done"
            class="h-3.5 w-3.5 shrink-0 rounded accent-accent"
          />
          <span class="truncate text-[13px] text-ink" class:line-through={t.status === 'done'} class:text-faint={t.status === 'done'}>{t.title}</span>
          {#if t.due_on}<span class="ml-auto shrink-0 font-mono text-[11px] text-faint">{t.due_on}</span>{/if}
        </li>
      {:else}
        <li class="py-6 text-center text-[13px] text-faint">Nothing due.</li>
      {/each}
    </ul>
  </section>
</div>
