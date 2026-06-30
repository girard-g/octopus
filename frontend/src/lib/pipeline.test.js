import { describe, it, expect } from 'vitest'
import { groupByStatus, movesForColumn, STATUSES } from './pipeline.js'

describe('pipeline helpers', () => {
  it('groups and sorts by board_order', () => {
    const cols = groupByStatus([
      { id: 'a', status: 'lead', board_order: 1 },
      { id: 'b', status: 'lead', board_order: 0 },
      { id: 'c', status: 'active', board_order: 0 },
    ])
    expect(cols.lead.map((p) => p.id)).toEqual(['b', 'a'])
    expect(cols.active.map((p) => p.id)).toEqual(['c'])
    expect(STATUSES.every((s) => Array.isArray(cols[s]))).toBe(true)
  })

  it('emits moves only for items whose status or order changed', () => {
    // Item dragged into 'active' at index 0; previously lead@5.
    const moves = movesForColumn('active', [{ id: 'x', status: 'lead', board_order: 5 }])
    expect(moves).toEqual([{ id: 'x', status: 'active', board_order: 0 }])
  })

  it('no moves when nothing changed', () => {
    const moves = movesForColumn('lead', [{ id: 'x', status: 'lead', board_order: 0 }])
    expect(moves).toEqual([])
  })
})
