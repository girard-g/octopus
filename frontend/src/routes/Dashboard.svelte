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
