<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let notes    = $state([])
  let contacts = $state([])
  let projects = $state([])
  let error    = $state('')
  let filter   = $state('all') // 'all' | 'contacts' | 'projects'
  let showNew  = $state(false)

  // modal state
  let newParentType = $state('contact')
  let newParentId   = $state('')
  let newBody       = $state('')
  let newError      = $state('')
  let newBusy       = $state(false)

  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))
  const projectsById = $derived(Object.fromEntries(projects.map((p) => [p.id, p.title])))

  const filtered = $derived(
    filter === 'contacts' ? notes.filter((n) => n.contact_id) :
    filter === 'projects' ? notes.filter((n) => n.project_id) :
    notes
  )

  async function load() {
    error = ''
    try {
      ;[notes, contacts, projects] = await Promise.all([
        api.get('/api/notes'),
        api.get('/api/contacts'),
        api.get('/api/projects'),
      ])
    } catch (e) { error = e.message }
  }

  $effect(() => { load() })

  async function deleteNote(id, ev) {
    ev.stopPropagation()
    try { await api.del('/api/notes/' + id); await load() }
    catch (e) { error = e.message }
  }

  function navigateParent(n) {
    if (n.contact_id) push('/contacts/' + n.contact_id)
    else if (n.project_id) push('/projects/' + n.project_id)
  }

  function openNew() {
    newParentType = 'contact'
    newParentId = contacts[0]?.id ?? ''
    newBody = ''
    newError = ''
    showNew = true
  }

  function onParentTypeChange() {
    newParentId = newParentType === 'contact'
      ? (contacts[0]?.id ?? '')
      : (projects[0]?.id ?? '')
  }

  async function saveNew(e) {
    e.preventDefault()
    newError = ''
    const b = newBody.trim()
    if (!b) { newError = 'body is required'; return }
    if (!newParentId) { newError = 'select a parent'; return }
    newBusy = true
    try {
      const payload = { body: b }
      if (newParentType === 'contact') payload.contact_id = newParentId
      else payload.project_id = newParentId
      await api.post('/api/notes', payload)
      showNew = false
      await load()
    } catch (err) { newError = err.message }
    finally { newBusy = false }
  }
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="flex flex-wrap items-center gap-4">
    <div>
      <h2 class="font-mono text-[15px] font-bold text-ink">
        <span class="text-accent glow-text">&gt;</span> notes
      </h2>
      <p class="mt-0.5 font-mono text-[12px] text-faint">// all notes across clients and projects</p>
    </div>
    <button
      onclick={openNew}
      class="ml-auto h-8 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
    >+ new note</button>
  </div>

  {#if error}
    <p class="mt-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Filter chips -->
<div class="rise mb-5 flex items-center gap-2" style="animation-delay:40ms">
  {#each [['all', 'all'], ['contacts', 'contacts'], ['projects', 'projects']] as [val, label] (val)}
    <button
      onclick={() => (filter = val)}
      class="rounded-sm border px-3 py-1 font-mono text-[12px] transition {filter === val
        ? 'border-accent text-accent shadow-[0_0_8px_rgba(62,245,196,0.18)]'
        : 'border-border text-faint hover:border-border-2 hover:text-muted'}"
    >[ {label} ]</button>
  {/each}
  <span class="ml-auto font-mono text-[12px] text-faint tabular-nums">[{filtered.length}]</span>
</div>

<!-- Notes feed -->
<div class="rise flex flex-col gap-2" style="animation-delay:80ms">
  {#if filtered.length === 0}
    <p class="font-mono text-[12px] text-faint">no notes</p>
  {:else}
    {#each filtered as n (n.id)}
      {@const isContact = !!n.contact_id}
      {@const parentName = isContact
        ? (contactsById[n.contact_id] ?? '…')
        : (projectsById[n.project_id] ?? '…')}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        role="button"
        tabindex="0"
        onclick={() => navigateParent(n)}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') navigateParent(n) }}
        class="group cursor-pointer rounded-sm border border-border bg-surface p-3 transition-all hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)] focus-visible:border-accent focus-visible:outline-none focus-visible:shadow-[0_0_0_3px_rgba(62,245,196,0.14)]"
      >
        <div class="flex items-start gap-2">
          <pre class="min-w-0 flex-1 whitespace-pre-wrap break-words font-mono text-[13px] text-ink">{n.body}</pre>
          <button
            onclick={(e) => deleteNote(n.id, e)}
            aria-label="Delete note"
            class="shrink-0 font-mono text-[16px] leading-none text-faint transition hover:text-st-lost"
          >×</button>
        </div>
        <div class="mt-2 flex items-center gap-3">
          <button
            onclick={(e) => { e.stopPropagation(); navigateParent(n) }}
            class="rounded-sm border border-border-2 px-1.5 py-0.5 font-mono text-[11px] transition hover:border-accent-dim hover:text-accent"
          >{isContact ? '@' : '#'} {parentName}</button>
          <span class="font-mono text-[11px] text-faint tabular-nums">{new Date(n.created_at).toLocaleDateString()}</span>
        </div>
      </div>
    {/each}
  {/if}
</div>

<!-- New-note modal -->
{#if showNew}
  <Modal title="New note" onclose={() => (showNew = false)}>
    <form onsubmit={saveNew} class="flex flex-col gap-3">
      {#if newError}
        <p class="rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {newError}</p>
      {/if}
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">parent type</p>
        <select
          bind:value={newParentType}
          onchange={onParentTypeChange}
          class={FIELD}
        >
          <option value="contact">Contact</option>
          <option value="project">Project</option>
        </select>
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">{newParentType === 'contact' ? 'contact' : 'project'}</p>
        <select bind:value={newParentId} class={FIELD}>
          {#if newParentType === 'contact'}
            {#each contacts as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          {:else}
            {#each projects as p (p.id)}<option value={p.id}>{p.title}</option>{/each}
          {/if}
        </select>
      </div>
      <div>
        <p class="mb-1.5 font-mono text-[11px] text-faint">body</p>
        <textarea
          bind:value={newBody}
          placeholder="note body…"
          rows="4"
          class="{FIELD} resize-none"
        ></textarea>
      </div>
      <button
        type="submit"
        disabled={newBusy}
        class="h-9 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50"
      >save note</button>
    </form>
  </Modal>
{/if}
