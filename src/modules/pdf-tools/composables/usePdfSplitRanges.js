export function splitRangeWarnings(ranges, totalPages) {
  const warnings = []
  const total = Number(totalPages || 0)
  if (!ranges.length || total <= 0) return warnings

  for (const item of ranges) {
    const rawName = String(item.name || '').trim()
    const name = rawName || '未命名'
    const pageStart = Number(item.pageStart || 0)
    const pageEnd = Number(item.pageEnd || 0)
    if (!rawName || !pageStart || !pageEnd) {
      warnings.push(`页段「${name}」缺少名称或起止页`)
      continue
    }
    if (pageStart > pageEnd) {
      warnings.push(`页段「${name}」起始页大于结束页`)
    }
    if (pageEnd > total) {
      warnings.push(`页段「${name}」结束页超过总页数`)
    }
  }

  const sorted = [...ranges]
    .map((item) => ({
      ...item,
      name: String(item.name || '').trim(),
      pageStart: Number(item.pageStart || 0),
      pageEnd: Number(item.pageEnd || 0),
    }))
    .filter((item) => item.name && item.pageStart > 0 && item.pageEnd >= item.pageStart)
    .sort((a, b) => a.pageStart - b.pageStart || a.pageEnd - b.pageEnd)

  let cursor = 1
  for (const item of sorted) {
    if (item.pageStart > cursor) {
      warnings.push(`第 ${cursor}-${item.pageStart - 1} 页未包含在拆分页段中`)
    } else if (item.pageStart < cursor) {
      warnings.push(`页段「${item.name || '未命名'}」与前面的页段存在重叠`)
    }
    cursor = Math.max(cursor, item.pageEnd + 1)
  }
  if (cursor <= total) {
    warnings.push(`第 ${cursor}-${total} 页未包含在拆分页段中`)
  }
  return warnings
}
