export function pageCount(range) {
  const pageStart = Number(range?.pageStart || 0)
  const pageEnd = Number(range?.pageEnd || 0)
  return pageStart > 0 && pageEnd >= pageStart ? pageEnd - pageStart + 1 : 0
}

export function navigatePage(currentPage, delta, maxPage) {
  return Math.min(Math.max(1, Number(maxPage || 1)), Math.max(1, Number(currentPage || 1) + delta))
}

export function setRangeStart(range, page) {
  if (!range) return
  range.pageStart = Number(page || 1)
  if (Number(range.pageEnd || 0) < range.pageStart) {
    range.pageEnd = range.pageStart
  }
}

export function setRangeEnd(range, page) {
  if (!range) return
  range.pageEnd = Number(page || 1)
  if (Number(range.pageStart || 0) > range.pageEnd) {
    range.pageStart = range.pageEnd
  }
}

export function buildRangeAfter(items, index, maxPage, options = {}) {
  const previous = items[index]
  const next = items[index + 1]
  const pageLimit = Math.max(1, Number(maxPage || 1))
  const start = Math.min(pageLimit, Math.max(1, Number(previous?.pageEnd || 0) + 1))
  const endLimit = next ? Math.max(start, Number(next.pageStart || start) - 1) : start
  const name =
    typeof options.name === 'function' ? options.name(items.length, index) : options.name || `文件${items.length + 1}`
  return {
    name,
    pageStart: start,
    pageEnd: endLimit,
    ...(options.extra || {}),
  }
}

export function insertRangeAfter(items, index, maxPage, options = {}) {
  const item = buildRangeAfter(items, index, maxPage, options)
  items.splice(index + 1, 0, item)
  return item
}

export function removeRangeAt(items, index, currentIndex = 0) {
  items.splice(index, 1)
  return Math.min(currentIndex, Math.max(0, items.length - 1))
}

export function parsePageSelection(input, maxPage = 0) {
  const value = String(input || '').trim()
  if (!value) return []
  const pages = []
  const seen = new Set()
  for (const rawPart of value.split(/[,，\s]+/)) {
    const part = rawPart.trim()
    if (!part) continue
    const rangeMatch = part.match(/^(\d+)\s*[-~—–]\s*(\d+)$/)
    if (rangeMatch) {
      const start = Number(rangeMatch[1])
      const end = Number(rangeMatch[2])
      if (!validPage(start, maxPage) || !validPage(end, maxPage) || start > end) return []
      for (let page = start; page <= end; page += 1) addPage(page, pages, seen)
      continue
    }
    const page = Number(part)
    if (!validPage(page, maxPage)) return []
    addPage(page, pages, seen)
  }
  return pages
}

function validPage(page, maxPage) {
  return Number.isInteger(page) && page > 0 && (!maxPage || page <= maxPage)
}

function addPage(page, pages, seen) {
  if (seen.has(page)) return
  seen.add(page)
  pages.push(page)
}
