<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
  const PROJECT_STATUS_TEXT = {
    lead: 'text-st-lead', proposal: 'text-st-proposal', active: 'text-st-active',
    done: 'text-st-done', lost: 'text-st-lost',
  }

  let { params } = $props()
  const id = $derived(params.id)

  let contact  = $state(null)
  let projects = $state([])
  let notes    = $state([])
  let error    = $state('')
  let newNote  = $state('')
  let noteBusy = $state(false)

  async function load() {
    error = ''
    try {
      const [c, allProjects, n] = await Promise.all([
        api.get('/api/contacts/' + id),
        api.get('/api/projects'),
        api.get('/api/notes?contact_id=' + id),
      ])
      contact = c
      // ponytail: no ?contact_id filter on /api/projects, filter client-side
      projects = allProjects.filter((p) => p.contact_id === id)
      notes = n
    } catch (e) { error = e.message }
  }

  $effect(() => { if (id) load() })

  async function deleteContact() {
    if (!confirm(`Delete ${contact?.name ?? 'this contact'}? This also deletes their projects and tasks.`)) return
    try { await api.del('/api/contacts/' + id); push('/contacts') }
    catch (e) { error = e.message }
  }

  async function addNote(e) {
    e.preventDefault()
    const b = newNote.trim()
    if (!b) return
    noteBusy = true
    try {
      await api.post('/api/notes', { body: b, contact_id: id })
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
      onclick={() => push('/contacts')}
      class="font-mono text-[12px] text-faint transition hover:text-accent"
    >&lt; contacts</button>
    <span class="font-mono text-[12px] text-faint">/</span>
    <h2 class="font-mono text-[15px] font-bold text-ink">{contact?.name ?? '…'}</h2>
    {#if contact}
      <span class="font-mono text-[11px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[contact.kind] ?? 'text-muted'}">[ {contact.kind} ]</span>
      {#if contact.email}
        <span class="font-mono text-[12px] text-muted">{contact.email}</span>
      {/if}
      {#if contact.phone}
        <span class="font-mono text-[12px] text-faint">{contact.phone}</span>
      {/if}
    {/if}
    <div class="ml-auto flex gap-2">
      <button
        onclick={deleteContact}
        class="h-8 rounded-sm border border-st-lost/40 px-3 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10"
      >delete</button>
    </div>
  </div>

  {#if error}
    <p class="mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Projects panel -->
<div class="rise mb-8" style="animation-delay:40ms">
  <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> projects</p>
  {#if projects.length === 0}
    <p class="font-mono text-[12px] text-faint">no projects</p>
  {:else}
    <div class="flex flex-col gap-2">
      {#each projects as p (p.id)}
        <button
          onclick={() => push('/projects/' + p.id)}
          class="flex items-center gap-3 rounded-sm border border-border bg-surface px-4 py-2.5 text-left transition hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
        >
          <span class="font-mono text-[11px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[p.status] ?? 'text-muted'}">[ {p.status} ]</span>
          <span class="font-mono text-[13px] text-ink">{p.title}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<!-- Notes panel -->
<div class="rise" style="animation-delay:80ms">
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
