<script>
  import { api } from '../lib/api.js'
  import { fmtTime } from '../lib/calendar.js'

  let counts = $state({ leads: 0, active: 0, open_tasks: 0 })
  let activeProjects = $state([])
  let dueTasks = $state([])
  let upcomingEvents = $state([])
  let contactsById = $state({})
  let projectsById = $state({})
  let newTask = $state('')
  let error = $state('')

  const MONTHS = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec']
  function fmtDate(iso) {
    const d = new Date(iso)
    return `${MONTHS[d.getMonth()]} ${d.getDate()}`
  }
  function fmtEvent(ev) {
    return ev.all_day ? fmtDate(ev.starts_at) : `${fmtDate(ev.starts_at)} ${fmtTime(ev.starts_at)}`
  }

  const tiles = $derived([
    { label: 'Leads', value: counts.leads, accent: false },
    { label: 'Active projects', value: counts.active, accent: true },
    { label: 'Open tasks', value: counts.open_tasks, accent: false },
  ])

  async function load() {
    error = ''
    try {
      const [dash, contacts, projects] = await Promise.all([
        api.get('/api/dashboard'),
        api.get('/api/contacts'),
        api.get('/api/projects'),
      ])
      counts = dash.counts
      activeProjects = dash.active_projects
      dueTasks = dash.due_tasks
      upcomingEvents = dash.upcoming_events ?? []
      contactsById = Object.fromEntries(contacts.map((c) => [c.id, c.name]))
      projectsById = Object.fromEntries(projects.map((p) => [p.id, p.title]))
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

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<div class="rise mb-5 grid grid-cols-3 gap-3">
  {#each tiles as t}
    <div class="group relative rounded-sm border border-border bg-surface p-4 transition-colors duration-100 hover:border-border-strong">
      <!-- corner ticks -->
      <span class="pointer-events-none absolute right-1.5 top-1.5 h-2 w-2 border-r border-t {t.accent ? 'border-accent' : 'border-border-strong'}"></span>
      <span class="pointer-events-none absolute bottom-1.5 left-1.5 h-2 w-2 border-b border-l {t.accent ? 'border-accent' : 'border-border-strong'}"></span>
      <div
        class="font-mono text-[32px] font-bold leading-none tabular-nums"
        class:text-accent={t.accent}
        class:glow-text={t.accent}
        class:text-ink={!t.accent}
      >{t.value}</div>
      <!-- Visible caption is lowercased prompt-style; the natural-case sr-only twin
           keeps the text matchable (e2e innerText) and SR-friendly. -->
      <div class="mt-3 font-mono text-[11px] lowercase tracking-wide text-faint" aria-hidden="true"><span class="text-accent-dim">&gt;</span> {t.label}</div>
      <span class="sr-only">{t.label}</span>
    </div>
  {/each}
</div>

<div class="rise grid grid-cols-2 gap-4" style="animation-delay:60ms">
  <section class="rounded-sm border border-border bg-surface">
    <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
      <h2 class="font-mono text-[12px] font-medium text-muted"><span class="text-accent glow-text">&gt;</span> active_projects</h2>
      <span class="font-mono text-[12px] tabular-nums text-faint">[{activeProjects.length}]</span>
    </div>
    <ul>
      {#each activeProjects as p}
        <li class="flex items-center gap-3 border-b border-border px-4 py-2.5 last:border-0">
          <span class="h-1.5 w-1.5 shrink-0 bg-st-active glow-soft"></span>
          <span class="truncate font-mono text-[13px] text-ink">{p.title}</span>
          <span class="ml-auto truncate font-mono text-[12px] text-faint">{contactsById[p.contact_id] ?? '—'}</span>
        </li>
      {:else}
        <li class="px-4 py-6 text-center font-mono text-[13px] text-faint">// no active projects</li>
      {/each}
    </ul>
  </section>

  <section class="rounded-sm border border-border bg-surface">
    <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
      <h2 class="font-mono text-[12px] font-medium text-muted"><span class="text-accent glow-text">&gt;</span> tasks_due</h2>
      <span class="font-mono text-[12px] tabular-nums text-faint">[{dueTasks.length}]</span>
    </div>
    <div class="px-4 pt-3">
      <form onsubmit={(e) => { e.preventDefault(); addTask() }} class="flex items-center gap-2 rounded-sm border border-border bg-surface-2 px-2.5 transition-colors duration-100 focus-within:border-accent focus-within:shadow-[0_0_0_3px_rgba(62,245,196,0.14)]">
        <span class="select-none font-mono text-[13px] text-accent-dim">$</span>
        <input
          bind:value={newTask}
          placeholder="add task…"
          class="h-8 flex-1 bg-transparent font-mono text-[13px] text-ink placeholder:text-faint focus:outline-none"
        />
        <button class="shrink-0 rounded-sm bg-accent px-2.5 py-1 font-mono text-[12px] font-bold text-on-accent transition hover:brightness-110">add</button>
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
            class="h-3.5 w-3.5 shrink-0 rounded-sm accent-accent"
          />
          <span class="truncate font-mono text-[13px] text-ink" class:line-through={t.status === 'done'} class:text-faint={t.status === 'done'}>{t.title}</span>
          {#if t.due_on}<span class="ml-auto shrink-0 font-mono text-[11px] tabular-nums text-faint">{t.due_on}</span>{/if}
        </li>
      {:else}
        <li class="py-6 text-center font-mono text-[13px] text-faint">// nothing due</li>
      {/each}
    </ul>
  </section>
</div>

<section class="rise mt-4 rounded-sm border border-border bg-surface" style="animation-delay:120ms">
  <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
    <h2 class="font-mono text-[12px] font-medium text-muted"><span class="text-accent glow-text">&gt;</span> upcoming</h2>
    <span class="font-mono text-[12px] tabular-nums text-faint">[{upcomingEvents.length}]</span>
  </div>
  <ul>
    {#each upcomingEvents as ev}
      <li class="flex items-center gap-3 border-b border-border px-4 py-2.5 last:border-0">
        <span class="truncate font-mono text-[13px] text-ink">{ev.title}</span>
        {#if ev.contact_id || ev.project_id}
          <span class="shrink-0 font-mono text-[11px] text-faint">{ev.contact_id ? (contactsById[ev.contact_id] ?? '') : (projectsById[ev.project_id] ?? '')}</span>
        {/if}
        <span class="ml-auto shrink-0 font-mono text-[11px] tabular-nums text-faint">{fmtEvent(ev)}</span>
      </li>
    {:else}
      <li class="px-4 py-6 text-center font-mono text-[13px] text-faint">// no upcoming events</li>
    {/each}
  </ul>
</section>
