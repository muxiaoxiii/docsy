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

function median(values, fallback = 0) {
  if (!values.length) return fallback
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)]
}

function safeColumnWidthMm(pageWidth, marginMm, cols, padMm = 6) {
  const margin = Math.max(0, Number(marginMm) || 0)
  const pad = Math.max(0, Number(padMm) || 0)
  const colCount = Math.max(1, Number(cols) || 1)
  const cellWidth = Math.max(1, (pageWidth - margin * 2) / colCount)
  return Math.max(20, Math.floor((cellWidth - pad) * 10) / 10)
}

function roundWidth(mm) {
  return Math.max(20, Math.floor(Number(mm) / 5) * 5)
}

/**
 * 布局感知推荐：跟随「当前每页张数 + 页面方向」变化，而不是导入时那一套。
 * 统一宽度 = 所有参与图片同时满足「栏宽」和「格高×宽高比」的最小值。
 * 导入推荐（analysis.recommended）继续保留，两者职责分离。
 */
export function recommendForLayout({
  images = [],
  grid = { rows: 2, cols: 1 },
  orientation = 'portrait',
  marginMm = 12,
  printSafetyPadMm = 6,
  showFilename = true,
  perPage = null,
  captionGapMm = 2,
  noteLines = 0,
  filenameLines = 1,
  dpi = 300,
}) {
  const rows = Math.max(1, Number(grid.rows) || 1)
  const cols = Math.max(1, Number(grid.cols) || 1)
  const capacity = perPage != null ? Math.max(1, Number(perPage) || 1) : rows * cols
  const page = orientation === 'landscape' ? { width: 297, height: 210 } : { width: 210, height: 297 }
  const margin = Math.max(0, Number(marginMm) || 12)
  const pad = Math.max(0, Number(printSafetyPadMm) || 6)
  const safeW = safeColumnWidthMm(page.width, margin, cols, pad)

  const captionGap = showFilename || noteLines > 0 ? Math.max(0, Number(captionGapMm) || 0) : 0
  const titleH = showFilename ? ((Math.max(1, filenameLines) * 8 * 25.4) / 72) * 1.32 + 0.45 : 0
  const noteH = noteLines > 0 ? ((noteLines * 8 * 25.4) / 72) * 1.32 + 0.45 : 0
  const safety = titleH + noteH > 0 ? (capacity > 2 ? 1.2 : 2.0) : 0
  const captionReserve = titleH + noteH + safety + captionGap
  const usableH = Math.max(20, page.height - margin * 2 - 2)
  const cellH = usableH / rows
  const imageCellH = Math.max(8, cellH - captionReserve)
  const safeDpi = Math.min(1200, Math.max(72, Number(dpi) || 300))

  const list = images.length ? images : [{ width: 1600, height: 900, path: '' }]
  let maxUnified = safeW
  let limitingIndex = 0
  let limitingBy = '栏宽'

  list.forEach((img, index) => {
    const iw = Math.max(1, Number(img.width) || 1)
    const ih = Math.max(1, Number(img.height) || 1)
    const aspect = iw / ih
    const maxByHeight = imageCellH * aspect
    const maxByNative = (iw * 25.4) / safeDpi
    // 统一宽度必须同时落在：安全栏宽、格高约束、避免放大到失真的上限
    const limit = Math.min(safeW, maxByHeight, Math.max(maxByNative, safeW))
    if (limit < maxUnified) {
      maxUnified = limit
      limitingIndex = index
      limitingBy = maxByHeight < safeW ? '格高×宽高比' : maxByNative < safeW ? '原图像素' : '打印安全栏宽'
    }
  })

  const recommendedWidth = roundWidth(Math.max(20, maxUnified))
  const widths = list.map((img) => Number(img.width) || 0).filter(Boolean)
  const medianW = median(widths, 1600)
  const nativeW = (medianW * 25.4) / safeDpi
  const originalFits =
    list.length > 0 &&
    list.every((img) => {
      const iw = Math.max(1, Number(img.width) || 1)
      const ih = Math.max(1, Number(img.height) || 1)
      const w = (iw * 25.4) / safeDpi
      const h = (ih * 25.4) / safeDpi
      return w <= safeW + 0.5 && h <= imageCellH + 0.5
    }) &&
    nativeW >= 80

  let scaleMode = 'fixed_width'
  if (originalFits && capacity <= 2) {
    scaleMode = 'original'
  }

  const orientLabel = orientation === 'landscape' ? '横页' : '竖页'
  const gridLabel = cols === 1 ? `${capacity} 张竖排` : `${rows}×${cols}`
  const limitShort =
    limitingBy === '格高×宽高比' ? '格高' : limitingBy === '原图像素' ? '像素' : '栏宽'
  const reason = `${gridLabel} · ${orientLabel} → ${recommendedWidth} mm（限：${limitShort} / 栏宽 ${safeW}）`

  return {
    orientation,
    layoutHint: gridLabel,
    perPage: capacity,
    rows,
    cols,
    scale_mode: scaleMode,
    recommended_width_mm: recommendedWidth,
    safe_column_width_mm: safeW,
    image_cell_height_mm: imageCellH,
    caption_reserve_mm: captionReserve,
    limiting_index: limitingIndex,
    limiting_by: limitingBy,
    margin_mm: margin,
    show_filename: showFilename,
    reason,
  }
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
