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
