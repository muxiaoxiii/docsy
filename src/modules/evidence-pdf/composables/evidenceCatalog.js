// Physical positions in the merged PDF. Duplex padding is between evidence items.
export function catalogRows(files, duplex = false) {
  let start = 1
  return files.map((file, index) => {
    const pages = Number(file.pages)
    if (!Number.isInteger(pages) || pages <= 0) throw new Error('请先确认所有文件的页数')
    const end = start + pages - 1
    const row = { number: String(index + 1), name: String(file.name || '').replace(/\.[^.]+$/, ''), pageRange: start === end ? String(start) : `${start}–${end}` }
    start = end + 1 + (duplex && pages % 2 ? 1 : 0)
    return row
  })
}

// Result order is the merge order; use processed page counts, not stale list previews.
export function catalogFromMerge(results, listedFiles, duplex, mergedPageCount) {
  if (!results.length || results.length !== listedFiles.length) throw new Error('合并结果与文件列表数量不一致')
  const byPath = new Map(listedFiles.map(file => [file.path, file]))
  const seen = new Set()
  const files = results.map(item => {
    const file = byPath.get(item.inputPath)
    if (!file || seen.has(item.inputPath)) throw new Error('合并结果与文件列表不一致')
    seen.add(item.inputPath)
    return { name: file.name, pages: item.pages }
  })
  const rows = catalogRows(files, duplex)
  const expected = files.reduce((total, file) => total + Number(file.pages) + (duplex && Number(file.pages) % 2 ? 1 : 0), 0)
  if (Number(mergedPageCount) !== expected) throw new Error('合并 PDF 的总页数与目录不一致')
  return rows
}
