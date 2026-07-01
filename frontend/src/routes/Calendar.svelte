<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'
  import { monthMatrix, monthRange, eventsByDay, fmtTime, toISODate, generateOccurrences } from '../lib/calendar.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
  const DOW = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

  // Current month state — defaults to real current month (browser only)
  const now = new Date()
  let year = $state(now.getFullYear())
  let monthIndex = $state(now.getMonth())

  let events = $state([])
  let projects = $state([])
  let contacts = $state([])
  let error = $state('')
  let modal = $state(null) // null | { mode: 'new'|'edit', ev: {...} }

  const matrix = $derived(monthMatrix(year, monthIndex))
  const byDay = $derived(eventsByDay(events))

  const monthLabel = $derived(
    `${year}-${String(monthIndex + 1).padStart(2, '0')}`
  )

  const todayIso = toISODate(new Date()) // LOCAL date, matches grid cell.iso

  $effect(() => {
    // key on year+monthIndex
    const y = year, m = monthIndex
    const { from, to } = monthRange(y, m)
    error = ''
    Promise.all([
      api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`),
      api.get('/api/projects'),
      api.get('/api/contacts'),
    ]).then(([evs, pjs, cts]) => {
      events = evs
      projects = pjs
      contacts = cts
    }).catch((e) => { error = e.message })
  })

  function prevMonth() {
    if (monthIndex === 0) { year -= 1; monthIndex = 11 } else { monthIndex -= 1 }
  }
  function nextMonth() {
    if (monthIndex === 11) { year += 1; monthIndex = 0 } else { monthIndex += 1 }
  }
  function goToday() {
    const n = new Date()
    year = n.getFullYear()
    monthIndex = n.getMonth()
  }

  // Build a new-event skeleton for a given day cell
  function newEvSkeleton(iso) {
    return {
      title: '',
      all_day: false,
      starts_at_local: `${iso}T09:00`,
      ends_at_local: `${iso}T10:00`,
      starts_date: iso,
      ends_date: iso,
      project_id: '',
      contact_id: '',
      notes: '',
      repeat: 'none',
      until: '',
    }
  }

  function openNew(iso) {
    modal = { mode: 'new', ev: newEvSkeleton(iso) }
  }

  function openEdit(ev, e) {
    e.stopPropagation()
    // Timed events: render the UTC instant in LOCAL time for datetime-local
    // (save converts back via new Date(local).toISOString(), so it round-trips).
    const pad = (n) => String(n).padStart(2, '0')
    const toLocal = (iso) => {
      const d = new Date(iso)
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
    }
    // All-day dates are stored as the UTC date (00:00:00Z / 23:59:59Z); keep the
    // UTC slice so the date inputs round-trip to the originally-picked day.
    const toDate = (iso) => iso.slice(0, 10)
    modal = {
      mode: 'edit',
      ev: {
        id: ev.id,
        title: ev.title,
        all_day: ev.all_day,
        starts_at_local: toLocal(ev.starts_at),
        ends_at_local: toLocal(ev.ends_at),
        starts_date: toDate(ev.starts_at),
        ends_date: toDate(ev.ends_at),
        project_id: ev.project_id ?? '',
        contact_id: ev.contact_id ?? '',
        notes: ev.notes ?? '',
      },
    }
  }

  function buildBody(ev) {
    let starts_at, ends_at
    if (ev.all_day) {
      starts_at = `${ev.starts_date}T00:00:00Z`
      ends_at = `${ev.ends_date}T23:59:59Z`
    } else {
      starts_at = new Date(ev.starts_at_local).toISOString()
      ends_at = new Date(ev.ends_at_local).toISOString()
    }
    return {
      title: ev.title.trim(),
      starts_at,
      ends_at,
      all_day: ev.all_day,
      project_id: ev.project_id || null,
      contact_id: ev.contact_id || null,
      notes: ev.notes || null,
    }
  }

  let modalError = $state('')

  async function saveModal(e) {
    e.preventDefault()
    const ev = modal.ev
    if (!ev.title.trim()) { modalError = 'Title is required'; return }
    const body = buildBody(ev)
    if (new Date(body.ends_at) < new Date(body.starts_at)) {
      modalError = 'End must be >= start'
      return
    }
    modalError = ''
    try {
      if (modal.mode === 'new') {
        if (ev.repeat && ev.repeat !== 'none') {
          if (!ev.until) { modalError = 'End date is required for repeats'; return }
          const start = new Date(body.starts_at)
          const end = new Date(body.ends_at)
          const until = new Date(`${ev.until}T00:00:00`)
          if (until < start) { modalError = 'End date must be on or after the start'; return }
          const gen = generateOccurrences({ start, end, freq: ev.repeat, until })
          if (gen.length === 0) { modalError = 'No occurrences in that range'; return }
          if (gen.length > 366) { modalError = 'Too many occurrences (max 366)'; return }
          const { title, all_day, project_id, contact_id, notes } = body
          const occurrences = gen.map((o) => ({
            title, all_day, project_id, contact_id, notes,
            starts_at: o.starts_at, ends_at: o.ends_at,
          }))
          await api.post('/api/events/series', { occurrences })
        } else {
          await api.post('/api/events', body)
        }
      } else {
        await api.put(`/api/events/${ev.id}`, body)
      }
      modal = null
      const { from, to } = monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
  }

  async function deleteEvent() {
    try {
      await api.del(`/api/events/${modal.ev.id}`)
      modal = null
      const { from, to } = monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
  }

  function handleDayKey(e, iso) {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openNew(iso) }
  }
</script>

<!-- Header bar -->
<div class="rise mb-5 flex flex-wrap items-center gap-3">
  <div class="flex items-center gap-2">
    <button
      onclick={prevMonth}
      class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink"
      aria-label="Previous month"
    >[ &lt; ]</button>
    <span class="font-mono text-[13px] text-accent glow-text tabular-nums">&gt; {monthLabel}</span>
    <button
      onclick={goToday}
      class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink"
    >[ today ]</button>
    <button
      onclick={nextMonth}
      class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink"
      aria-label="Next month"
    >[ &gt; ]</button>
  </div>
  <button
    onclick={() => openNew(todayIso)}
    class="ml-auto inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> new event
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<!-- Calendar grid -->
<div class="rise rounded-sm border border-border bg-surface" style="animation-delay:40ms">
  <!-- Day-of-week headers -->
  <div class="grid grid-cols-7 border-b border-border">
    {#each DOW as d}
      <div class="px-2 py-1.5 font-mono text-[11px] text-faint text-center">{d}</div>
    {/each}
  </div>

  <!-- 6 rows of days -->
  {#each matrix as week}
    <div class="grid grid-cols-7 border-b border-border/50 last:border-0">
      {#each week as cell}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          role="button"
          tabindex="0"
          onclick={() => openNew(cell.iso)}
          onkeydown={(e) => handleDayKey(e, cell.iso)}
          class="min-h-[90px] cursor-pointer border-r border-border/30 p-1.5 transition-colors duration-100 last:border-0 hover:bg-surface-2 {!cell.inMonth ? 'opacity-40' : ''}"
          aria-label="New event on {cell.iso}"
        >
          <!-- Day number -->
          <div class="mb-1 flex justify-end">
            <span class="font-mono text-[12px] tabular-nums {cell.iso === todayIso ? 'rounded-sm bg-accent px-1 text-on-accent glow-soft font-bold' : 'text-faint'}">{cell.day}</span>
          </div>
          <!-- Event chips -->
          {#if byDay.has(cell.iso)}
            {@const dayEvents = byDay.get(cell.iso)}
            {#each dayEvents.slice(0, 3) as ev}
              <button
                onclick={(e) => openEdit(ev, e)}
                class="mb-0.5 w-full truncate rounded-sm bg-accent-dim/20 px-1.5 py-0.5 text-left font-mono text-[11px] text-accent transition hover:bg-accent-dim/40"
                title="{ev.title}{ev.all_day ? '' : ' ' + fmtTime(ev.starts_at)}"
              >
                {#if !ev.all_day}<span class="text-faint">{fmtTime(ev.starts_at)} </span>{/if}{ev.title}
              </button>
            {/each}
            {#if dayEvents.length > 3}
              <div class="mt-0.5 font-mono text-[10px] text-faint">+{dayEvents.length - 3} more</div>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/each}
</div>

<!-- Event modal -->
{#if modal}
  <Modal title={modal.mode === 'new' ? 'new event' : 'edit event'} onclose={() => { modal = null; modalError = '' }}>
    <form onsubmit={saveModal} class="flex flex-col gap-3">
      {#if modalError}
        <p class="rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[11px] text-st-lost">[ ERR ] {modalError}</p>
      {/if}
      <div>
        <p class="label mb-1.5">Title</p>
        <input bind:value={modal.ev.title} placeholder="Event title" required class={FIELD} />
      </div>
      <div class="flex items-center gap-2">
        <input type="checkbox" id="all_day" bind:checked={modal.ev.all_day} class="accent-[#3ef5c4]" />
        <label for="all_day" class="font-mono text-[13px] text-muted cursor-pointer">All day</label>
      </div>
      {#if modal.ev.all_day}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="label mb-1.5">Start date</p>
            <input type="date" bind:value={modal.ev.starts_date} required class={FIELD} />
          </div>
          <div>
            <p class="label mb-1.5">End date</p>
            <input type="date" bind:value={modal.ev.ends_date} required class={FIELD} />
          </div>
        </div>
      {:else}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="label mb-1.5">Start</p>
            <input type="datetime-local" bind:value={modal.ev.starts_at_local} required class={FIELD} />
          </div>
          <div>
            <p class="label mb-1.5">End</p>
            <input type="datetime-local" bind:value={modal.ev.ends_at_local} required class={FIELD} />
          </div>
        </div>
      {/if}
      {#if modal.mode === 'new'}
        <div class="grid grid-cols-2 gap-2">
          <div>
            <p class="label mb-1.5">Repeat</p>
            <select bind:value={modal.ev.repeat} class={FIELD}>
              <option value="none">Does not repeat</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="monthly">Monthly</option>
            </select>
          </div>
          {#if modal.ev.repeat !== 'none'}
            <div>
              <p class="label mb-1.5">Ends</p>
              <input type="date" bind:value={modal.ev.until} required class={FIELD} />
            </div>
          {/if}
        </div>
      {/if}
      <div>
        <p class="label mb-1.5">Project <span class="text-faint">(optional)</span></p>
        <select bind:value={modal.ev.project_id} class={FIELD}>
          <option value="">— none —</option>
          {#each projects as p}<option value={p.id}>{p.title}</option>{/each}
        </select>
      </div>
      <div>
        <p class="label mb-1.5">Contact <span class="text-faint">(optional)</span></p>
        <select bind:value={modal.ev.contact_id} class={FIELD}>
          <option value="">— none —</option>
          {#each contacts as c}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </div>
      <div>
        <p class="label mb-1.5">Notes <span class="text-faint">(optional)</span></p>
        <textarea bind:value={modal.ev.notes} placeholder="Notes…" rows="2" class="{FIELD} resize-none"></textarea>
      </div>
      <div class="flex gap-2 pt-1">
        <button type="submit" class="flex-1 h-9 rounded-sm bg-accent font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110">
          {modal.mode === 'new' ? 'create' : 'save'}
        </button>
        {#if modal.mode === 'edit'}
          <button
            type="button"
            onclick={deleteEvent}
            class="h-9 rounded-sm border border-st-lost/40 px-3 font-mono text-[12px] text-st-lost transition hover:bg-st-lost/10"
          >delete</button>
        {/if}
      </div>
    </form>
  </Modal>
{/if}
