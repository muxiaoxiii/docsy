let nextRangeId = 0

// Identity belongs to the row, never its position, name or editable page range.
export function splitRangeRowKey(row) {
  if (!row.rangeId) row.rangeId = `split-range-${globalThis.crypto?.randomUUID?.() || `${Date.now()}-${++nextRangeId}`}`
  return row.rangeId
}

// Insert in visible list order. A missing boundary is an explicit draft,
// rather than a guessed page or an overlapping copy of an existing segment.
export function splitRangeAfterDefaults(items, index, totalPages) {
  const previous = items[index]
  if (!previous) return null
  const next = items.slice(index + 1).find((row) => Number(row.pageStart) > 0)
  const start = Number(previous.pageEnd) > 0 ? Number(previous.pageEnd) + 1 : null
  const end = next ? Number(next.pageStart) - 1 : Number(totalPages)
  return {
    pageStart: start && start <= end && start <= totalPages ? start : null,
    pageEnd: end >= 1 && end <= totalPages ? end : null,
  }
}

// Editing a field never inserts, removes, reorders or changes another row.
// Temporary gaps/overlaps are reported by splitRangeWarnings, not "repaired"
// by silently consuming the user's other evidence ranges.
export function editSplitRangeBoundary(items, index, page, totalPages, boundary) {
  const row = items[index]
  if (!row || page == null || !Number.isFinite(Number(page))) return -1
  if (!['start', 'end'].includes(boundary)) return -1
  const target = Math.min(Number(totalPages), Math.max(1, Math.round(Number(page))))
  row[boundary === 'start' ? 'pageStart' : 'pageEnd'] = target
  row.source = 'manual'
  return index
}

export function unassignedSplitRanges(items, totalPages) {
  let cursor = 1
  const gaps = []
  for (const row of [...items].sort((a, b) => a.pageStart - b.pageStart)) {
    const start = Number(row.pageStart),
      end = Number(row.pageEnd)
    if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || end < start) continue
    if (start > cursor) gaps.push({ pageStart: cursor, pageEnd: Math.min(totalPages, start - 1) })
    cursor = Math.max(cursor, end + 1)
  }
  if (cursor <= totalPages) gaps.push({ pageStart: cursor, pageEnd: totalPages })
  return gaps.filter((row) => row.pageEnd >= row.pageStart)
}
