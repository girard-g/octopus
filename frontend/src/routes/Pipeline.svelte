<script>
  import { dndzone } from 'svelte-dnd-action'
  import { api } from '../lib/api.js'
  import { STATUSES, STATUS_LABELS, groupByStatus, movesForColumn } from '../lib/pipeline.js'
  import Modal from '../lib/components/Modal.svelte'

  // Per-status terminal styling. Static class strings so Tailwind's scanner emits them.
  const STATUS_STYLE = {
    lead:     { text: 'text-st-lead',     bar: 'bg-st-lead' },
    proposal: { text: 'text-st-proposal', bar: 'bg-st-proposal' },
    active:   { text: 'text-st-active',    bar: 'bg-st-active' },
    done:     { text: 'text-st-done',      bar: 'bg-st-done' },
    lost:     { text: 'text-st-lost',      bar: 'bg-st-lost' },
  }
  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

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
    try {
      await api.del(`/api/projects/${p.id}`)
      editing = null
      await load()
    } catch (e) { error = e.message }
  }

  $effect(() => { load() })
</script>

<div class="rise mb-5 flex items-center justify-between">
  <p class="font-mono text-[12px] text-faint"><span class="text-accent-dim">//</span> drag cards across stages to update status</p>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> New lead
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<div class="rise flex gap-3 overflow-x-auto pb-3" style="animation-delay:40ms">
  {#each STATUSES as s}
    <div class="flex w-52 shrink-0 flex-col">
      <div class="mb-2.5 flex items-center justify-between px-0.5">
        <span class="font-mono text-[11px] font-bold uppercase tracking-wider {STATUS_STYLE[s].text}">[ {STATUS_LABELS[s]} ]</span>
        <span class="font-mono text-[12px] tabular-nums text-faint">[{cols[s].length}]</span>
      </div>
      <div
        class="flex min-h-[88px] flex-1 flex-col gap-2 rounded-sm border border-border/60 bg-bg-2/50 p-1.5"
        use:dndzone={{ items: cols[s], flipDurationMs: 150, dropTargetStyle: {} }}
        onconsider={dndHandlers(s).consider}
        onfinalize={dndHandlers(s).finalize}
      >
        {#each cols[s] as p (p.id)}
          <button
            onclick={() => openEdit(p)}
            class="group relative w-full overflow-hidden rounded-sm border border-border bg-surface text-left transition-all duration-100 hover:-translate-y-px hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
          >
            <span class="absolute inset-y-0 left-0 w-[3px] {STATUS_STYLE[s].bar}"></span>
            <div class="py-2.5 pl-3.5 pr-2.5">
              <div class="font-mono text-[13px] font-medium leading-snug text-ink">{p.title}</div>
              <div class="mt-1 truncate font-mono text-[11px] text-faint">{contactsById[p.contact_id] ?? '—'}</div>
              {#if p.invoice_url}
                <div class="mt-1.5 inline-flex items-center gap-1 font-mono text-[11px] text-accent glow-text">&gt; invoice</div>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/each}
</div>

{#if creating}
  <Modal title="New lead" onclose={() => (creating = null)}>
    <form onsubmit={createLead} class="flex flex-col gap-3">
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
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Create lead</button>
    </form>
  </Modal>
{/if}

{#if editing}
  <Modal title="Edit project" onclose={() => (editing = null)}>
    <form onsubmit={saveEdit} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={editing.title} placeholder="Title" required class={FIELD} />
      </div>
      <div>
        <p class="label mb-1.5">Contact</p>
        <select bind:value={editing.contact_id} class={FIELD}>
          {#each contacts as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </div>
      <div>
        <p class="label mb-1.5">Description</p>
        <textarea bind:value={editing.description} placeholder="Description" rows="3" class="{FIELD} resize-none"></textarea>
      </div>
      <div>
        <p class="label mb-1.5">Invoice URL</p>
        <input bind:value={editing.invoice_url} placeholder="Indy invoice URL" class={FIELD} />
      </div>
      <p class="flex items-center gap-2 font-mono text-[11px] text-faint">
        <span class="font-bold uppercase tracking-wider {STATUS_STYLE[editing.status].text}">[ {STATUS_LABELS[editing.status]} ]</span>
        change by dragging on the board
      </p>
      <div class="mt-1 flex gap-2">
        <button class="h-9 flex-1 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
        <button type="button" onclick={() => removeProject(editing)} class="h-9 rounded-sm border border-st-lost/40 px-3 font-mono text-[13px] font-medium text-st-lost transition hover:bg-st-lost/10">Delete</button>
      </div>
    </form>
  </Modal>
{/if}
