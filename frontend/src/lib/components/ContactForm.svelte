<script>
  import Modal from './Modal.svelte'
  let { value, companies, onclose, onsubmit } = $props()
  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
</script>

<Modal title={value.id ? 'Edit contact' : 'New contact'} {onclose}>
  <form onsubmit={onsubmit} class="flex flex-col gap-3">
    <div>
      <p class="label mb-1.5">Kind</p>
      <select bind:value={value.kind} class={FIELD}>
        <option value="person">Person</option>
        <option value="company">Company</option>
      </select>
    </div>
    <div>
      <p class="label mb-1.5">Name</p>
      <input bind:value={value.name} placeholder="Name" required class={FIELD} />
    </div>
    <div>
      <p class="label mb-1.5">Email</p>
      <input bind:value={value.email} placeholder="Email" class={FIELD} />
    </div>
    <div>
      <p class="label mb-1.5">Phone</p>
      <input bind:value={value.phone} placeholder="Phone" class={FIELD} />
    </div>
    {#if value.kind === 'person'}
      <div>
        <p class="label mb-1.5">Company</p>
        <select bind:value={value.company_id} class={FIELD}>
          <option value={null}>— No company —</option>
          {#each companies as co}
            {#if co.id !== value.id}<option value={co.id}>{co.name}</option>{/if}
          {/each}
        </select>
      </div>
    {/if}
    <button class="mt-1 h-9 w-full rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">Save</button>
  </form>
</Modal>
