<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let contacts = $state([])
  let error = $state('')
  let editing = $state(null) // null = closed; {} = new; {...} = edit existing

  const companies = $derived(contacts.filter((c) => c.kind === 'company'))

  async function load() {
    error = ''
    try { contacts = await api.get('/api/contacts') } catch (e) { error = e.message }
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
    try {
      await api.del(`/api/contacts/${c.id}`)
      await load()
    } catch (e) { error = e.message }
  }

  $effect(() => { load() })
</script>

<div class="rise mb-5 flex items-center justify-between">
  <p class="font-mono text-[12px] text-faint"><span class="text-accent-dim">//</span> manage clients and contacts</p>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> New contact
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<div class="rise max-md:overflow-x-auto rounded-sm border border-border bg-surface" style="animation-delay:40ms">
  <table class="w-full border-collapse">
    <thead>
      <tr class="border-b border-border">
        <th class="px-4 py-2 text-left font-mono text-[11px] text-faint">&gt; name</th>
        <th class="px-4 py-2 text-left font-mono text-[11px] text-faint">&gt; kind</th>
        <th class="px-4 py-2 text-left font-mono text-[11px] text-faint">&gt; email</th>
        <th class="px-4 py-2 text-left font-mono text-[11px] text-faint">&gt; phone</th>
        <th class="px-4 py-2"></th>
      </tr>
    </thead>
    <tbody>
      {#each contacts as c (c.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
        <tr
          onclick={() => push('/contacts/' + c.id)}
          class="cursor-pointer border-b border-border/60 transition-colors duration-100 last:border-0 hover:bg-surface-2"
        >
          <td class="px-4 py-2.5 font-mono text-[13px] font-medium text-ink">{c.name}</td>
          <td class="px-4 py-2.5 font-mono text-[12px] text-muted">[ {c.kind} ]</td>
          <td class="px-4 py-2.5 font-mono text-[12px] text-muted">{c.email ?? '—'}</td>
          <td class="px-4 py-2.5 font-mono text-[12px] text-muted">{c.phone ?? '—'}</td>
          <td class="px-4 py-2.5 text-right">
            <button
              onclick={(e) => { e.stopPropagation(); openEdit(c) }}
              class="h-7 rounded-sm border border-border-2 px-2.5 font-mono text-[12px] text-muted transition hover:border-accent-dim hover:text-ink"
            >edit</button>
            <button
              onclick={(e) => { e.stopPropagation(); remove(c) }}
              class="ml-1.5 h-7 rounded-sm border border-st-lost/40 px-2.5 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10"
            >delete</button>
          </td>
        </tr>
      {:else}
        <tr>
          <td colspan="5" class="px-4 py-6 font-mono text-[12px] text-faint">no contacts yet</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if editing}
  <Modal title={editing.id ? 'Edit contact' : 'New contact'} onclose={() => (editing = null)}>
    <form onsubmit={save} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Kind</p>
        <select bind:value={editing.kind} class={FIELD}>
          <option value="person">Person</option>
          <option value="company">Company</option>
        </select>
      </div>
      <div>
        <p class="label mb-1.5">Name</p>
        <input bind:value={editing.name} placeholder="Name" required class={FIELD} />
      </div>
      <div>
        <p class="label mb-1.5">Email</p>
        <input bind:value={editing.email} placeholder="Email" class={FIELD} />
      </div>
      <div>
        <p class="label mb-1.5">Phone</p>
        <input bind:value={editing.phone} placeholder="Phone" class={FIELD} />
      </div>
      {#if editing.kind === 'person'}
        <div>
          <p class="label mb-1.5">Company</p>
          <select bind:value={editing.company_id} class={FIELD}>
            <option value={null}>— No company —</option>
            {#each companies as co}
              {#if co.id !== editing.id}<option value={co.id}>{co.name}</option>{/if}
            {/each}
          </select>
        </div>
      {/if}
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
    </form>
  </Modal>
{/if}
