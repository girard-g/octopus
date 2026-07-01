import { describe, it, expect } from 'vitest'
import { groupTasks, movesForTaskColumn, TASK_STATUSES } from './tasks.js'

describe('tasks helpers', () => {
  it('groups tasks into columns, preserving order', () => {
    const cols = groupTasks([
      { id: 'a', status: 'todo', created_at: '2024-01-01' },
      { id: 'b', status: 'done', created_at: '2024-01-02' },
      { id: 'c', status: 'todo', created_at: '2024-01-03' },
    ])
    expect(cols.todo.map((t) => t.id)).toEqual(['a', 'c'])
    expect(cols.done.map((t) => t.id)).toEqual(['b'])
    expect(cols.doing).toEqual([])
    expect(TASK_STATUSES.every((s) => Array.isArray(cols[s]))).toBe(true)
  })

  it('emits moves with new position for reordered or moved items', () => {
    const moves = movesForTaskColumn('doing', [
      { id: 'x', status: 'todo', position: 0 },   // status changed
      { id: 'y', status: 'doing', position: 5 },   // position changed (now index 1)
    ])
    expect(moves).toEqual([
      { id: 'x', status: 'doing', position: 0 },
      { id: 'y', status: 'doing', position: 1 },
    ])
  })

  it('emits no moves when status and position are already correct', () => {
    const moves = movesForTaskColumn('done', [
      { id: 'a', status: 'done', position: 0 },
      { id: 'b', status: 'done', position: 1 },
    ])
    expect(moves).toEqual([])
  })
})
