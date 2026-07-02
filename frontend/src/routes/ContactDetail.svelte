<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import ContactForm from '../lib/components/ContactForm.svelte'
  import { eventsForContact, buildTimeline, lastTouch, humanizeSince, companyRoster, companyName } from '../lib/contacts.js'

  const PROJECT_STATUS_TEXT = { active: 'text-st-active', archived: 'text-muted' }

  let { params } = $props()
  const id = $derived(params.id)

  let contact = $state(null)
  let contacts = $state([])
  let projects = $state([])
  let notes = $state([])
  let events = $state([])
  let error = $state('')
  let newNote = $state('')
  let noteBusy = $state(false)
  let editing = $state(null)
  let composer // textarea ref (bind:this)

  const myProjects = $derived(projects.filter((p) => p.contact_id === id))
  const myEvents = $derived(eventsForContact(events, id))
  const timeline = $derived(buildTimeline(notes, myEvents, new Date()))
  const activeCount = $derived(myProjects.filter((p) => p.status === 'active').length)
  const touch = $derived(humanizeSince(lastTouch(notes, myEvents, new Date()), new Date()))
  const roster = $derived(contact?.kind === 'company' ? companyRoster(contacts, id) : [])
  const company = $derived(contact?.company_id ? companyName(contacts, contact.company_id) : null)
  const companies = $derived(contacts.filter((c) => c.kind === 'company'))

  async function load() {
    error = ''
    try {
      // ponytail: no ?contact_id filters on /api/projects or /api/events — filter client-side.
      const [c, cs, ps, ns, es] = await Promise.all([
        api.get('/api/contacts/' + id),
        api.get('/api/contacts'),
        api.get('/api/projects'),
        api.get('/api/notes?contact_id=' + id),
        api.get('/api/events'),
      ])
      contact = c; contacts = cs; projects = ps; notes = ns; events = es
    } catch (e) { error = e.message }
  }

  $effect(() => { if (id) load() })

  function openEdit() { editing = { ...contact } }
  async function saveEdit(e) {
    e.preventDefault()
    const body = {
      kind: editing.kind,
      name: editing.name,
      email: editing.email || null,
      phone: editing.phone || null,
      company_id: editing.company_id || null,
    }
    try { await api.put('/api/contacts/' + id, body); editing = null; await load() }
    catch (err) { error = err.message }
  }

  async function deleteContact() {
    if (!confirm(`Delete ${contact?.name ?? 'this contact'}? This also deletes their projects and tasks.`)) return
    try { await api.del('/api/contacts/' + id); push('/contacts') } catch (e) { error = e.message }
  }

  async function addNote(e) {
    e?.preventDefault()
    const b = newNote.trim()
    if (!b) return
    noteBusy = true
    try { await api.post('/api/notes', { body: b, contact_id: id }); newNote = ''; await load() }
    catch (err) { error = err.message } finally { noteBusy = false }
  }

  function composerKey(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') { e.preventDefault(); addNote() }
  }

  async function deleteNote(noteId) {
    try { await api.del('/api/notes/' + noteId); await load() } catch (e) { error = e.message }
  }

  const fmtDay = (iso) => new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  const fmtWhen = (iso) => new Date(iso).toLocaleString(undefined, { weekday: 'short', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
</script>

<!-- Header -->
<div class="rise mb-6">
  <div class="mb-2 flex flex-wrap items-center gap-3">
    <button onclick={() => push('/contacts')} class="font-mono text-[12px] text-faint transition hover:text-accent">&lt; contacts</button>
    <span class="font-mono text-[12px] text-faint">/</span>
    <h2 class="font-mono text-[15px] font-bold text-ink">{contact?.name ?? '…'}</h2>
    {#if contact}
      <span class="font-mono text-[11px] font-bold uppercase tracking-wider {contact.kind === 'company' ? 'text-accent' : 'text-st-lead'}">[ {contact.kind} ]</span>
    {/if}
    <div class="ml-auto flex flex-wrap gap-1.5">
      {#if contact?.email}
        <a href={'mailto:' + contact.email} class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">✉ email</a>
      {/if}
      {#if contact?.phone}
        <a href={'tel:' + contact.phone} class="inline-flex h-8 items-center gap-1.5 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">☏ call</a>
      {/if}
      <button onclick={openEdit} class="h-8 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink">edit</button>
      <button onclick={deleteContact} class="h-8 rounded-sm border border-st-lost/40 px-2.5 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10">delete</button>
    </div>
  </div>

  {#if contact}
    <p class="font-mono text-[12px]">
      {#if contact.kind === 'person'}
        {#if company}
          <button onclick={() => push('/contacts/' + contact.company_id)} class="text-muted transition hover:text-accent">⌂ {company}</button>
        {:else}<span class="text-faint">⌂ —</span>{/if}
        <span class="mx-1.5 text-border-2">·</span>
      {/if}
      <span class="text-muted">{contact.email ?? '—'}</span>
      <span class="mx-1.5 text-border-2">·</span>
      <span class="text-faint">{contact.phone ?? '—'}</span>
    </p>
  {/if}

  {#if error}
    <p class="mt-3 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>
  {/if}
</div>

<!-- Stat strip -->
{#if contact}
  <div class="rise mb-8 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-sm border border-border bg-surface px-4 py-2.5 font-mono text-[12px]" style="animation-delay:30ms">
    <span class="text-accent">◈</span>
    <span class="text-ink">{activeCount}<span class="text-faint"> active</span></span>
    <span class="text-border-2">·</span>
    <span class="text-ink">{myProjects.length}<span class="text-faint"> projects</span></span>
    <span class="text-border-2">·</span>
    <span class="text-ink">{notes.length}<span class="text-faint"> notes</span></span>
    <span class="text-border-2">·</span>
    <span class="text-faint">last touch {touch}</span>
  </div>
{/if}

<div class="grid gap-8 lg:grid-cols-[1fr_260px]">
  <!-- Timeline -->
  <div class="rise order-2 lg:order-1" style="animation-delay:80ms">
    <div class="mb-3 flex items-center justify-between gap-2">
      <p class="font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> timeline</p>
      <div class="flex items-center gap-2 font-mono text-[11px]">
        <span class="text-faint">log ▸</span>
        <button onclick={() => composer?.focus()} class="text-muted transition hover:text-accent">note</button>
        <!-- ponytail: navigate-only, no contact prefill -->
        <button onclick={() => push('/calendar')} class="text-muted transition hover:text-accent">event</button>
        <button onclick={() => push('/projects')} class="text-muted transition hover:text-accent">project</button>
      </div>
    </div>

    <form onsubmit={addNote} class="mb-4 flex flex-col gap-2">
      <textarea
        bind:this={composer}
        bind:value={newNote}
        onkeydown={composerKey}
        placeholder="log a note…  (⌘⏎ to add)"
        rows="2"
        class="w-full resize-none rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none"
      ></textarea>
      <button type="submit" disabled={noteBusy} class="h-8 self-start rounded-sm bg-accent px-3 font-mono text-[12px] font-bold text-on-accent transition glow-soft hover:brightness-110 disabled:opacity-50">add note</button>
    </form>

    {#if timeline.upcoming.length === 0 && timeline.history.length === 0}
      <p class="font-mono text-[12px] text-faint">· start of history ·</p>
    {:else}
      <div class="flex flex-col gap-2">
        {#if timeline.upcoming.length}
          <p class="label mt-1 text-accent">⧗ upcoming</p>
          {#each timeline.upcoming as it (it.id)}
            <div class="flex items-start gap-3 rounded-sm border border-accent-dim/40 bg-surface px-3 py-2">
              <span class="mt-0.5 text-[12px] text-accent">◆</span>
              <div class="min-w-0 flex-1">
                <p class="font-mono text-[13px] text-ink">{it.text}</p>
                <p class="mt-0.5 font-mono text-[11px] text-faint">{fmtWhen(it.when)}</p>
              </div>
              <button onclick={() => push('/calendar')} aria-label="Open calendar" class="shrink-0 font-mono text-[13px] text-faint transition hover:text-accent">↗</button>
            </div>
          {/each}
          <div class="my-1 border-t border-border/60"></div>
        {/if}

        {#each timeline.history as it (it.id)}
          <div class="group flex items-start gap-3 rounded-sm border border-border bg-surface px-3 py-2">
            <span class="mt-0.5 text-[12px] {it.type === 'event' ? 'text-st-lead' : 'text-accent'}">{it.type === 'event' ? '◆' : '●'}</span>
            <div class="min-w-0 flex-1">
              <pre class="whitespace-pre-wrap break-words font-mono text-[13px] text-ink">{it.text}</pre>
              <p class="mt-0.5 font-mono text-[11px] text-faint">{fmtDay(it.when)}</p>
            </div>
            {#if it.type === 'note'}
              <button onclick={() => deleteNote(it.id)} aria-label="Delete note" class="shrink-0 font-mono text-[15px] leading-none text-faint transition hover:text-st-lost">×</button>
            {:else}
              <button onclick={() => push('/calendar')} aria-label="Open calendar" class="shrink-0 font-mono text-[13px] text-faint transition hover:text-accent">↗</button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Projects + roster -->
  <div class="order-1 flex flex-col gap-8 lg:order-2">
    <div class="rise" style="animation-delay:50ms">
      <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> projects</p>
      {#if myProjects.length === 0}
        <p class="font-mono text-[12px] text-faint">no projects</p>
      {:else}
        <div class="flex flex-col gap-2">
          {#each myProjects as p (p.id)}
            <button onclick={() => push('/projects/' + p.id)} class="flex items-center gap-2.5 rounded-sm border border-border bg-surface px-3 py-2 text-left transition hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]">
              <span class="font-mono text-[10px] font-bold uppercase tracking-wider {PROJECT_STATUS_TEXT[p.status] ?? 'text-muted'}">[ {p.status} ]</span>
              <span class="min-w-0 flex-1 truncate font-mono text-[13px] text-ink">{p.title}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    {#if contact?.kind === 'company'}
      <div class="rise" style="animation-delay:70ms">
        <p class="mb-3 font-mono text-[12px] text-faint"><span class="text-accent glow-text">&gt;</span> roster</p>
        {#if roster.length === 0}
          <p class="font-mono text-[12px] text-faint">no people yet</p>
        {:else}
          <div class="flex flex-col gap-2">
            {#each roster as person (person.id)}
              <button onclick={() => push('/contacts/' + person.id)} class="flex flex-col items-start rounded-sm border border-border bg-surface px-3 py-2 text-left transition hover:border-accent-dim">
                <span class="font-mono text-[13px] text-ink">{person.name}</span>
                {#if person.email}<span class="font-mono text-[11px] text-faint">{person.email}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if editing}
  <ContactForm value={editing} {companies} onclose={() => (editing = null)} onsubmit={saveEdit} />
{/if}
