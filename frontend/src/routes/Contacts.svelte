<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import ContactForm from '../lib/components/ContactForm.svelte'
  import { filterContacts, projectCountsByContact } from '../lib/contacts.js'

  const KINDS = ['all', 'person', 'company']

  let contacts = $state([])
  let projects = $state([])
  let error = $state('')
  let editing = $state(null) // null=closed; {}=new; {...}=edit
  let query = $state('')
  let kind = $state('all')

  const companies = $derived(contacts.filter((c) => c.kind === 'company'))
  const nameById = $derived(new Map(contacts.map((c) => [c.id, c.name])))
  const counts = $derived(projectCountsByContact(projects))
  const shown = $derived(filterContacts(contacts, query, kind))

  async function load() {
    error = ''
    try {
      const [cs, ps] = await Promise.all([api.get('/api/contacts'), api.get('/api/projects')])
      contacts = cs
      projects = ps
    } catch (e) { error = e.message }
  }

  function openNew() { editing = { kind: 'person', name: '', email: '', phone: '', company_id: null } }
  function openEdit(c) { editing = { ...c } }

  async function save(e) {
    e.preventDefault()
    const body = {
      kind: editing.kind,
      name: editing.name,
      email: editing.email || null,
      phone: editing.phone || null,
      company_id: editing.company_id || null,
    }
    try {
      if (editing.id) await api.put(`/api/contacts/${editing.id}`, body)
      else await api.post('/api/contacts', body)
      editing = null
      await load()
    } catch (e) { error = e.message }
  }

  async function remove(c) {
    if (!confirm(`Delete ${c.name}? This also deletes their projects and tasks.`)) return
    try { await api.del(`/api/contacts/${c.id}`); await load() } catch (e) { error = e.message }
  }

  function copy(text) { navigator.clipboard?.writeText(text) }

  $effect(() => { load() })
</script>

<div class="rise mb-5 flex items-center justify-between gap-3">
  <p class="font-mono text-[12px] text-faint"><span class="text-accent-dim">//</span> manage clients and contacts</p>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  ><span class="text-[15px] leading-none">+</span> New contact</button>
</div>

<!-- Command bar: search + kind toggle -->
<div class="rise mb-5 flex flex-wrap items-center gap-3" style="animation-delay:30ms">
  <div class="relative min-w-[220px] flex-1">
    <span class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 font-mono text-[13px] text-accent">&gt;</span>
    <input
      bind:value={query}
      placeholder="search name, email, company…"
      class="w-full rounded-sm border border-border bg-surface-2 py-2 pl-7 pr-2.5 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none"
    />
  </div>
  <div class="flex rounded-sm border border-border">
    {#each KINDS as k}
      <button
        onclick={() => (kind = k)}
        class="h-8 px-3 font-mono text-[12px] transition-colors {kind === k ? 'bg-surface-2 text-accent' : 'text-faint hover:text-ink'}"
      >{k}</button>
    {/each}
  </div>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

{#if shown.length === 0}
  <p class="rise font-mono text-[12px] text-faint" style="animation-delay:60ms">no contacts{query || kind !== 'all' ? ' match' : ' yet'}</p>
{:else}
  <div class="rise grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3" style="animation-delay:60ms">
    {#each shown as c (c.id)}
      {@const ct = counts.get(c.id) ?? { active: 0, total: 0 }}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        onclick={() => push('/contacts/' + c.id)}
        role="button"
        tabindex="0"
        onkeydown={(e) => { if (e.key === 'Enter') push('/contacts/' + c.id) }}
        class="group relative cursor-pointer rounded-sm border border-border bg-surface p-4 transition hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
      >
        <div class="absolute right-2 top-2 flex gap-1 opacity-0 transition group-hover:opacity-100">
          <button onclick={(e) => { e.stopPropagation(); openEdit(c) }} class="h-6 rounded-sm border border-border-2 px-2 font-mono text-[11px] text-muted transition hover:border-accent-dim hover:text-ink">edit</button>
          <button onclick={(e) => { e.stopPropagation(); remove(c) }} class="h-6 rounded-sm border border-st-lost/40 px-2 font-mono text-[11px] text-st-lost transition hover:bg-st-lost/10">del</button>
        </div>

        <span class="font-mono text-[11px] font-bold uppercase tracking-wider {c.kind === 'company' ? 'text-accent' : 'text-st-lead'}">[ {c.kind} ]</span>
        <p class="mt-1.5 font-mono text-[14px] font-medium text-ink">{c.name}</p>
        {#if c.kind === 'person'}
          <p class="mt-0.5 font-mono text-[12px] text-faint">⌂ {c.company_id ? (nameById.get(c.company_id) ?? '—') : '—'}</p>
        {/if}

        <div class="mt-2.5 flex flex-col gap-0.5 font-mono text-[12px]">
          <span class="text-muted">{c.email ?? '—'}</span>
          <span class="text-faint">{c.phone ?? '—'}</span>
        </div>

        <div class="mt-3 flex items-center justify-between border-t border-border/60 pt-2.5">
          <span class="font-mono text-[11px] text-faint">◈ {ct.active} active · {ct.total} total</span>
          <div class="flex items-center gap-1.5">
            {#if c.email}
              <a href={'mailto:' + c.email} onclick={(e) => e.stopPropagation()} aria-label="Email" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">✉</a>
            {/if}
            {#if c.phone}
              <a href={'tel:' + c.phone} onclick={(e) => e.stopPropagation()} aria-label="Call" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">☏</a>
            {/if}
            {#if c.email}
              <button onclick={(e) => { e.stopPropagation(); copy(c.email) }} aria-label="Copy email" class="grid h-6 w-6 place-items-center rounded-sm border border-border-2 text-[12px] text-muted transition hover:border-accent-dim hover:text-accent">⧉</button>
            {/if}
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

{#if editing}
  <ContactForm value={editing} {companies} onclose={() => (editing = null)} onsubmit={save} />
{/if}
