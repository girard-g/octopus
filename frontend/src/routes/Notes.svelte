<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'
  import { noteTitle, buildFolderTree, sortNotes, searchNotes, folderPath, folderBlastRadius } from '../lib/notes.js'
  import { renderMarkdown } from '../lib/markdown.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let folders  = $state([])
  let notes    = $state([])
  let contacts = $state([])
  let projects = $state([])
  let error    = $state('')

  let query          = $state('')
  let selectedFolder = $state(null)   // folder id, null = Unfiled context
  let expanded       = $state(new Set())
  let mobileView     = $state('edit') // 'edit' | 'preview'

  // editing buffer; null when nothing open
  let draft      = $state(null)
  let saveStatus = $state('idle')     // 'idle' | 'saving' | 'saved'
  let saveTimer  = null
  let creating   = false // guards against a duplicate POST if persist() re-enters while a create is in flight

  const tree         = $derived(buildFolderTree(folders))
  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))
  const projectsById = $derived(Object.fromEntries(projects.map((p) => [p.id, p.title])))
  const searchHits   = $derived(query.trim() ? sortNotes(searchNotes(notes, query)) : [])
  const unfiled      = $derived(sortNotes(notes.filter((n) => !n.folder_id)))

  function notesIn(folderId) {
    return sortNotes(notes.filter((n) => n.folder_id === folderId))
  }

  async function load() {
    error = ''
    try {
      ;[folders, notes, contacts, projects] = await Promise.all([
        api.get('/api/folders'),
        api.get('/api/notes'),
        api.get('/api/contacts'),
        api.get('/api/projects'),
      ])
    } catch (e) { error = e.message }
  }
  $effect(() => { load() })

  function toggle(id) {
    const next = new Set(expanded)
    next.has(id) ? next.delete(id) : next.add(id)
    expanded = next
  }

  // ---- editor / autosave ----
  function hasContent(d) { return !!((d.title || '').trim() || (d.body || '').trim()) }

  function mergeNote(n) {
    const i = notes.findIndex((x) => x.id === n.id)
    if (i >= 0) notes[i] = n
    else notes = [n, ...notes]
  }

  async function persist() {
    clearTimeout(saveTimer); saveTimer = null
    if (!draft) return
    if (!hasContent(draft)) { saveStatus = 'idle'; return } // never write an empty draft; never wipe to empty
    if (!draft.id && creating) return // a create is already in flight for this draft; the next scheduleSave/blur will retry
    const payload = {
      title: draft.title.trim() || null,
      body: draft.body,
      folder_id: draft.folder_id,
      contact_id: draft.contact_id,
      project_id: draft.project_id,
      pinned: draft.pinned,
    }
    saveStatus = 'saving'
    try {
      if (draft.id) { mergeNote(await api.put('/api/notes/' + draft.id, payload)) }
      else { creating = true; const n = await api.post('/api/notes', payload); draft.id = n.id; mergeNote(n) }
      saveStatus = 'saved'
    } catch (e) { error = e.message; saveStatus = 'idle' }
    finally { creating = false }
  }

  function scheduleSave() {
    saveStatus = 'saving'
    clearTimeout(saveTimer)
    saveTimer = setTimeout(persist, 600)
  }

  async function openNote(n) {
    await persist()
    draft = { id: n.id, title: n.title || '', body: n.body || '', folder_id: n.folder_id, contact_id: n.contact_id, project_id: n.project_id, pinned: n.pinned }
    saveStatus = 'saved'
    mobileView = 'preview'
  }

  async function newNote() {
    await persist()
    draft = { id: null, title: '', body: '', folder_id: selectedFolder, contact_id: null, project_id: null, pinned: false }
    saveStatus = 'idle'
    mobileView = 'edit'
  }

  async function deleteNote() {
    if (!draft?.id) { draft = null; return }
    try { await api.del('/api/notes/' + draft.id); notes = notes.filter((n) => n.id !== draft.id); draft = null }
    catch (e) { error = e.message }
  }

  function togglePin() { draft.pinned = !draft.pinned; scheduleSave() }

  // when the parent link changes, keep at most one and save
  function onLinkChange(kind, value) {
    if (kind === 'contact') { draft.contact_id = value || null; if (value) draft.project_id = null }
    else { draft.project_id = value || null; if (value) draft.contact_id = null }
    scheduleSave()
  }

  // ---- folders ----
  // folderEdit: null = closed. { id, name, parent_id } — id null ⇒ create, else rename/re-parent.
  let folderEdit     = $state(null)
  let folderToDelete = $state(null)

  function openNewFolder() { folderEdit = { id: null, name: '', parent_id: selectedFolder } }
  function openEditFolder(node) { folderEdit = { id: node.id, name: node.name, parent_id: node.parent_id ?? null } }

  // inclusive set of a folder's own id + all descendants (walk children via parent_id)
  function descendantIds(id) {
    const childrenOf = new Map()
    for (const f of folders) {
      if (!childrenOf.has(f.parent_id)) childrenOf.set(f.parent_id, [])
      childrenOf.get(f.parent_id).push(f.id)
    }
    const set = new Set([id])
    const stack = [id]
    while (stack.length) {
      const cur = stack.pop()
      for (const c of childrenOf.get(cur) || []) { set.add(c); stack.push(c) }
    }
    return set
  }

  // re-parent targets: exclude the folder itself and all its descendants (no cycles)
  const parentOptions = $derived.by(() => {
    if (!folderEdit || folderEdit.id == null) return folders
    const excl = descendantIds(folderEdit.id)
    return folders.filter((f) => !excl.has(f.id))
  })

  async function saveFolder(e) {
    e.preventDefault()
    const name = folderEdit.name.trim()
    if (!name) return
    // Always send parent_id: the backend full-replaces it, so omitting would re-root the folder.
    const payload = { name, parent_id: folderEdit.parent_id }
    try {
      if (folderEdit.id == null) {
        const f = await api.post('/api/folders', payload)
        folders = [...folders, f]
        if (folderEdit.parent_id) expanded = new Set(expanded).add(folderEdit.parent_id)
      } else {
        await api.put('/api/folders/' + folderEdit.id, payload)
        await load() // reload so the derived tree / breadcrumbs stay consistent after a re-parent
      }
      folderEdit = null
    } catch (err) { error = err.message }
  }

  const deleteImpact = $derived(folderToDelete ? folderBlastRadius(folders, notes, folderToDelete.id) : null)

  async function confirmDeleteFolder() {
    const id = folderToDelete.id
    try {
      await api.del('/api/folders/' + id)
      folderToDelete = null
      await load() // reload: cascaded subfolders gone, notes fell to Unfiled
      if (draft && !notes.some((n) => n.id === draft.id)) draft = null
      if (selectedFolder === id) selectedFolder = null
    } catch (e) { error = e.message }
  }
