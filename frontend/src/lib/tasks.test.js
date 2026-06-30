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

  it('emits a move when an item lands in a different-status column', () => {
    const moves = movesForTaskColumn('doing', [
      { id: 'x', status: 'todo' },
      { id: 'y', status: 'doing' },
    ])
    expect(moves).toEqual([{ id: 'x', status: 'doing' }])
  })

  it('emits no moves when all items already have the column status', () => {
    const moves = movesForTaskColumn('done', [
      { id: 'a', status: 'done' },
      { id: 'b', status: 'done' },
    ])
    expect(moves).toEqual([])
  })
})
