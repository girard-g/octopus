export const TASK_STATUSES = ['todo', 'doing', 'done']
export const TASK_LABELS = { todo: 'Todo', doing: 'Doing', done: 'Done' }

// Group a flat task list into { status: [tasks] }, preserving API order.
export function groupTasks(tasks) {
  const cols = Object.fromEntries(TASK_STATUSES.map((s) => [s, []]))
  for (const t of tasks) (cols[t.status] ??= []).push(t)
  return cols
}

// After a dnd drop, compute PUT payloads: only items whose status differs from the column.
export function movesForTaskColumn(status, items) {
  return items
    .filter((t) => t.status !== status)
    .map((t) => ({ id: t.id, status }))
}
