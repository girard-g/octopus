<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

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
    if (!confirm(`Delete ${c.name}?`)) return
    await api.del(`/api/contacts/${c.id}`)
    await load()
  }

  $effect(() => { load() })
</script>

<div class="mb-4 flex items-center justify-between">
  <h1 class="text-2xl font-bold text-slate-800">Contacts</h1>
  <button onclick={openNew} class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white">New contact</button>
</div>
{#if error}<p class="mb-3 text-red-600">{error}</p>{/if}

<table class="w-full bg-white text-sm shadow-sm">
  <thead class="border-b text-left text-slate-500">
    <tr><th class="p-2">Name</th><th class="p-2">Kind</th><th class="p-2">Email</th><th class="p-2">Phone</th><th class="p-2"></th></tr>
  </thead>
  <tbody>
    {#each contacts as c}
      <tr class="border-b last:border-0">
        <td class="p-2 font-medium">{c.name}</td>
        <td class="p-2 text-slate-500">{c.kind}</td>
        <td class="p-2">{c.email ?? ''}</td>
        <td class="p-2">{c.phone ?? ''}</td>
        <td class="p-2 text-right">
          <button onclick={() => openEdit(c)} class="text-blue-600 hover:underline">Edit</button>
          <button onclick={() => remove(c)} class="ml-2 text-red-600 hover:underline">Delete</button>
        </td>
      </tr>
    {:else}
      <tr><td colspan="5" class="p-3 text-slate-400">No contacts yet.</td></tr>
    {/each}
  </tbody>
</table>

{#if editing}
  <Modal title={editing.id ? 'Edit contact' : 'New contact'} onclose={() => (editing = null)}>
    <form onsubmit={save} class="space-y-3">
      <select bind:value={editing.kind} class="w-full rounded border border-slate-300 px-2 py-1.5">
        <option value="person">Person</option>
        <option value="company">Company</option>
      </select>
      <input bind:value={editing.name} placeholder="Name" required class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <input bind:value={editing.email} placeholder="Email" class="w-full rounded border border-slate-300 px-2 py-1.5" />
      <input bind:value={editing.phone} placeholder="Phone" class="w-full rounded border border-slate-300 px-2 py-1.5" />
      {#if editing.kind === 'person'}
        <select bind:value={editing.company_id} class="w-full rounded border border-slate-300 px-2 py-1.5">
          <option value={null}>— No company —</option>
          {#each companies as co}
            {#if co.id !== editing.id}<option value={co.id}>{co.name}</option>{/if}
          {/each}
        </select>
      {/if}
      <button class="w-full rounded bg-blue-600 py-2 text-white">Save</button>
    </form>
  </Modal>
{/if}
