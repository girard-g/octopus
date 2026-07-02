import { describe, it, expect } from 'vitest'
import {
  filterContacts, projectCountsByContact, eventsForContact,
  buildTimeline, lastTouch, humanizeSince, companyRoster, companyName,
} from './contacts.js'

const NOW = '2026-07-02T12:00:00Z'

const contacts = [
  { id: 'co1', kind: 'company', name: 'Acme Corp', email: 'ops@acme.io', phone: null, company_id: null },
  { id: 'p1', kind: 'person', name: 'Jane Doe', email: 'jane@acme.io', phone: '555', company_id: 'co1' },
  { id: 'p2', kind: 'person', name: 'Bob Smith', email: 'bob@x.io', phone: null, company_id: null },
]

describe('filterContacts', () => {
  it('filters by kind', () => {
    expect(filterContacts(contacts, '', 'company').map((c) => c.id)).toEqual(['co1'])
    expect(filterContacts(contacts, '', 'person').map((c) => c.id)).toEqual(['p1', 'p2'])
    expect(filterContacts(contacts, '', 'all')).toHaveLength(3)
  })
  it('matches name, email, and resolved company name', () => {
    expect(filterContacts(contacts, 'jane', 'all').map((c) => c.id)).toEqual(['p1'])
    expect(filterContacts(contacts, 'bob@x', 'all').map((c) => c.id)).toEqual(['p2'])
    // Jane belongs to Acme → matches on company name even though her name doesn't contain "acme"
    expect(filterContacts(contacts, 'acme', 'all').map((c) => c.id)).toEqual(['co1', 'p1'])
  })
})

describe('projectCountsByContact', () => {
  it('counts active and total per contact, skips null contact_id', () => {
    const projects = [
      { id: 'a', contact_id: 'co1', status: 'active' },
      { id: 'b', contact_id: 'co1', status: 'archived' },
      { id: 'c', contact_id: 'p1', status: 'active' },
      { id: 'd', contact_id: null, status: 'active' },
    ]
    const m = projectCountsByContact(projects)
    expect(m.get('co1')).toEqual({ active: 1, total: 2 })
    expect(m.get('p1')).toEqual({ active: 1, total: 1 })
    expect(m.has('d')).toBe(false)
  })
})

describe('eventsForContact', () => {
  it('keeps events whose contact_ids includes the id', () => {
    const events = [
      { id: 'e1', contact_ids: ['p1', 'co1'] },
      { id: 'e2', contact_ids: ['p2'] },
      { id: 'e3', contact_ids: [] },
    ]
    expect(eventsForContact(events, 'p1').map((e) => e.id)).toEqual(['e1'])
  })
})

describe('buildTimeline', () => {
  const notes = [
    { id: 'n1', body: 'quote sent', created_at: '2026-07-01T09:00:00Z' },
    { id: 'n2', body: 'intro', created_at: '2026-06-25T09:00:00Z' },
  ]
  const events = [
    { id: 'e1', title: 'Kickoff', starts_at: '2026-07-04T10:00:00Z', all_day: false },
    { id: 'e2', title: 'Discovery', starts_at: '2026-06-28T10:00:00Z', all_day: false },
  ]
  it('splits upcoming (asc) from history (desc)', () => {
    const { upcoming, history } = buildTimeline(notes, events, NOW)
    expect(upcoming.map((i) => i.id)).toEqual(['e1'])
    expect(upcoming[0]).toMatchObject({ type: 'event', text: 'Kickoff' })
    // history: n1 (07-01) > e2 (06-28) > n2 (06-25)
    expect(history.map((i) => i.id)).toEqual(['n1', 'e2', 'n2'])
    expect(history[0]).toMatchObject({ type: 'note', text: 'quote sent' })
  })
  it('handles empty inputs', () => {
    expect(buildTimeline([], [], NOW)).toEqual({ upcoming: [], history: [] })
  })
})

describe('lastTouch', () => {
  it('returns most recent past date, ignoring future', () => {
    const notes = [{ id: 'n', body: 'x', created_at: '2026-06-30T00:00:00Z' }]
    const events = [
      { id: 'e', title: 'y', starts_at: '2026-07-01T00:00:00Z' },
      { id: 'f', title: 'z', starts_at: '2026-08-01T00:00:00Z' }, // future — ignored
    ]
    expect(lastTouch(notes, events, NOW)).toBe('2026-07-01T00:00:00Z')
  })
  it('returns null when nothing in the past', () => {
    expect(lastTouch([], [], NOW)).toBe(null)
  })
})

describe('humanizeSince', () => {
  it('formats relative days', () => {
    expect(humanizeSince(null, NOW)).toBe('—')
    expect(humanizeSince('2026-07-02T06:00:00Z', NOW)).toBe('today')
    expect(humanizeSince('2026-06-29T12:00:00Z', NOW)).toBe('3d ago')
  })
})

describe('companyRoster / companyName', () => {
  it('lists people of a company', () => {
    expect(companyRoster(contacts, 'co1').map((c) => c.id)).toEqual(['p1'])
  })
  it('resolves company name or null', () => {
    expect(companyName(contacts, 'co1')).toBe('Acme Corp')
    expect(companyName(contacts, 'nope')).toBe(null)
  })
})
