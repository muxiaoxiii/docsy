/** Pages are explicit. Reordering never silently joins two source images. */
export function imagePages(images) {
  const pages = []
  for (const image of images) {
    let page = pages.find((page) => page.id === image.pageId)
    if (!page) {
      page = { id: image.pageId, images: [] }
      pages.push(page)
    }
    page.images.push(image)
  }
  return pages
}
export function autoImageGrid(count, landscape = false) {
  if (count <= 1) return { rows: 1, cols: 1 }
  const rows = count === 2 ? 2 : count <= 4 ? 2 : count <= 6 ? 3 : count <= 9 ? 3 : 4
  const cols = Math.ceil(count / rows)
  return landscape ? { rows: cols, cols: rows } : { rows, cols }
}
/** Keep every image on its page, adjusting the other dimension first. */
export function fitImageGrid(count, layout, changed = 'cols') {
  const dimension = (value) => Math.max(1, Math.min(4, Math.round(Number(value) || 1)))
  let rows = dimension(layout.rows),
    cols = dimension(layout.cols)
  if (rows * cols < count) {
    if (changed === 'rows') {
      cols = Math.min(4, Math.ceil(count / rows))
      rows = Math.max(rows, Math.ceil(count / cols))
    } else {
      rows = Math.min(4, Math.ceil(count / cols))
      cols = Math.max(cols, Math.ceil(count / rows))
    }
  }
  return { rows, cols }
}
export function joinImagePage(images, path, pageId) {
  const source = images.find((image) => image.path === path)
  const pages = imagePages(images)
  const target = pages.find((page) => page.id === pageId)
  if (!source || !target || source.pageId === pageId || target.images.length >= 16) return images
  return pages
    .map((page) => ({ ...page, images: page.images.filter((image) => image.path !== path) }))
    .map((page) => (page.id === pageId ? { ...page, images: [...page.images, { ...source, pageId }] } : page))
    .flatMap((page) => page.images)
}
export function reorderPageImage(images, from, to, newPageId) {
  if (from === to || !images[from] || !images[to]) return images
  const pages = imagePages(images),
    source = images[from],
    target = images[to]
  const sourcePage = pages.find((page) => page.id === source.pageId)
  const targetPage = pages.find((page) => page.id === target.pageId)
  if (sourcePage === targetPage) {
    const index = sourcePage.images.indexOf(source),
      next = sourcePage.images.indexOf(target)
    sourcePage.images.splice(index, 1)
    sourcePage.images.splice(next, 0, source)
    return pages.flatMap((page) => page.images)
  }
  const keepId = sourcePage.images.length === 1 ? source.pageId : newPageId
  sourcePage.images = sourcePage.images.filter((image) => image.path !== source.path)
  const remaining = pages.filter((page) => page.images.length)
  const insert = remaining.indexOf(targetPage) + (to > from ? 1 : 0)
  remaining.splice(insert, 0, { id: keepId, images: [{ ...source, pageId: keepId }] })
  return remaining.flatMap((page) => page.images)
}
export function splitImagePage(images, path, newPageId) {
  const pages = imagePages(images),
    index = pages.findIndex((page) => page.images.some((image) => image.path === path))
  if (index < 0 || pages[index].images.length === 1) return images
  const source = pages[index].images.find((image) => image.path === path)
  pages[index].images = pages[index].images.filter((image) => image !== source)
  pages.splice(index + 1, 0, { id: newPageId, images: [{ ...source, pageId: newPageId }] })
  return pages.flatMap((page) => page.images)
}