</script>

<div class="flex min-h-[70vh] gap-4">
  <!-- Sidebar -->
  <aside class="w-64 shrink-0 rounded-sm border border-border bg-surface-2 p-3">
    <div class="mb-3 flex items-center gap-2">
      <input bind:value={query} placeholder="⌕ search…" class="{FIELD} py-1.5" />
    </div>
    <div class="mb-3 flex gap-2">
      <button onclick={newNote} class="h-7 flex-1 rounded-sm bg-accent px-2 font-mono text-[12px] font-bold text-on-accent transition glow-soft hover:brightness-110">+ note</button>
      <button onclick={openNewFolder} class="h-7 rounded-sm border border-border-2 px-2 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">+ folder</button>
    </div>

    {#if error}<p class="mb-2 rounded-sm border border-st-lost/30 bg-st-lost/10 px-2 py-1 font-mono text-[11px] text-st-lost">[ERR] {error}</p>{/if}

    {#if query.trim()}
      <p class="mb-1.5 font-mono text-[10px] uppercase tracking-wide text-faint">{searchHits.length} result(s)</p>
      {#each searchHits as n (n.id)}
        <button onclick={() => openNote(n)} class="block w-full truncate rounded-sm px-2 py-1 text-left font-mono text-[12px] transition hover:bg-surface {draft?.id === n.id ? 'text-accent' : 'text-muted'}">
          {n.pinned ? '★ ' : ''}{noteTitle(n)}
          <span class="block truncate font-mono text-[10px] text-faint">{folderPath(folders, n.folder_id) || 'unfiled'}</span>
        </button>
      {/each}
    {:else}
      {#each tree as node (node.id)}
        {@render folderNode(node, 0)}
      {/each}
      <!-- Unfiled -->
      <button onclick={() => (selectedFolder = null)} class="mt-2 flex w-full items-center gap-1 rounded-sm px-1 py-1 text-left font-mono text-[12px] {selectedFolder === null ? 'text-accent' : 'text-faint'}">⌁ unfiled <span class="ml-auto tabular-nums text-faint">{unfiled.length}</span></button>
      {#each unfiled as n (n.id)}
        {@render noteRow(n, 1)}
      {/each}
    {/if}
  </aside>

  <!-- Editor -->
  <section class="min-w-0 flex-1 rounded-sm border border-border bg-surface p-4">
    {#if !draft}
      <p class="font-mono text-[12px] text-faint">// select a note, or press + note</p>
    {:else}
      <input bind:value={draft.title} oninput={scheduleSave} onblur={persist} placeholder="title…" class="mb-2 w-full bg-transparent font-mono text-[15px] font-bold text-ink placeholder:text-faint focus:outline-none" />

      <!-- mobile toggle -->
      <div class="mb-2 flex gap-1 md:hidden">
        {#each ['edit', 'preview'] as v (v)}
          <button onclick={() => (mobileView = v)} class="rounded-sm border px-2 py-0.5 font-mono text-[11px] {mobileView === v ? 'border-accent text-accent' : 'border-border text-faint'}">[ {v} ]</button>
        {/each}
      </div>

      <div class="flex min-h-[45vh] gap-3">
        <textarea
          bind:value={draft.body}
          oninput={scheduleSave}
          onblur={persist}
          placeholder="# markdown…"
          class="{FIELD} min-h-[45vh] flex-1 resize-none leading-relaxed {mobileView === 'edit' ? '' : 'hidden'} md:block"
        ></textarea>
        <div class="prose-console min-h-[45vh] flex-1 overflow-auto rounded-sm border border-border bg-bg p-3 font-mono text-[13px] text-ink {mobileView === 'preview' ? '' : 'hidden'} md:block">
          {@html renderMarkdown(draft.body)}
        </div>
      </div>

      <!-- footer -->
      <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-border pt-3 font-mono text-[11px]">
        <label class="text-faint">folder
          <select bind:value={draft.folder_id} onchange={scheduleSave} class="ml-1 rounded-sm border border-border bg-surface-2 px-1.5 py-1 text-ink">
            <option value={null}>unfiled</option>
            {#each folders as f (f.id)}<option value={f.id}>{folderPath(folders, f.id)}</option>{/each}
          </select>
        </label>
        <label class="text-faint">@contact
          <select value={draft.contact_id ?? ''} onchange={(e) => onLinkChange('contact', e.target.value)} class="ml-1 rounded-sm border border-border bg-surface-2 px-1.5 py-1 text-ink">
            <option value="">—</option>
            {#each contacts as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </label>
        <label class="text-faint">#project
          <select value={draft.project_id ?? ''} onchange={(e) => onLinkChange('project', e.target.value)} class="ml-1 rounded-sm border border-border bg-surface-2 px-1.5 py-1 text-ink">
            <option value="">—</option>
            {#each projects as p (p.id)}<option value={p.id}>{p.title}</option>{/each}
          </select>
        </label>
        <button onclick={togglePin} class="rounded-sm border px-1.5 py-1 {draft.pinned ? 'border-accent-dim text-accent' : 'border-border text-faint'}">{draft.pinned ? '★ pinned' : '☆ pin'}</button>
        <button onclick={deleteNote} class="rounded-sm border border-border px-1.5 py-1 text-faint transition hover:border-st-lost/50 hover:text-st-lost">delete</button>
        <span class="ml-auto text-faint">{saveStatus === 'saving' ? 'saving…' : saveStatus === 'saved' ? 'saved ✓' : ''}</span>
      </div>
    {/if}
  </section>
</div>

<!-- recursive folder node -->
{#snippet folderNode(node, depth)}
  <div class="flex items-center gap-1" style="padding-left:{depth * 12}px">
    <button onclick={() => toggle(node.id)} class="font-mono text-[11px] text-faint">{node.children.length || notesIn(node.id).length ? (expanded.has(node.id) ? '▾' : '▸') : '·'}</button>
    <button onclick={() => (selectedFolder = node.id)} class="flex-1 truncate text-left font-mono text-[12px] {selectedFolder === node.id ? 'text-accent' : 'text-muted'}">{node.name}</button>
    <button onclick={() => openEditFolder(node)} aria-label="Edit folder" class="font-mono text-[12px] text-faint transition hover:text-accent">✎</button>
    <button onclick={() => (folderToDelete = node)} aria-label="Delete folder" class="font-mono text-[12px] text-faint transition hover:text-st-lost">×</button>
  </div>
  {#if expanded.has(node.id)}
    {#each node.children as child (child.id)}{@render folderNode(child, depth + 1)}{/each}
    {#each notesIn(node.id) as n (n.id)}{@render noteRow(n, depth + 1)}{/each}
  {/if}
{/snippet}

{#snippet noteRow(n, depth)}
  <button onclick={() => openNote(n)} style="padding-left:{depth * 12 + 16}px" class="block w-full truncate rounded-sm py-1 pr-2 text-left font-mono text-[12px] transition hover:bg-surface {draft?.id === n.id ? 'text-accent' : 'text-muted'}">
    {n.pinned ? '★ ' : ''}{noteTitle(n)}
  </button>
{/snippet}

{#if folderEdit}
  <Modal title={folderEdit.id == null ? 'New folder' : 'Edit folder'} onclose={() => (folderEdit = null)}>
    <form onsubmit={saveFolder} class="flex flex-col gap-3">
      <input bind:value={folderEdit.name} placeholder="folder name…" class={FIELD} />
      <label class="font-mono text-[11px] text-faint">parent
        <select bind:value={folderEdit.parent_id} class="{FIELD} mt-1">
          <option value={null}>— (top level)</option>
          {#each parentOptions as f (f.id)}<option value={f.id}>{folderPath(folders, f.id)}</option>{/each}
        </select>
      </label>
      <button type="submit" class="h-9 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">{folderEdit.id == null ? 'create' : 'save'}</button>
    </form>
  </Modal>
{/if}

{#if folderToDelete}
  <Modal title="Delete folder" onclose={() => (folderToDelete = null)}>
    <p class="mb-4 font-mono text-[13px] text-ink">Delete <span class="text-accent">{folderToDelete.name}</span>? This removes {deleteImpact.subfolders} subfolder(s) and moves {deleteImpact.notes} note(s) to unfiled.</p>
    <div class="flex gap-2">
      <button onclick={confirmDeleteFolder} class="h-9 flex-1 rounded-sm border border-st-lost/50 font-mono text-[13px] text-st-lost transition hover:bg-st-lost/10">delete</button>
      <button onclick={() => (folderToDelete = null)} class="h-9 flex-1 rounded-sm border border-border font-mono text-[13px] text-muted">cancel</button>
    </div>
  </Modal>
{/if}
