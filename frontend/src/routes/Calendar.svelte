<script>
  import { api } from '../lib/api.js'
  import Modal from '../lib/components/Modal.svelte'
  import { monthMatrix, monthRange, eventsByDay, fmtTime, toISODate, generateOccurrences, localDateTimeToUtc, dayRange, dayAgenda, dayCellAction } from '../lib/calendar.js'

  const FIELD = 'w-full rounded-sm border border-border bg-surface-2 px-2.5 py-2 font-mono text-[13px] text-ink placeholder:text-faint focus:border-accent focus:shadow-[0_0_0_3px_rgba(62,245,196,0.14)] focus:outline-none'
  const DOW = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

  // Current month state — defaults to real current month (browser only)
  const now = new Date()
  let year = $state(now.getFullYear())
  let monthIndex = $state(now.getMonth())
  let view = $state('month') // 'month' | 'day'
  let selectedDate = $state(toISODate(new Date()))

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
    const range = view === 'day' ? dayRange(selectedDate) : monthRange(year, monthIndex)
    error = ''
    Promise.all([
      api.get(`/api/events?from=${encodeURIComponent(range.from)}&to=${encodeURIComponent(range.to)}`),
      api.get('/api/projects'),
      api.get('/api/contacts'),
    ]).then(([evs, pjs, cts]) => {
      events = evs
      projects = pjs
      contacts = cts
    }).catch((e) => { error = e.message })
  })

  const agenda = $derived(dayAgenda(events))
  function shiftDay(delta) {
    const d = new Date(`${selectedDate}T00:00:00`)
    d.setDate(d.getDate() + delta)
    selectedDate = toISODate(d)
  }

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
      date: iso,
      start_time: '09:00',
      end_time: '10:00',
      starts_date: iso,
      ends_date: iso,
      project_id: '',
      contact_ids: [],
      notes: '',
      repeat: 'none',
      until: '',
    }
  }

  function openNew(iso) {
    modal = { mode: 'new', ev: newEvSkeleton(iso) }
  }

  // Reactive <768px flag driven by matchMedia (browser only; SSR-safe guard).
  let isMobile = $state(false)
  $effect(() => {
    if (typeof window === 'undefined') return
    const mq = window.matchMedia('(max-width: 767px)')
    isMobile = mq.matches
    const on = (e) => { isMobile = e.matches }
    mq.addEventListener('change', on)
    return () => mq.removeEventListener('change', on)
  })

  function openDayCell(iso) {
    if (dayCellAction(isMobile) === 'day') {
      selectedDate = iso
      view = 'day'
    } else {
      openNew(iso)
    }
  }

  function openEdit(ev, e) {
    e.stopPropagation()
    const pad = (n) => String(n).padStart(2, '0')
    const localDate = (iso) => { const d = new Date(iso); return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` }
    const localTime = (iso) => { const d = new Date(iso); return `${pad(d.getHours())}:${pad(d.getMinutes())}` }
    // All-day dates are stored as the UTC date (00:00:00Z / 23:59:59Z); keep the
    // UTC slice so the date inputs round-trip to the originally-picked day.
    const toDate = (iso) => iso.slice(0, 10)
    modal = {
      mode: 'edit',
      ev: {
        id: ev.id,
        title: ev.title,
        all_day: ev.all_day,
        date: localDate(ev.starts_at),
        start_time: localTime(ev.starts_at),
        end_time: localTime(ev.ends_at),
        starts_date: toDate(ev.starts_at),
        ends_date: toDate(ev.ends_at),
        project_id: ev.project_id ?? '',
        contact_ids: ev.contact_ids ?? [],
        notes: ev.notes ?? '',
        series_id: ev.series_id ?? null,
        orig_starts_at: ev.starts_at,
        scope: 'one',
      },
    }
  }

  function buildBody(ev) {
    let starts_at, ends_at
    if (ev.all_day) {
      starts_at = `${ev.starts_date}T00:00:00Z`
      ends_at = `${ev.ends_date}T23:59:59Z`
    } else {
      starts_at = localDateTimeToUtc(ev.date, ev.start_time)
      ends_at = localDateTimeToUtc(ev.date, ev.end_time)
    }
    return {
      title: ev.title.trim(),
      starts_at,
      ends_at,
      all_day: ev.all_day,
      project_id: ev.project_id || null,
      contact_ids: ev.contact_ids ?? [],
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
          const { title, all_day, project_id, contact_ids, notes } = body
          const occurrences = gen.map((o) => ({
            title, all_day, project_id, contact_ids, notes,
            starts_at: o.starts_at, ends_at: o.ends_at,
          }))
          await api.post('/api/events/series', { occurrences })
        } else {
          await api.post('/api/events', body)
        }
      } else {
        if (ev.series_id && ev.scope !== 'one') {
          const shift_seconds = Math.round(
            (new Date(body.starts_at).getTime() - new Date(ev.orig_starts_at).getTime()) / 1000
          )
          await api.patch(`/api/events/${ev.id}/series?scope=${ev.scope}`, {
            title: body.title,
            notes: body.notes,
            project_id: body.project_id,
            contact_ids: ev.contact_ids ?? [],
            all_day: body.all_day,
            shift_seconds,
          })
        } else {
          await api.put(`/api/events/${ev.id}`, body)
        }
      }
      modal = null
      const { from, to } = view === 'day' ? dayRange(selectedDate) : monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
  }

  async function deleteEvent() {
    const ev = modal.ev
    const scope = ev.series_id ? ev.scope : 'one'
    try {
      await api.del(`/api/events/${ev.id}?scope=${scope}`)
      modal = null
      const { from, to } = view === 'day' ? dayRange(selectedDate) : monthRange(year, monthIndex)
      events = await api.get(`/api/events?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`)
    } catch (err) { modalError = err.message }
  }

  function handleDayKey(e, iso) {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openDayCell(iso) }
  }

  const contactName = (id) => contacts.find((c) => c.id === id)?.name ?? '?'
  function toggleContact(id) {
    const set = modal.ev.contact_ids
    modal.ev.contact_ids = set.includes(id) ? set.filter((x) => x !== id) : [...set, id]
  }
</script>

<!-- Header bar -->
<div class="rise mb-5 flex flex-wrap items-center gap-3">
  <div class="flex items-center gap-1">
    <button onclick={() => (view = 'month')}
      class="h-8 rounded-sm border px-2.5 font-mono text-[12px] transition {view === 'month' ? 'border-accent bg-accent-dim/20 text-accent' : 'border-border text-muted hover:text-ink'}">month</button>
    <button onclick={() => (view = 'day')}
      class="h-8 rounded-sm border px-2.5 font-mono text-[12px] transition {view === 'day' ? 'border-accent bg-accent-dim/20 text-accent' : 'border-border text-muted hover:text-ink'}">day</button>
  </div>
  {#if view === 'month'}
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
  {/if}
  {#if view === 'day'}
    <div class="flex items-center gap-2">
      <button onclick={() => shiftDay(-1)} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink" aria-label="Previous day">[ &lt; ]</button>
      <span class="font-mono text-[13px] text-accent glow-text tabular-nums">&gt; {selectedDate}</span>
      <button onclick={() => (selectedDate = toISODate(new Date()))} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink">[ today ]</button>
      <button onclick={() => shiftDay(1)} class="h-8 rounded-sm border border-border px-2.5 font-mono text-[13px] text-muted transition hover:border-accent-dim hover:text-ink" aria-label="Next day">[ &gt; ]</button>
    </div>
  {/if}
  <button
    onclick={() => openNew(view === 'day' ? selectedDate : todayIso)}
    class="ml-auto inline-flex h-8 items-center gap-1.5 rounded-sm bg-accent px-3 font-mono text-[13px] font-bold text-on-accent transition glow-soft hover:brightness-110"
  >
    <span class="text-[15px] leading-none">+</span> new event
  </button>
</div>

{#if error}<p class="rise mb-4 rounded-sm border border-st-lost/30 bg-st-lost/10 px-3 py-2 font-mono text-[12px] text-st-lost">[ ERR ] {error}</p>{/if}

<!-- Calendar grid -->
{#if view === 'month'}
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
            onclick={() => openDayCell(cell.iso)}
            onkeydown={(e) => handleDayKey(e, cell.iso)}
            class="min-h-[54px] md:min-h-[90px] cursor-pointer border-r border-border/30 p-1.5 transition-colors duration-100 last:border-0 hover:bg-surface-2 {!cell.inMonth ? 'opacity-40' : ''}"
            aria-label={isMobile ? cell.iso : `New event on ${cell.iso}`}
          >
            <!-- Day number -->
            <div class="mb-1 flex justify-end">
              <span class="font-mono text-[12px] tabular-nums {cell.iso === todayIso ? 'rounded-sm bg-accent px-1 text-on-accent glow-soft font-bold' : 'text-faint'}">{cell.day}</span>
            </div>
            <!-- Event chips -->
            {#if byDay.has(cell.iso)}
              {@const dayEvents = byDay.get(cell.iso)}
              <!-- Mobile: up to 4 event dots -->
              <div class="flex flex-wrap gap-0.5 md:hidden" aria-hidden="true">
                {#each dayEvents.slice(0, 4) as _ev}
                  <span class="h-1.5 w-1.5 rounded-full bg-accent glow-soft"></span>
                {/each}
              </div>
              <!-- Desktop: text chips (unchanged) -->
              <div class="hidden md:block">
                {#each dayEvents.slice(0, 3) as ev}
                  <button
                    onclick={(e) => openEdit(ev, e)}
                    class="mb-0.5 w-full truncate rounded-sm bg-accent-dim/20 px-1.5 py-0.5 text-left font-mono text-[11px] text-accent transition hover:bg-accent-dim/40"
                    title="{ev.title}{ev.all_day ? '' : ' ' + fmtTime(ev.starts_at)}{ev.contact_ids?.length ? ' — ' + ev.contact_ids.map(contactName).join(', ') : ''}"
                  >
                    {#if !ev.all_day}<span class="text-faint">{fmtTime(ev.starts_at)} </span>{/if}{ev.title}
                  </button>
                {/each}
                {#if dayEvents.length > 3}
                  <div class="mt-0.5 font-mono text-[10px] text-faint">+{dayEvents.length - 3} more</div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/if}

{#if view === 'day'}
  <div class="rise rounded-sm border border-border bg-surface" style="animation-delay:40ms">
    {#if agenda.allDay.length === 0 && agenda.timed.length === 0}
      <p class="px-4 py-8 text-center font-mono text-[13px] text-faint">// no events</p>
    {:else}
      {#each agenda.allDay as ev}
        <button onclick={(e) => openEdit(ev, e)} class="flex w-full items-center gap-3 border-b border-border/50 px-4 py-2.5 text-left transition last:border-0 hover:bg-surface-2">
          <span class="w-14 font-mono text-[11px] text-faint">all-day</span>
          <span class="font-mono text-[13px] text-ink">{ev.title}</span>
          {#if ev.contact_ids?.length}<span class="font-mono text-[11px] text-accent">{ev.contact_ids.map(contactName).map((n) => '@' + n).join(' ')}</span>{/if}
        </button>
      {/each}
      {#each agenda.timed as ev}
        <button onclick={(e) => openEdit(ev, e)} class="flex w-full items-center gap-3 border-b border-border/50 px-4 py-2.5 text-left transition last:border-0 hover:bg-surface-2">
          <span class="w-14 font-mono text-[12px] tabular-nums text-accent">{fmtTime(ev.starts_at)}</span>
          <span class="font-mono text-[13px] text-ink">{ev.title}</span>
          {#if ev.contact_ids?.length}<span class="font-mono text-[11px] text-accent">{ev.contact_ids.map(contactName).map((n) => '@' + n).join(' ')}</span>{/if}
        </button>
      {/each}
    {/if}
  </div>
{/if}

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
      <div class="flex items-center gap-2" class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
        <input type="checkbox" id="all_day" bind:checked={modal.ev.all_day} class="accent-[#3ef5c4]" disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
        <label for="all_day" class="font-mono text-[13px] text-muted cursor-pointer">All day</label>
      </div>
      {#if modal.ev.all_day}
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <div>
            <p class="label mb-1.5">Start date</p>
            <input type="date" bind:value={modal.ev.starts_date} required class={FIELD} />
          </div>
          <div class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
            <p class="label mb-1.5">End date</p>
            <input type="date" bind:value={modal.ev.ends_date} required class={FIELD} disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
          </div>
        </div>
      {:else}
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
          <div class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
            <p class="label mb-1.5">Date</p>
            <input type="date" bind:value={modal.ev.date} required class={FIELD} disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
          </div>
          <div>
            <p class="label mb-1.5">Start</p>
            <input type="time" bind:value={modal.ev.start_time} required class={FIELD} />
          </div>
          <div class:opacity-50={modal.ev.series_id && modal.ev.scope !== 'one'}>
            <p class="label mb-1.5">End</p>
            <input type="time" bind:value={modal.ev.end_time} required class={FIELD} disabled={modal.ev.series_id && modal.ev.scope !== 'one'} />
          </div>
        </div>
      {/if}
      {#if modal.mode === 'new'}
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
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
        <p class="label mb-1.5">People <span class="text-faint">(optional)</span></p>
        <div class="flex flex-wrap gap-1.5">
          {#each contacts as c}
            {@const on = modal.ev.contact_ids.includes(c.id)}
            <button
              type="button"
              onclick={() => toggleContact(c.id)}
              class="rounded-sm border px-2 py-1 font-mono text-[12px] transition {on ? 'border-accent bg-accent-dim/25 text-accent' : 'border-border bg-surface-2 text-muted hover:border-accent-dim'}"
            >{on ? '✓ ' : ''}{c.name}</button>
          {/each}
          {#if contacts.length === 0}<span class="font-mono text-[12px] text-faint">// no contacts</span>{/if}
        </div>
      </div>
      <div>
        <p class="label mb-1.5">Notes <span class="text-faint">(optional)</span></p>
        <textarea bind:value={modal.ev.notes} placeholder="Notes…" rows="2" class="{FIELD} resize-none"></textarea>
      </div>
      {#if modal.mode === 'edit' && modal.ev.series_id}
        <div>
          <p class="label mb-1.5">Apply to</p>
          <select bind:value={modal.ev.scope} class={FIELD}>
            <option value="one">This event only</option>
            <option value="following">This and following</option>
            <option value="series">Entire series</option>
          </select>
          {#if modal.ev.scope !== 'one'}
            <p class="font-mono text-[11px] text-faint mt-1.5">Range edits change title/notes and shift the start time. Date, end time and all-day are locked — use "This event only" to change them.</p>
          {/if}
        </div>
      {/if}
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
