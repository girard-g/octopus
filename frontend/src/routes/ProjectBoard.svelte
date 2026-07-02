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
  // Card left-bar encodes PRIORITY (the column already conveys status).
  const PRIORITY_BAR = { high: 'bg-st-lost', medium: 'bg-st-proposal', low: 'bg-st-done' }
  const TYPE_BADGE = {
    feature:     'text-st-done',
    bug:         'text-st-lost',
    enhancement: 'text-accent',
    chore:       'text-faint',
    docs:        'text-st-proposal',
  }
  const PROJECT_STATUS_TEXT = { active: 'text-st-active', archived: 'text-muted' }
  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
  const ADDFIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-1.5 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  const todayISO = new Date().toISOString().slice(0, 10)
  const isOverdue = (t) => t.due_on && t.status !== 'done' && t.due_on < todayISO

  // Focus an input the moment it mounts (per-column add / inline edit).
  function focusOnMount(node) { node.focus() }

  let { params } = $props()
  const id = $derived(params.id)

  let project   = $state(null)
  let contacts  = $state([])
  let tasks     = $state([])
  let notes     = $state([])
  let cols      = $state(Object.fromEntries(TASK_STATUSES.map((s) => [s, []])))
  let error     = $state('')
  let editing   = $state(null)   // project edit modal
  let adding    = $state(null)   // which column's inline add input is open (status | null)
  let addTitle  = $state('')
  let newNote   = $state('')
  let noteBusy  = $state(false)
  let notesOpen = $state(false)
  let editingTask = $state(null)   // a shallow copy of the task being edited
  let newChecklistItem = $state('')

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
            await api.put('/api/tasks/' + m.id, { ...t, status: m.status, position: m.position, project_id: id })
          }
        } catch (err) { error = err.message }
        await load()
      },
    }
  }

  // Per-column add: create straight into that column's status, appended at the end.
  function startAdd(s) { adding = s; addTitle = '' }
  function cancelAdd() { adding = null; addTitle = '' }
  async function submitAdd(s) {
    const t = addTitle.trim()
    if (!t) { cancelAdd(); return }
    try {
      await api.post('/api/tasks', { title: t, project_id: id, status: s, position: cols[s].length })
      addTitle = ''            // keep the input open for rapid multi-add
      await load()
    } catch (err) { error = err.message }
  }

  async function deleteTask(taskId) {
    try { await api.del('/api/tasks/' + taskId); await load() }
    catch (e) { error = e.message }
  }

  function openTask(t) {
    editingTask = { ...t, checklist: (t.checklist ?? []).map((c) => ({ ...c })) }
  }
  function addChecklistItem() {
    const title = newChecklistItem.trim()
    if (!title) return
    editingTask.checklist = [...editingTask.checklist, { title, done: false }]
    newChecklistItem = ''
  }
  function removeChecklistItem(i) {
    editingTask.checklist = editingTask.checklist.filter((_, idx) => idx !== i)
  }
  async function saveTask(e) {
    e.preventDefault()
    const t = editingTask
    try {
      await api.put('/api/tasks/' + t.id, {
        title: t.title,
        status: t.status,
        project_id: id,
        due_on: t.due_on || null,
        priority: t.priority || null,
        size: t.size || null,
        description: t.description || null,
        version: t.version || null,
        type: t.type || null,
        checklist: t.checklist,
        position: t.position ?? 0,
      })
      editingTask = null
      await load()
    } catch (err) { error = err.message }
  }

  function openEdit() { editing = { ...project } }
  async function saveEdit(e) {
    e.preventDefault()
    try {
      await api.put('/api/projects/' + id, {
        contact_id: editing.contact_id || null,
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
    try { await api.del('/api/projects/' + id); push('/projects') }
    catch (e) { error = e.message }
  }

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

<!-- ponytail: fixed board height = shell header (h-13 = 52px) + route padding (py-7 = 28+28).
     Retune 108px if App.svelte's header height or content padding changes.
     Mobile (<md) uses natural height + per-scroller heights instead — viewport chrome shifts too much to pin. -->
<section class="rise flex h-auto flex-col md:h-[calc(100vh-108px)]">
  <!-- Header -->
  <header class="mb-4 flex flex-wrap items-center gap-x-3 gap-y-2">
    <button
      onclick={() => push('/projects')}
      class="font-mono text-[12px] text-faint transition hover:text-accent"
    >&lt; projects</button>
    <span class="font-mono text-[12px] text-faint">/</span>
    <h2 class="font-mono text-[15px] font-bold text-ink">{project?.title ?? '…'}</h2>
    {#if project}
      <span class="font-mono text-[11px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[project.status] ?? 'text-muted'}">[ {project.status} ]</span>
      {#if contactsById[project.contact_id]}<span class="font-mono text-[12px] text-muted">{contactsById[project.contact_id]}</span>{/if}
    {/if}

    <div class="ml-auto flex flex-wrap items-center gap-2">
      <button
        onclick={() => (notesOpen = !notesOpen)}
        class="h-8 rounded-sm border px-3 font-mono text-[12px] transition {notesOpen ? 'border-accent-dim text-ink' : 'border-border-2 text-muted hover:border-accent-dim hover:text-ink'}"
      >notes [{notes.length}]</button>
      {#if project?.invoice_url}
        <a
          href={project.invoice_url}
          target="_blank"
          rel="noopener noreferrer"
          class="inline-flex h-8 items-center rounded-sm border border-border-2 px-3 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
        >invoice ↗</a>
      {/if}
      <button
        onclick={toggleArchive}
        class="h-8 rounded-sm border border-border-2 px-3 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
      >{project?.status === 'archived' ? 'restore' : 'archive'}</button>
      <button
        onclick={openEdit}
        class="h-8 rounded-sm border border-border-2 px-3 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
      >edit</button>
      <button
        onclick={deleteProject}
        class="h-8 rounded-sm border border-st-lost/40 px-3 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10"
      >delete</button>
    </div>

    {#if project?.description}
      <p class="w-full max-w-3xl font-mono text-[12px] leading-relaxed text-faint">{project.description}</p>
    {/if}
  </header>

  {#if error}
    <p class="mb-3 shrink-0 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}

  <!-- Board + optional notes rail -->
  <div class="flex min-h-0 flex-1 flex-col gap-4 md:flex-row">
    <div class="flex h-[65dvh] min-h-0 gap-3 overflow-x-auto pb-1 snap-x snap-mandatory md:h-auto md:flex-1 md:snap-none">
      {#each TASK_STATUSES as s}
        <div class="flex min-h-0 w-[85vw] shrink-0 snap-center flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40 md:w-[280px] md:min-w-[240px] md:shrink md:flex-1 md:snap-align-none">
          <!-- status accent strip -->
          <span class="h-[3px] w-full {TASK_STYLE[s].bar}"></span>
          <!-- column header -->
          <div class="flex shrink-0 items-center justify-between border-b border-border/50 px-3 py-2">
            <span class="font-mono text-[11px] font-bold uppercase tracking-wider {TASK_STYLE[s].text}">{TASK_LABELS[s]}</span>
            <span class="font-mono text-[12px] tabular-nums text-faint">{cols[s].length}</span>
          </div>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <!-- scrollable, droppable card list -->
          <div
            class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2"
            use:dndzone={{ items: cols[s], flipDurationMs: 150, dropTargetStyle: {} }}
            onconsider={dndHandlers(s).consider}
            onfinalize={dndHandlers(s).finalize}
          >
            {#each cols[s] as t (t.id)}
              <div class="group relative flex items-start gap-2 overflow-hidden rounded-sm border border-border bg-surface py-2.5 pl-3.5 pr-2 transition-all duration-100 hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
                <span class="absolute inset-y-0 left-0 w-[3px] {PRIORITY_BAR[t.priority] ?? 'bg-border'}"></span>
                <button
                  type="button"
                  onclick={() => openTask(t)}
                  class="min-w-0 flex-1 text-left"
                >
                  <div class="break-words font-mono text-[13px] font-medium leading-snug text-ink">{t.title}</div>
                  {#if t.due_on || t.size || t.checklist?.length || t.type || t.version}
                    <div class="mt-1 flex flex-wrap items-center gap-x-2.5 gap-y-1 font-mono text-[11px]">
                      {#if t.type}<span class="uppercase {TYPE_BADGE[t.type] ?? 'text-faint'}">{t.type}</span>{/if}
                      {#if t.version}<span class="text-faint">{t.version}</span>{/if}
                      {#if t.due_on}<span class={isOverdue(t) ? 'text-st-lost' : 'text-faint'}>{t.due_on}</span>{/if}
                      {#if t.size}<span class="uppercase text-faint">[{t.size}]</span>{/if}
                      {#if t.checklist?.length}<span class="text-faint">✓ {t.checklist.filter((c) => c.done).length}/{t.checklist.length}</span>{/if}
                    </div>
                  {/if}
                </button>
                <button
                  onclick={(e) => { e.stopPropagation(); deleteTask(t.id) }}
                  aria-label="Delete task"
                  class="shrink-0 font-mono text-[16px] leading-none text-faint opacity-100 transition hover:text-st-lost focus:opacity-100 md:opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
                >×</button>
              </div>
            {/each}
          </div>
          <!-- per-column add -->
          <div class="shrink-0 border-t border-border/50 p-1.5">
            {#if adding === s}
              <form onsubmit={(e) => { e.preventDefault(); submitAdd(s) }}>
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  use:focusOnMount
                  bind:value={addTitle}
                  onkeydown={(e) => { if (e.key === 'Escape') cancelAdd() }}
                  onblur={() => { if (!addTitle.trim()) cancelAdd() }}
                  placeholder="task title…  (enter to add, esc to close)"
                  class={ADDFIELD}
                />
              </form>
            {:else}
              <button
                onclick={() => startAdd(s)}
                class="flex w-full items-center gap-1.5 rounded-sm px-2 py-1.5 font-mono text-[12px] text-faint transition hover:bg-surface-2 hover:text-accent"
              ><span class="text-[15px] leading-none">+</span> add task</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    {#if notesOpen}
      <aside class="flex max-h-[50dvh] w-full shrink-0 flex-col overflow-hidden rounded-md border border-border/60 bg-bg-2/40 md:max-h-none md:w-[300px]">
        <div class="flex shrink-0 items-center justify-between border-b border-border/50 px-3 py-2">
          <span class="font-mono text-[11px] font-bold uppercase tracking-wider text-muted"><span class="text-accent glow-text">&gt;</span> notes</span>
          <button onclick={() => (notesOpen = false)} aria-label="Close notes" class="font-mono text-[15px] leading-none text-faint transition hover:text-ink">×</button>
        </div>
        <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
          {#if notes.length === 0}
            <p class="px-1 py-3 font-mono text-[12px] text-faint">no notes yet</p>
          {:else}
            {#each notes as n (n.id)}
              <div class="group rounded-sm border border-border bg-surface p-2.5">
                <div class="flex items-start gap-2">
                  <pre class="min-w-0 flex-1 whitespace-pre-wrap break-words font-mono text-[13px] text-ink">{n.body}</pre>
                  <button
                    onclick={() => deleteNote(n.id)}
                    aria-label="Delete note"
                    class="shrink-0 font-mono text-[15px] leading-none text-faint opacity-0 transition hover:text-st-lost focus:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100"
                  >×</button>
                </div>
                <div class="mt-1.5 font-mono text-[11px] text-faint">{new Date(n.created_at).toLocaleDateString()}</div>
              </div>
            {/each}
          {/if}
        </div>
        <form onsubmit={addNote} class="flex shrink-0 flex-col gap-2 border-t border-border/50 p-2">
          <textarea
            bind:value={newNote}
            placeholder="note body…"
            rows="2"
            class="{FIELD} resize-none"
          ></textarea>
          <button
            type="submit"
            disabled={noteBusy}
            class="h-8 self-start rounded-sm bg-accent px-3 font-mono text-[12px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
          >add note</button>
        </form>
      </aside>
    {/if}
  </div>
</section>

<!-- Edit project modal (sibling of the board section — never inside a transformed container) -->
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
          <option value={null}>— none —</option>
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

{#if editingTask}
  <Modal title="Edit task" onclose={() => (editingTask = null)}>
    <form onsubmit={saveTask} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={editingTask.title} required class={FIELD} />
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <p class="label mb-1.5">Priority</p>
          <select bind:value={editingTask.priority} class={FIELD}>
            <option value={null}>— none —</option>
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
          </select>
        </div>
        <div>
          <p class="label mb-1.5">Size</p>
          <select bind:value={editingTask.size} class={FIELD}>
            <option value={null}>— none —</option>
            <option value="xs">XS</option>
            <option value="s">S</option>
            <option value="m">M</option>
            <option value="l">L</option>
            <option value="xl">XL</option>
          </select>
        </div>
      </div>
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
      <div>
        <p class="label mb-1.5">Due date</p>
        <input type="date" bind:value={editingTask.due_on} class={FIELD} />
      </div>
      <div>
        <p class="label mb-1.5">Description</p>
        <textarea bind:value={editingTask.description} rows="3" class="{FIELD} resize-none"></textarea>
      </div>
      <div>
        <p class="label mb-1.5">Checklist</p>
        <div class="flex flex-col gap-1.5">
          {#each editingTask.checklist as item, i (i)}
            <div class="flex items-center gap-2">
              <input type="checkbox" bind:checked={item.done} class="h-3.5 w-3.5 shrink-0 rounded-sm accent-accent" />
              <span class="min-w-0 flex-1 truncate font-mono text-[13px] text-ink" class:line-through={item.done} class:text-faint={item.done}>{item.title}</span>
              <button type="button" onclick={() => removeChecklistItem(i)} aria-label="Remove item" class="shrink-0 font-mono text-[15px] leading-none text-faint transition hover:text-st-lost">×</button>
            </div>
          {/each}
          <div class="flex gap-2">
            <input bind:value={newChecklistItem} onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); addChecklistItem() } }} placeholder="add item…" class="{FIELD} flex-1" />
            <button type="button" onclick={addChecklistItem} class="h-[38px] shrink-0 rounded-sm border border-border-2 px-3 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink">+</button>
          </div>
        </div>
      </div>
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
    </form>
  </Modal>
{/if}
