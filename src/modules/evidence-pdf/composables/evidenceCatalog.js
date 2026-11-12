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
