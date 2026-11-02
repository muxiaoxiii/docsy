// Subdivide an existing interval. Every source page remains in exactly one row.
export function batchSplitRanges(
  source,
  { mode = 'count', count = 2, pages = 10, lengths = '', starts = '', prefix = '' } = {},
) {
  const start = Number(source?.pageStart),
    end = Number(source?.pageEnd)
  const total = end - start + 1
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 1 || total < 1) throw new Error('请先选择有效页段')
  let sizes
  if (mode === 'starts') {
    const parts = String(starts)
      .trim()
      .split(/[,，;；\s]+/)
      .filter(Boolean)
    if (!parts.length || parts.some((part) => !/^\d+$/.test(part) || !Number.isSafeInteger(Number(part))))
      throw new Error('请输入新增页段的 PDF 起始页，例如 208, 262')
    const cuts = [...new Set(parts.map(Number))].sort((a, b) => a - b)
    if (cuts.some((page) => page <= start || page > end))
      throw new Error(`新增起点应在 ${start + 1}–${end} 页之间；原段首页不需要再次拆分`)
    const boundaries = [start, ...cuts, end + 1]
    return boundaries.slice(0, -1).map((page, index) => ({
      name: index === 0 ? source.name : `${String(prefix || '新页段').trim()}${index}`,
      pageStart: page,
      pageEnd: boundaries[index + 1] - 1,
      source: 'manual',
    }))
  } else if (mode === 'count') {
    const n = Number(count)
    if (!Number.isInteger(n) || n < 2 || n > total) throw new Error(`段数应为 2–${total}，每段至少保留一页`)
    sizes = Array.from({ length: n }, (_, index) => Math.floor(total / n) + (index < total % n ? 1 : 0))
  } else if (mode === 'pages') {
    const n = Number(pages)
    if (!Number.isInteger(n) || n < 1) throw new Error('每段页数应为正整数')
    sizes = Array.from({ length: Math.ceil(total / n) }, (_, index) => Math.min(n, total - index * n))
  } else if (mode === 'lengths') {
    const parts = String(lengths)
      .trim()
      .split(/[,，;；\s]+/)
      .filter(Boolean)
    if (
      !parts.length ||
      parts.some((part) => !/^\d+$/.test(part) || !Number.isSafeInteger(Number(part)) || Number(part) < 1)
    ) {
      throw new Error('请输入各段页数，例如 1, 4, 27；每个数字必须为正整数')
    }
    sizes = parts.map(Number)
    const used = sizes.reduce((sum, size) => sum + size, 0)
    if (used > total) throw new Error(`填写的页数共 ${used} 页，超过所选页段的 ${total} 页`)
    if (used < total) sizes.push(total - used)
  } else throw new Error('请选择分段方式')
  if (sizes.length < 2) throw new Error('当前设置只产生一段，请减少每段页数')
  let cursor = start
  const base = String(prefix || source.name || '文件').trim() || '文件'
  return sizes.map((size, index) => {
    const item = { name: `${base}-${index + 1}`, pageStart: cursor, pageEnd: cursor + size - 1, source: 'manual' }
    cursor += size
    return item
  })
}

export function validRangeSubdivision(source, ranges) {
  if (!Array.isArray(ranges) || ranges.length < 2) return false
  let cursor = Number(source?.pageStart)
  for (const range of ranges) {
    if (
      !Number.isInteger(range.pageStart) ||
      !Number.isInteger(range.pageEnd) ||
      range.pageStart !== cursor ||
      range.pageEnd < cursor
    )
      return false
    cursor = range.pageEnd + 1
  }
  return cursor === Number(source?.pageEnd) + 1
}
