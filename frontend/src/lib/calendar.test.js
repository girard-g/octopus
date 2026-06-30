// frontend/src/lib/calendar.test.js
import { describe, it, expect } from 'vitest'
import { monthMatrix, monthRange, eventsByDay, fmtTime } from './calendar.js'

// June 2026: monthIndex=5, year=2026. June 1 2026 is a Monday → gridStart = June 1.
// Helpers use the LOCAL timezone basis; tests build their fixtures from local
// Date constructors so assertions are timezone-independent.

describe('monthMatrix', () => {
  const matrix = monthMatrix(2026, 5) // June 2026

  it('returns 6 weeks of 7 days (42 cells)', () => {
    expect(matrix).toHaveLength(6)
    expect(matrix.every((w) => w.length === 7)).toBe(true)
  })

  it('first cell is Monday June 1 (inMonth=true, since June 1 is a Monday)', () => {
    const first = matrix[0][0]
    expect(first.iso).toBe('2026-06-01')
    expect(first.inMonth).toBe(true)
    expect(first.day).toBe(1)
  })

  it('marks trailing days outside June as inMonth=false', () => {
    // June ends on the 30th; the last cell (July 12) must be out of month.
    const lastCell = matrix[5][6]
    expect(lastCell.inMonth).toBe(false)
    expect(lastCell.iso).toBe('2026-07-12')
  })

  it('weeks start Monday', () => {
    for (const week of matrix) {
      expect(week[0].date.getDay()).toBe(1)
    }
  })
})

// Month with leading days: May 2026 — May 1 is a Friday.
describe('monthMatrix leading days', () => {
  it('includes leading days from previous month when month does not start Monday', () => {
    const matrix = monthMatrix(2026, 4) // May 2026
    const first = matrix[0][0]
    // May 1 2026 is Friday → grid starts April 27 (Monday)
    expect(first.iso).toBe('2026-04-27')
    expect(first.inMonth).toBe(false)
  })
})

describe('monthRange', () => {
  it('covers full 42-cell grid for June 2026 (local-midnight UTC instants)', () => {
    const { from, to } = monthRange(2026, 5)
    // June 2026 grid starts June 1 (Monday); ends 42 days later = July 13.
    expect(from).toBe(new Date(2026, 5, 1).toISOString())
    expect(to).toBe(new Date(2026, 6, 13).toISOString())
  })

  it('from < to', () => {
    const { from, to } = monthRange(2026, 0)
    expect(new Date(from) < new Date(to)).toBe(true)
  })
})

describe('eventsByDay', () => {
  // Build starts_at from local Date constructors so the expected grouping key
  // is deterministic regardless of the runner's timezone.
  const events = [
    { id: '1', title: 'A', starts_at: new Date(2026, 5, 15, 9, 0).toISOString() },
    { id: '2', title: 'B', starts_at: new Date(2026, 5, 15, 14, 0).toISOString() },
    { id: '3', title: 'C', starts_at: new Date(2026, 5, 20, 8, 0).toISOString() },
  ]

  it('groups events by local start date', () => {
    const map = eventsByDay(events)
    expect(map.get('2026-06-15')).toHaveLength(2)
    expect(map.get('2026-06-20')).toHaveLength(1)
  })

  it('returns a Map', () => {
    expect(eventsByDay(events) instanceof Map).toBe(true)
  })

  it('returns empty Map for empty input', () => {
    expect(eventsByDay([]).size).toBe(0)
  })
})

describe('fmtTime', () => {
  it('formats local HH:MM (24h)', () => {
    expect(fmtTime(new Date(2026, 5, 15, 9, 30).toISOString())).toBe('09:30')
    expect(fmtTime(new Date(2026, 5, 15, 0, 0).toISOString())).toBe('00:00')
    expect(fmtTime(new Date(2026, 5, 15, 23, 59).toISOString())).toBe('23:59')
  })
})
