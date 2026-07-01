export const TASK_STATUSES = ['todo', 'doing', 'done']
export const TASK_LABELS = { todo: 'Todo', doing: 'Doing', done: 'Done' }

// Group a flat task list into { status: [tasks] }, preserving API order.
export function groupTasks(tasks) {
  const cols = Object.fromEntries(TASK_STATUSES.map((s) => [s, []]))
  for (const t of tasks) (cols[t.status] ??= []).push(t)
  return cols
}

// After a dnd drop, compute deltas for a column: any item whose status or
// position (its index in the column) changed. Callers send the FULL task
// object with these fields overridden.
export function movesForTaskColumn(status, items) {
  return items
    .map((t, i) => ({ t, i }))
    .filter(({ t, i }) => t.status !== status || t.position !== i)
    .map(({ t, i }) => ({ id: t.id, status, position: i }))
}
