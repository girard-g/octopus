<script>
  import { push } from 'svelte-spa-router'
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'

  let filter = $state('active')            // 'active' | 'archived'
  let projects = $state([])
  let contacts = $state([])
  let error = $state('')
  let creating = $state(null)              // { title } | null

  const contactsById = $derived(Object.fromEntries(contacts.map((c) => [c.id, c.name])))

  async function load() {
    error = ''
    try {
      ;[projects, contacts] = await Promise.all([
        api.get('/api/projects?status=' + filter),
        api.get('/api/contacts'),
      ])
    } catch (e) { error = e.message }
  }

  function openNew() { creating = { title: '' } }
  async function createProject(e) {
    e.preventDefault()
    if (!creating.title.trim()) return
    try {
      await api.post('/api/projects', { title: creating.title.trim() })
      creating = null
      filter = 'active'   // new projects default to active — show it
      await load()
    } catch (err) { error = err.message }
  }

  $effect(() => { filter; load() })
</script>

<div class="rise mb-5 flex items-center justify-between">
  <div class="flex gap-1.5">
    {#each ['active', 'archived'] as f}
      <button
        onclick={() => (filter = f)}
        class="h-8 rounded-sm border px-3 font-mono text-[12px] lowercase transition"
        class:border-accent={filter === f}
        class:text-accent={filter === f}
        class:glow-soft={filter === f}
        class:border-border={filter !== f}
        class:text-faint={filter !== f}
      >{f}</button>
    {/each}
  </div>
  <button
    onclick={openNew}
    class="inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> New project
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<div class="rise grid grid-cols-2 gap-3 md:grid-cols-3" style="animation-delay:40ms">
  {#each projects as p (p.id)}
    <button
      onclick={() => push('/projects/' + p.id)}
      class="group relative overflow-hidden rounded-sm border border-border bg-surface p-4 text-left transition-all duration-100 hover:-translate-y-px hover:border-accent-dim hover:shadow-[0_0_14px_rgba(62,245,196,0.12)]"
    >
      <div class="font-mono text-[13px] font-medium leading-snug text-ink">{p.title}</div>
      <div class="mt-1 truncate font-mono text-[11px] text-faint">{contactsById[p.contact_id] ?? '—'}</div>
      <div class="mt-3 font-mono text-[11px] tabular-nums text-faint"><span class="text-accent-dim">&gt;</span> {p.task_count} task{p.task_count === 1 ? '' : 's'}</div>
    </button>
  {:else}
    <p class="col-span-full py-10 text-center font-mono text-[13px] text-faint">// no {filter} projects</p>
  {/each}
</div>

{#if creating}
  <Modal title="New project" onclose={() => (creating = null)}>
    <form onsubmit={createProject} class="flex flex-col gap-3">
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={creating.title} placeholder="Project title" required class={FIELD} />
      </div>
      <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Create project</button>
    </form>
  </Modal>
{/if}
