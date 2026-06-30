// All calendar dates use the LOCAL timezone as the single basis: grid cells,
// event grouping, and time display all derive from local Date fields so an
// event always lands on (and reads as) the day/time the user sees.
// ponytail: an all-day event stored as 00:00:00Z is correct on UTC+ machines;
// on behind-UTC machines it can shift a day. Acceptable here; revisit if the
// app must serve negative-offset timezones.

/** Local 'YYYY-MM-DD' for a Date. */
export function toISODate(d) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

/**
 * monthMatrix(year, monthIndex) → 6×7 grid of day objects.
 * Weeks start Monday. Includes leading/trailing days from adjacent months.
 * @param {number} year
 * @param {number} monthIndex  0-based (Jan=0)
 * @returns {Array<{date:Date, day:number, inMonth:boolean, iso:string}[]>}
 */
export function monthMatrix(year, monthIndex) {
  const first = new Date(year, monthIndex, 1)
  // Day of week of first (0=Sun..6=Sat); convert to Mon-start (0=Mon..6=Sun)
  const startDow = (first.getDay() + 6) % 7
  const gridStart = new Date(year, monthIndex, 1 - startDow)

  const weeks = []
  for (let w = 0; w < 6; w++) {
    const week = []
    for (let d = 0; d < 7; d++) {
      const date = new Date(gridStart)
      date.setDate(gridStart.getDate() + w * 7 + d)
      week.push({ date, day: date.getDate(), inMonth: date.getMonth() === monthIndex, iso: toISODate(date) })
    }
    weeks.push(week)
  }
  return weeks
}

/**
 * monthRange(year, monthIndex) → { from, to } RFC3339 UTC instants covering the
 * full visible grid: from = local midnight of the first cell, to = local
 * midnight of the cell after the last (42 days later). Used for the events fetch.
 */
export function monthRange(year, monthIndex) {
  const first = new Date(year, monthIndex, 1)
  const startDow = (first.getDay() + 6) % 7
  const gridStart = new Date(year, monthIndex, 1 - startDow)
  const gridEnd = new Date(gridStart)
  gridEnd.setDate(gridStart.getDate() + 42)
  // .toISOString() converts each local-midnight Date to its true UTC instant.
  return { from: gridStart.toISOString(), to: gridEnd.toISOString() }
}

/**
 * eventsByDay(events) → Map<'YYYY-MM-DD', Event[]>
 * Groups events by their LOCAL start date so they match the grid cells.
 */
export function eventsByDay(events) {
  const map = new Map()
  for (const ev of events) {
    const day = toISODate(new Date(ev.starts_at))
    if (!map.has(day)) map.set(day, [])
    map.get(day).push(ev)
  }
  return map
}

/**
 * fmtTime(iso) → local 'HH:MM' (24h) from an RFC3339 string.
 */
export function fmtTime(iso) {
  const d = new Date(iso)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}
