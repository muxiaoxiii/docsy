import { parentDir } from '../../../core/filePath.js'

// Files and pages retain first-seen source order, matching the native exporter.
export function buildOutputPlan(images, perPage, mode) {
  const groups = new Map()
  for (const image of images) {
    const key = mode === 'per_folder' ? parentDir(image.path) : 'merged'
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key).push(image)
  }
  let pageIndex = 0
  return [...groups].map(([source, entries]) => {
    const pages = []
    for (let i = 0; i < entries.length; i += perPage) {
      pages.push({ source, pageIndex: pageIndex++, images: entries.slice(i, i + perPage) })
    }
    return { source, pages }
  })
}

export function migratePaddlerPreferences(raw) {
  const saved = JSON.parse(JSON.stringify(raw))
  if (saved.settings && !Object.hasOwn(saved.settings, 'size_mode')) {
    saved.settings.size_mode = ['fit', 'original'].includes(saved.settings.scale_mode)
      ? saved.settings.scale_mode : 'manual'
  }
  // Legacy page numbers carry no content identity. Never reinterpret them as a global scale.
  if (!saved.pageScalePlanKey) saved.pageScales = {}
  return saved
}
