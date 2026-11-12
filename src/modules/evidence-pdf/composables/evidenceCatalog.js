import { formatPageNumber } from '../../../shared/pdf-tools/composables/pdfPageNumberRules.js'

// Physical positions in the merged PDF. Duplex padding is between evidence items.
export function catalogRows(files, duplex = false) {
  let start = 1
  return files.map((file, index) => {
    const pages = Number(file.pages)
    if (!Number.isInteger(pages) || pages <= 0) throw new Error('请先确认所有文件的页数')
    const end = start + pages - 1 + (duplex && pages % 2 ? 1 : 0)
    const row = { number: String(index + 1), name: String(file.name || '').replace(/\.[^.]+$/, ''), pageRange: file.catalogPageRange || (start === end ? String(start) : `${start}–${end}`) }
    start = end + 1
    return row
  })
}

// Result order is the merge order; use processed page counts, not stale list previews.
export function catalogFromMerge(results, listedFiles, duplex, mergedPageCount, countBlanks = false) {
  if (!results.length || results.length !== listedFiles.length) throw new Error('合并结果与文件列表数量不一致')
  const byPath = new Map(listedFiles.map(file => [file.path, file]))
  const seen = new Set()
  const files = results.map(item => {
    const file = byPath.get(item.inputPath)
    if (!file || seen.has(item.inputPath)) throw new Error('合并结果与文件列表不一致')
    seen.add(item.inputPath)
    if (Number(file.pages) !== Number(item.pages)) throw new Error('文件页数在处理期间发生变化，请刷新页数后重新合并')
    return { name: file.name, pages: item.pages, catalogPageRange: file.catalogPageRange }
  })
  const rows = catalogRows(files, duplex && countBlanks)
  const expected = files.reduce((total, file) => total + Number(file.pages) + (duplex && Number(file.pages) % 2 ? 1 : 0), 0)
  if (Number(mergedPageCount) !== expected) throw new Error('合并 PDF 的总页数与目录不一致')
  return rows
}

// Treat a numbered separator as the final page of the preceding evidence.
export function separatorPageJobs(files, resolved) {
  return files.map((file, index) => {
    if (Number(file.pages) % 2 !== 1) return null
    const entry = resolved[index]
    const last = Number(file.pages) + 1
    const applies = config => config && Number(config.pageStart || 1) <= last && (!Number(config.pageEnd) || Number(config.pageEnd) >= last)
    const single = config => applies(config) ? { ...config, pageStart: 1, pageEnd: 1, ...(config.artifactKind === 'PageNumber' ? { duplexMode: 'none' } : {}) } : null
    const extraOverlays = (entry?.job.extraOverlays || []).filter(applies).map(single)
    if (!extraOverlays.some(overlay => overlay.artifactKind === 'PageNumber')) {
      throw new Error(`请先为 ${file.name} 的空白分隔页启用页码`)
    }
    return { pageStart: Number(entry.job.pageStart) + Number(file.pages), totalPages: entry.job.totalPages,
      header: single(entry.job.header), footer: single(entry.job.footer), extraOverlays }
  })
}

export function catalogNumberRange(overlays, pageStart) {
  return (overlays || []).map(overlay => {
    const first = Number(pageStart || 1) + Number(overlay.pageStart) - 1 + Number(overlay.numberOffset || 0)
    const last = Number(pageStart || 1) + Number(overlay.pageEnd) - 1 + Number(overlay.numberOffset || 0)
    const a = formatPageNumber(first, overlay.numberStyle)
    const b = formatPageNumber(last, overlay.numberStyle)
    return first === last ? a : `${a}–${b}`
  }).join('；')
}
