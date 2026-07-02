// Pure helpers for the Contacts feature. No I/O — all aggregation client-side.
const ms = (d) => new Date(d).getTime()

export function filterContacts(contacts, query, kind) {
  const q = query.trim().toLowerCase()
  const nameById = new Map(contacts.map((c) => [c.id, c.name]))
  return contacts.filter((c) => {
    if (kind !== 'all' && c.kind !== kind) return false
    if (!q) return true
    const company = c.company_id ? nameById.get(c.company_id) || '' : ''
    return (
      c.name.toLowerCase().includes(q) ||
      (c.email || '').toLowerCase().includes(q) ||
      company.toLowerCase().includes(q)
    )
  })
}

export function projectCountsByContact(projects) {
  const m = new Map()
  for (const p of projects) {
    if (!p.contact_id) continue
    const c = m.get(p.contact_id) || { active: 0, total: 0 }
    c.total += 1
    if (p.status === 'active') c.active += 1
    m.set(p.contact_id, c)
  }
  return m
}

export function eventsForContact(events, contactId) {
  return events.filter((e) => (e.contact_ids || []).includes(contactId))
}

export function buildTimeline(notes, events, now) {
  const nowMs = ms(now)
  const noteItems = notes.map((n) => ({ type: 'note', id: n.id, when: n.created_at, text: n.body }))
  const eventItems = events.map((e) => ({ type: 'event', id: e.id, when: e.starts_at, text: e.title, all_day: e.all_day }))
  const upcoming = eventItems
    .filter((e) => ms(e.when) > nowMs)
    .sort((a, b) => ms(a.when) - ms(b.when))
  const past = eventItems.filter((e) => ms(e.when) <= nowMs)
  const history = [...noteItems, ...past].sort((a, b) => ms(b.when) - ms(a.when))
  return { upcoming, history }
}

export function lastTouch(notes, events, now) {
  const nowMs = ms(now)
  const dates = [
    ...notes.map((n) => n.created_at),
    ...events.map((e) => e.starts_at),
  ].filter((d) => ms(d) <= nowMs)
  if (!dates.length) return null
  return dates.reduce((a, b) => (ms(a) > ms(b) ? a : b))
}

export function humanizeSince(dateStr, now) {
  if (!dateStr) return '—'
  const days = Math.floor((ms(now) - ms(dateStr)) / 86400000)
  if (days <= 0) return 'today'
  return `${days}d ago`
}

export function companyRoster(contacts, companyId) {
  return contacts.filter((c) => c.company_id === companyId)
}

export function companyName(contacts, companyId) {
  return contacts.find((c) => c.id === companyId)?.name ?? null
}
