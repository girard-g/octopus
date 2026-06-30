<script>
  import { dndzone } from 'svelte-dnd-action'
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import { TASK_STATUSES, TASK_LABELS, groupTasks, movesForTaskColumn } from '../lib/tasks.js'
  import Modal from '../lib/components/Modal.svelte'

  // Static class strings so Tailwind's scanner emits them.
  const TASK_STYLE = {
    todo:  { text: 'text-st-lead',     bar: 'bg-st-lead' },
    doing: { text: 'text-st-proposal', bar: 'bg-st-proposal' },
    done:  { text: 'text-st-done',     bar: 'bg-st-done' },
  }
  const PROJECT_STATUS_TEXT = {
    lead: 'text-st-lead', proposal: 'text-st-proposal', active: 'text-st-active',
    done: 'text-st-done', lost: 'text-st-lost',
  }
  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let { params } = $props()
  const id = $derived(params.id)

  let project   = $state(null)
  let contacts  = $state([])
  let tasks     = $state([])
  let notes     = $state([])
  let cols      = $state(Object.fromEntries(TASK_STATUSES.map((s) => [s, []])))
  let error     = $state('')
  let editing   = $state(null)   // project edit modal
  let newTitle  = $state('')
  let newNote   = $state('')
  let noteBusy  = $state(false)

  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))

  async function load() {
    error = ''
    try {
      ;[project, contacts, tasks, notes] = await Promise.all([
        api.get('/api/projects/' + id),
        api.get('/api/contacts'),
        api.get('/api/tasks?project_id=' + id),
        api.get('/api/notes?project_id=' + id),
      ])
      cols = groupTasks(tasks)
    } catch (e) { error = e.message }
  }

  $effect(() => { if (id) load() })

  // dnd: status captured per column via factory.
  function dndHandlers(status) {
    return {
      consider: (e) => { cols[status] = e.detail.items; cols = cols },
      finalize: async (e) => {
        cols[status] = e.detail.items; cols = cols
        const moves = movesForTaskColumn(status, e.detail.items)
        try {
          for (const m of moves) {
            const t = e.detail.items.find((x) => x.id === m.id)
            await api.put('/api/tasks/' + m.id, {
              title: t.title,
              status: m.status,
              project_id: id,
              due_on: t.due_on ?? null,
            })
          }
        } catch (err) { error = err.message }
        await load()
      },
    }
  }

  async function addTask(e) {
    e.preventDefault()
    const t = newTitle.trim()
    if (!t) return
    try {
      await api.post('/api/tasks', { title: t, project_id: id })
      newTitle = ''
      await load()
    } catch (err) { error = err.message }
  }

  async function deleteTask(taskId) {
    try { await api.del('/api/tasks/' + taskId); await load() }
    catch (e) { error = e.message }
  }

  function openEdit() { editing = { ...project } }
  async function saveEdit(e) {
    e.preventDefault()
    try {
      await api.put('/api/projects/' + id, {
        contact_id: editing.contact_id,
        title: editing.title,
        description: editing.description || null,
        invoice_url: editing.invoice_url || null,
      })
      editing = null
      await load()
    } catch (err) { error = err.message }
  }

  async function deleteProject() {
    if (!confirm('Delete ' + (project?.title ?? 'this project') + '?')) return
    try { await api.del('/api/projects/' + id); push('/pipeline') }
    catch (e) { error = e.message }
  }

  async function addNote(e) {
    e.preventDefault()
    const b = newNote.trim()
    if (!b) return
    noteBusy = true
    try {
      await api.post('/api/notes', { body: b, project_id: id })
      newNote = ''
      await load()
    } catch (err) { error = err.message }
    finally { noteBusy = false }
  }

  async function deleteNote(noteId) {
    try { await api.del('/api/notes/' + noteId); await load() }
    catch (e) { error = e.message }
  }
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="mb-4 flex flex-wrap items-center gap-3">
    <button
      onclick={() => push('/pipeline')}
      class="font-mono text-[12px] text-faint transition hover:text-accent"
    >&lt; pipeline</button>
    <span class="font-mono text-[12px] text-faint">/</span>
    <h2 class="font-mono text-[15px] font-bold text-ink">{project?.title ?? '…'}</h2>
    {#if project}
      <span class="font-mono text-[11px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[project.status] ?? 'text-muted'}">[ {project.status} ]</span>
      <span class="font-mono text-[12px] text-muted">{contactsById[project.contact_id] ?? '—'}</span>
    {/if}
    <div class="ml-auto flex gap-2">
      <button
        onclick={openEdit}
        class="h-8 rounded-sm border border-border-2 px-3 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
      >edit</button>
      <button
        onclick={deleteProject}
        class="h-8 rounded-sm border border-st-lost/40 px-3 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10"
      >delete</button>
    </div>
  </div>

  {#if error}
    <p class="mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Task board -->
<div class="rise mb-6 flex gap-3 overflow-x-auto pb-3" style="animation-delay:40ms">
  {#each TASK_STATUSES as s}
    <div class="flex w-60 shrink-0 flex-col">
      <div class="mb-2.5 flex items-center justify-between px-0.5">
        <span class="font-mono text-[11px] font-bold uppercase tracking-wider {TASK_STYLE[s].text}">[ {TASK_LABELS[s]} ]</span>
        <span class="font-mono text-[12px] tabular-nums text-faint">[{cols[s].length}]</span>
      </div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="flex min-h-[88px] flex-1 flex-col gap-2 rounded-sm border border-border/60 bg-bg-2/50 p-1.5"
        use:dndzone={{ items: cols[s], flipDurationMs: 150, dropTargetStyle: {} }}
        onconsider={dndHandlers(s).consider}
        onfinalize={dndHandlers(s).finalize}
      >
        {#each cols[s] as t (t.id)}
          <div class="group relative overflow-hidden rounded-sm border border-border bg-surface transition-all duration-100 hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
            <span class="absolute inset-y-0 left-0 w-[3px] {TASK_STYLE[s].bar}"></span>
            <div class="flex items-start justify-between py-2.5 pl-3.5 pr-2">
              <div class="min-w-0 flex-1">
                <div class="font-mono text-[13px] font-medium leading-snug text-ink">{t.title}</div>
                {#if t.due_on}
                  <div class="mt-0.5 font-mono text-[11px] text-faint">{t.due_on}</div>
                {/if}
              </div>
              <button
                onclick={() => deleteTask(t.id)}
                aria-label="Delete task"
                class="ml-2 shrink-0 font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
              >×</button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/each}
</div>

<!-- Add-task command input -->
<form onsubmit={addTask} class="rise mb-10 flex gap-2" style="animation-delay:80ms">
  <div class="relative flex-1">
    <span class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 font-mono text-[13px] text-accent glow-text">$</span>
    <input
      bind:value={newTitle}
      placeholder="add task…"
      class="w-full rounded-sm border border-border bg-surface-2 py-2 pl-7 pr-2.5 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none"
    />
  </div>
  <button
    type="submit"
    class="h-[38px] rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >add</button>
</form>

<!-- Notes panel -->
<div class="rise" style="animation-delay:120ms">
  <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> notes</p>

  <div class="mb-4 flex flex-col gap-2">
    {#if notes.length === 0}
      <p class="font-mono text-[12px] text-faint">no notes yet</p>
    {:else}
      {#each notes as n (n.id)}
        <div class="group rounded-sm border border-border bg-surface p-3">
          <div class="flex items-start gap-2">
            <pre class="min-w-0 flex-1 whitespace-pre-wrap break-words font-mono text-[13px] text-ink">{n.body}</pre>
            <button
              onclick={() => deleteNote(n.id)}
              aria-label="Delete note"
              class="shrink-0 font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
            >×</button>
          </div>
          <div class="mt-1.5 font-mono text-[11px] text-faint">{new Date(n.created_at).toLocaleDateString()}</div>
        </div>
      {/each}
    {/if}
  </div>

  <form onsubmit={addNote} class="flex flex-col gap-2">
    <textarea
      bind:value={newNote}
      placeholder="note body…"
      rows="3"
      class="{FIELD} resize-none"
    ></textarea>
    <button
      type="submit"
      disabled={noteBusy}
      class="self-start h-9 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
    >add note</button>
  </form>
</div>

<!-- Edit project modal -->
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
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
    </form>
  </Modal>
{/if}
