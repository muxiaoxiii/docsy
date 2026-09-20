/**
 * 每页张数 × 排列方式 的布局控件模型。
 * 后端仍使用 layout 字符串（'1' / '2x1' / '1x2' / 'custom' ...），这里负责双向映射，
 * 并生成「跟随当前每页张数 + 页面方向」的布局感知推荐。
 */

export function clampNumber(value, min, max, fallback) {
  const number = Number(value)
  if (!Number.isFinite(number)) return fallback
  return Math.min(max, Math.max(min, Math.round(number)))
}

export function parseLayoutString(layout, customRows = 2, customCols = 2) {
  if (layout === 'custom') {
    return {
      rows: clampNumber(customRows, 1, 8, 2),
      cols: clampNumber(customCols, 1, 8, 2),
    }
  }
  if (String(layout).includes('x')) {
    const [rows, cols] = String(layout).split('x').map(Number)
    return {
      rows: clampNumber(rows, 1, 8, 2),
      cols: clampNumber(cols, 1, 8, 2),
    }
  }
  const count = clampNumber(Number(layout), 1, 64, 4)
  if (count === 1) return { rows: 1, cols: 1 }
  if (count === 2) return { rows: 2, cols: 1 }
  if (count === 3) return { rows: 1, cols: 3 }
  if (count === 4) return { rows: 2, cols: 2 }
  if (count === 6) return { rows: 2, cols: 3 }
  if (count === 9) return { rows: 3, cols: 3 }
  const cols = Math.ceil(Math.sqrt(count))
  return { rows: Math.ceil(count / cols), cols }
}

export const PER_PAGE_CHOICES = [
  { value: 1, label: '1 张' },
  { value: 2, label: '2 张' },
  { value: 3, label: '3 张' },
  { value: 4, label: '4 张' },
  { value: 6, label: '6 张' },
  { value: 9, label: '9 张' },
  { value: 'custom', label: '自定义' },
]

export function isFlowLayoutMode(outputFormat, useTable) {
  return outputFormat === 'docx' && !useTable
}

/** 从 layout 字符串反推每页张数与排列方式（用于旧偏好迁移）。 */
export function controlsFromLayout(layout, customRows, customCols, flow) {
  const grid = parseLayoutString(layout, customRows, customCols)
  const perPage = grid.rows * grid.cols
  if (flow) {
    return { imagesPerPage: perPage, arrangeMode: 'stack' }
  }
  if (layout === 'custom') {
    return { imagesPerPage: 'custom', arrangeMode: 'grid' }
  }
  // 常见预设：直接用张数；排列从 grid 形状反推
  const knownCounts = new Set([1, 2, 3, 4, 6, 9])
  if (!knownCounts.has(perPage)) {
    return { imagesPerPage: 'custom', arrangeMode: 'grid', customRows: grid.rows, customCols: grid.cols }
  }
  let arrangeMode = 'grid'
  if (perPage === 1) arrangeMode = 'single'
  else if (perPage === 2) arrangeMode = grid.cols === 2 ? 'side' : 'stack'
  else if (perPage === 3) arrangeMode = grid.rows === 3 ? 'stack' : 'side'
  else if (perPage === 4) arrangeMode = 'grid'
  else if (perPage === 6) arrangeMode = grid.rows === 3 ? '3x2' : '2x3'
  else if (perPage === 9) arrangeMode = 'grid'
  return { imagesPerPage: perPage, arrangeMode }
}

/** 排列选项：与每页张数、是否表格模式共同决定。 */
export function arrangeOptionsFor(perPage, flow) {
  if (flow) {
    return [{ value: 'stack', label: '上下排列' }]
  }
  const count = perPage === 'custom' ? null : Number(perPage)
  if (count === 1) return [{ value: 'single', label: '整页一张' }]
  if (count === 2) {
    return [
      { value: 'stack', label: '上下' },
      { value: 'side', label: '左右' },
    ]
  }
  if (count === 3) {
    return [
      { value: 'side', label: '横排 1×3' },
      { value: 'stack', label: '竖排 3×1' },
    ]
  }
  if (count === 4) {
    return [
      { value: 'grid', label: '2×2' },
      { value: 'side', label: '横排 1×4' },
      { value: 'stack', label: '竖排 4×1' },
    ]
  }
  if (count === 6) {
    return [
      { value: '2x3', label: '2×3' },
      { value: '3x2', label: '3×2' },
    ]
  }
  if (count === 9) return [{ value: 'grid', label: '3×3' }]
  return [{ value: 'grid', label: '自定义网格' }]
}

/** 由「每页张数 + 排列」生成后端 layout 字符串。 */
export function layoutFromControls({ imagesPerPage, arrangeMode, customRows, customCols, flow }) {
  if (imagesPerPage === 'custom') {
    const rows = clampNumber(customRows, 1, 8, 2)
    const cols = clampNumber(customCols, 1, 8, 2)
    // 无表格时行列会叠成上下一列，这里仍传 custom，后端/前端会做 rows*cols → 1 列
    return { layout: 'custom', customRows: rows, customCols: cols }
  }
  const count = clampNumber(Number(imagesPerPage), 1, 64, 2)
  if (flow) {
    // 无表格：任意张数都按上下一列；用张数字面量，parse 后再叠成 stack
    return { layout: String(count), customRows: null, customCols: null }
  }
  if (count === 1) return { layout: '1', customRows: null, customCols: null }
  if (count === 2) {
    return { layout: arrangeMode === 'side' ? '1x2' : '2x1', customRows: null, customCols: null }
  }
  if (count === 3) {
    return { layout: arrangeMode === 'stack' ? '3x1' : '1x3', customRows: null, customCols: null }
  }
  if (count === 4) {
    if (arrangeMode === 'side') return { layout: '1x4', customRows: null, customCols: null }
    if (arrangeMode === 'stack') return { layout: '4x1', customRows: null, customCols: null }
    return { layout: '4', customRows: null, customCols: null }
  }
  if (count === 6) {
    return { layout: arrangeMode === '3x2' ? '3x2' : '2x3', customRows: null, customCols: null }
  }
  if (count === 9) return { layout: '3x3', customRows: null, customCols: null }
  return { layout: String(count), customRows: null, customCols: null }
}

export function perPageFromLayout(layout, customRows, customCols, flow) {
  const grid = parseLayoutString(layout, customRows, customCols)
  return flow ? grid.rows * grid.cols : grid.rows * grid.cols
}

/** 导入推荐与当前布局是否明显不一致（例如导入按 2 张推荐，用户改成了 4 张）。 */
export function importRecommendationDrifts(importRec, currentGrid, currentPerPage) {
  if (!importRec) return false
  const importGrid = parseLayoutString(importRec.layout || '2x1')
  const importPerPage = importGrid.rows * importGrid.cols
  if (importPerPage !== currentPerPage) return true
  if (importGrid.cols !== currentGrid.cols) return true
  return false
}
