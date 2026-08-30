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

/**
 * 智能设置页段起始页：改完 items[index].pageStart 后同步修正前一段结束页，
 * 保持整份页段无缝（无空洞、无重叠）；前一段被完全并入时自动移除，并继续向前合并。
 * 返回调整后该页段在数组中的 index（可能因前段移除而前移）。
 */
export function smartSetRangeStart(items, index, page, totalPages) {
  let i = Math.max(0, Math.min(items.length - 1, Number(index) || 0))
  const item = items[i]
  if (!item) return i
  const total = Math.max(1, Number(totalPages) || 1)
  const target = Math.min(total, Math.max(1, Math.round(Number(page) || 1)))
  item.pageStart = target
  if (item.pageEnd < target) item.pageEnd = target
  while (i > 0) {
    const prev = items[i - 1]
    prev.pageEnd = items[i].pageStart - 1
    if (prev.pageEnd < prev.pageStart) {
      items.splice(i - 1, 1)
      i -= 1
    } else {
      break
    }
  }
  return i
}

/**
 * 智能设置页段结束页：改完 items[index].pageEnd 后同步修正后一段起始页，
 * 保持整份页段无缝；后一段被完全并入时自动移除，并继续向后合并。
 * 返回调整后该页段在数组中的 index。
 */
export function smartSetRangeEnd(items, index, page, totalPages) {
  let i = Math.max(0, Math.min(items.length - 1, Number(index) || 0))
  const item = items[i]
  if (!item) return i
  const total = Math.max(1, Number(totalPages) || 1)
  const target = Math.min(total, Math.max(1, Math.round(Number(page) || 1)))
  item.pageEnd = target
  if (item.pageStart > target) item.pageStart = target
  while (i < items.length - 1) {
    const next = items[i + 1]
    next.pageStart = items[i].pageEnd + 1
    if (next.pageStart > next.pageEnd) {
      items.splice(i + 1, 1)
    } else {
      break
    }
  }
  return i
}

/**
 * 以 page 为起始页插入新页段，结束页与后续页段接续（整份页段保持无缝）：
 * - page 落在某页段中间：原段保留 [段首, page-1]，新段 = [page, 原段尾]
 *   （若原段是最后一段，新段结束页即文档最后一页）
 * - page 恰为某页段起始（含第 1 页）：提出一页，新段 = [page, page]，原段从 page+1 开始
 * - 单页段且 page 就是该页：新段插在其后 [page+1, 下一段起始-1 或最后一页]
 * 返回新页段在数组中的 index；page 越界或无法插入时返回 -1。
 */
export function insertRangeAtPage(items, page, totalPages, options = {}) {
  const total = Math.max(1, Number(totalPages) || 1)
  const target = Math.min(total, Math.max(1, Math.round(Number(page) || 1)))
  const name =
    typeof options.name === 'function' ? options.name(items.length) : options.name || `文件${items.length + 1}`
  const extra = options.extra || {}
  if (!items.length) {
    items.push({ name, pageStart: target, pageEnd: total, ...extra })
    return 0
  }

  const index = items.findIndex((item) => target >= Number(item.pageStart || 0) && target <= Number(item.pageEnd || 0))
  if (index < 0) {
    // 防御：page 不在任何页段内，插到第一个起始页大于它的页段之前
    const insertAt = items.findIndex((item) => Number(item.pageStart || 0) > target)
    const newItem = {
      name,
      pageStart: target,
      pageEnd: insertAt >= 0 ? Number(items[insertAt].pageStart) - 1 : total,
      ...extra,
    }
    if (insertAt >= 0) items.splice(insertAt, 0, newItem)
    else items.push(newItem)
    return insertAt >= 0 ? insertAt : items.length - 1
  }

  const segment = items[index]
  const start = Number(segment.pageStart || 0)
  const end = Number(segment.pageEnd || 0)
  if (target === start) {
    // 快速拆分命令需要幂等：目标页已经是一个页段的起点时，直接
    // 复用该页段并聚焦它，不再把边界页额外提出成单页段。
    if (options.reuseBoundary) return index
    if (end === start) {
      // 单页段：新段插在其后，接续下一页（或到最后一页）
      const newStart = start + 1
      const newEnd = index + 1 < items.length ? Number(items[index + 1].pageStart) - 1 : total
      if (newStart > newEnd) return -1
      items.splice(index + 1, 0, { name, pageStart: newStart, pageEnd: newEnd, ...extra })
      return index + 1
    }
    // 边界：提出一页
    const newItem = { name, pageStart: target, pageEnd: target, ...extra }
    segment.pageStart = target + 1
    items.splice(index, 0, newItem)
    return index
  }
  // 段中：新段承接原段尾部
  const tailEnd = end
  segment.pageEnd = target - 1
  items.splice(index + 1, 0, { name, pageStart: target, pageEnd: tailEnd, ...extra })
  return index + 1
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
