export const STATUSES = ['lead', 'proposal', 'active', 'done', 'lost']
export const STATUS_LABELS = {
  lead: 'Lead', proposal: 'Proposal', active: 'Active', done: 'Done', lost: 'Lost',
}

// Group a flat project list into { status: [projects sorted by board_order] }.
export function groupByStatus(projects) {
  const cols = Object.fromEntries(STATUSES.map((s) => [s, []]))
  for (const p of projects) (cols[p.status] ??= []).push(p)
  for (const s of STATUSES) cols[s].sort((a, b) => a.board_order - b.board_order)
  return cols
}

// After a dnd drop, compute the /move payloads for a column's items:
// each item that changed status or position gets {id, status, board_order:index}.
export function movesForColumn(status, items) {
  return items.map((p, i) => ({ id: p.id, status, board_order: i }))
    .filter((m, i) => items[i].status !== status || items[i].board_order !== i)
}
