export function safeImageWidth(images, cellWidth, imageCellHeight) {
  return Math.max(
    0.1,
    images.reduce((width, image) => {
      return Math.min(width, (imageCellHeight * image.width) / Math.max(1, image.height))
    }, cellWidth),
  )
}

export function safeColumnWidth(pageWidth, marginMm, cols, safetyPadMm = 6) {
  const margin = Math.max(0, Number(marginMm) || 0)
  const pad = Math.max(0, Number(safetyPadMm) || 0)
  const colCount = Math.max(1, Number(cols) || 1)
  const cellWidth = Math.max(1, (pageWidth - margin * 2) / colCount)
  return Math.max(20, Math.floor((cellWidth - pad) * 10) / 10)
}

export function effectiveImageWidth(requested, maximum) {
  return Math.min(Math.max(0.1, Number(requested) || 160), 500, maximum)
}

export function effectivePageWidth(globalWidth, pageScale = 1.0, maxSafeWidth = 500) {
  const w = Math.min(Math.max(0.1, Number(globalWidth) || 160), maxSafeWidth)
  return Math.max(0.1, Math.min(500, w * (Number(pageScale) || 1.0)))
}

export function pairOffsets(count, rows, cols, mode = 'cell-center') {
  const aligns = new Array(count).fill('center')
  const isStack2 = rows === 2 && cols === 1 && count === 2
  const isSide2 = rows === 1 && cols === 2 && count === 2
  if (!isStack2 && !isSide2) return aligns

  switch (mode) {
    case 'page-gather':
      if (isStack2) {
        aligns[0] = 'bottom'
        aligns[1] = 'top'
      } else {
        aligns[0] = 'right'
        aligns[1] = 'left'
      }
      break
    case 'page-spread':
    case 'edge-align':
    case 'gap-max':
      if (isStack2) {
        aligns[0] = 'top'
        aligns[1] = 'bottom'
      } else {
        aligns[0] = 'left'
        aligns[1] = 'right'
      }
      break
    case 'cell-center':
    default:
      break
  }
  return aligns
}

export function pairAlignToXY(align) {
  if (!align || align === 'center') return { x: 'center', y: 'center' }
  if (typeof align === 'object') {
    return {
      x: align.x || 'center',
      y: align.y || 'center',
    }
  }
  switch (align) {
    case 'top':
      return { x: 'center', y: 'top' }
    case 'bottom':
      return { x: 'center', y: 'bottom' }
    case 'left':
      return { x: 'left', y: 'center' }
    case 'right':
      return { x: 'right', y: 'center' }
    default:
      return { x: 'center', y: 'center' }
  }
}

export function estimateTextWidthPt(text, fontSizePt = 8) {
  const lines = String(text || '').split(/\r?\n/)
  let maxWidth = 0
  for (const line of lines) {
    let width = 0
    for (const ch of line) {
      const code = ch.codePointAt(0)
      const isCjk =
        (code >= 0x4e00 && code <= 0x9fff) ||
        (code >= 0x3400 && code <= 0x4dbf) ||
        (code >= 0x20000 && code <= 0x2a6df) ||
        (code >= 0x3000 && code <= 0x303f) ||
        (code >= 0xff01 && code <= 0xff60) ||
        (code >= 0xffe0 && code <= 0xffe6)
      if (isCjk) {
        width += fontSizePt * 1.0
      } else {
        width += fontSizePt * 0.52
      }
    }
    if (width > maxWidth) {
      maxWidth = width
    }
  }
  return maxWidth
}

export function estimateTextWidthMm(text, fontSizePt = 8) {
  return (estimateTextWidthPt(text, fontSizePt) * 25.4) / 72
}

function aabbIntersect(a, b, epsilon = 0.2) {
  const iw = Math.min(a.right, b.right) - Math.max(a.x, b.x)
  const ih = Math.min(a.bottom, b.bottom) - Math.max(a.y, b.y)
  return iw > epsilon && ih > epsilon
}

export function detectPageConflicts({
  images = [],
  grid = { rows: 1, cols: 1 },
  cellWidth = 186,
  imageCellHeight = 100,
  fixedWidthMm = 160,
  pageScale = 1.0,
  scaleMode = 'fixed_width',
  dpi = 300,
  pageWidth,
  pageHeight,
  marginMm = 12,
  showFilename = true,
  captionPosition = 'below',
  captionReserveMm = 0,
  fontSizePt = 8,
  noteFontSizePt = 8,
  pairMode = 'cell-center',
}) {
  const rows = Math.max(1, Number(grid.rows) || 1)
  const cols = Math.max(1, Number(grid.cols) || 1)
  const margin = Math.max(0, Number(marginMm) || 0)
  const cellW = Math.max(1, Number(cellWidth) || 1)
  const hasAnyCaptionText = showFilename || images.some((img) => Boolean(img?.description || img?.note))
  const captionH = hasAnyCaptionText ? Math.max(0, Number(captionReserveMm) || 0) : 0
  const cellH = Math.max(1, Number(imageCellHeight) || 1) + captionH
  const pageW = Number(pageWidth) || (cols * cellW + margin * 2)
  const pageH = Number(pageHeight) || (rows * cellH + margin * 2)

  const effectiveW = effectivePageWidth(fixedWidthMm, pageScale, 500)
  const offsets = pairOffsets(images.length, rows, cols, pairMode)

  // 1. 计算同页每张图和每个标题的几何 AABB（mm 页面坐标系）
  const imageBoxes = []
  const captionBoxes = []

  images.forEach((img, idx) => {
    const r = Math.floor(idx / cols)
    const c = idx % cols
    const cellX = margin + c * cellW
    const cellY = margin + r * cellH

    const imgAreaY = captionPosition === 'above' ? cellY + captionH : cellY
    const imgAreaH = Math.max(1, cellH - captionH)
    const capAreaY = captionPosition === 'above' ? cellY : cellY + imgAreaH

    let drawW, drawH
    if (scaleMode === 'fixed_width') {
      const ratio = img.width > 0 ? img.height / img.width : 1
      drawW = effectiveW
      drawH = drawW * ratio
    } else {
      const safeDpi = dpi === 0 ? 300 : Math.min(1200, Math.max(72, dpi))
      const nativeW = (img.width * 25.4) / safeDpi
      const nativeH = (img.height * 25.4) / safeDpi
      const fitScale = Math.min(cellW / nativeW, imgAreaH / nativeH)
      const scale = (scaleMode === 'original' ? Math.min(fitScale, 1.0) : fitScale) * pageScale
      drawW = nativeW * scale
      drawH = nativeH * scale
    }

    const align = pairAlignToXY(offsets[idx])
    const alignX = align.x
    const alignY = align.y

    let imgX = cellX + (cellW - drawW) / 2
    if (alignX === 'left') imgX = cellX
    else if (alignX === 'right') imgX = cellX + cellW - drawW

    let imgY = imgAreaY + (imgAreaH - drawH) / 2
    if (alignY === 'top') imgY = imgAreaY
    else if (alignY === 'bottom') imgY = imgAreaY + imgAreaH - drawH

    const rectImg = {
      index: idx,
      path: img.path,
      x: imgX,
      y: imgY,
      w: drawW,
      h: drawH,
      right: imgX + drawW,
      bottom: imgY + drawH,
      cellW,
      imgAreaH,
    }
    imageBoxes.push(rectImg)

    const hasTitle = showFilename
    const rawName = img.name || img.path || ''
    const slashIdx = Math.max(rawName.lastIndexOf('/'), rawName.lastIndexOf('\\'))
    const baseName = slashIdx >= 0 ? rawName.slice(slashIdx + 1) : rawName
    const titleText = img.title !== undefined ? img.title : baseName
    const descText = img.description || ''

    if ((hasTitle || descText) && captionH > 0) {
      const titleW = hasTitle && titleText ? estimateTextWidthMm(titleText, fontSizePt) : 0
      const descW = descText ? estimateTextWidthMm(descText, noteFontSizePt || fontSizePt) : 0
      const textW = Math.min(cellW, Math.max(10, Math.max(titleW, descW)))
      const capX = cellX + (cellW - textW) / 2
      captionBoxes.push({
        index: idx,
        x: capX,
        y: capAreaY,
        w: textW,
        h: captionH,
        right: capX + textW,
        bottom: capAreaY + captionH,
      })
    }
  })

  // 2. 检测图-图重叠与图-文重叠
  let hasAnyOverflow = false
  let hasAnyOverlap = false
  let hasAnyCaptionOverlap = false

  const results = imageBoxes.map((box, idx) => {
    const img = images[idx]
    const exceedsImageArea = box.w > box.cellW + 0.5 || box.h > box.imgAreaH + 0.5
    const exceedsPageMargin =
      box.x < margin - 0.5 ||
      box.y < margin - 0.5 ||
      box.right > pageW - margin + 0.5 ||
      box.bottom > pageH - margin + 0.5

    // 与其它图片相交
    let overlap = false
    for (let j = 0; j < imageBoxes.length; j++) {
      if (j === idx) continue
      if (aabbIntersect(box, imageBoxes[j])) {
        overlap = true
        break
      }
    }
    if (overlap) hasAnyOverlap = true

    // 与同页图注相交（包括自己或他人的标题）
    let captionOverlap = false
    for (let k = 0; k < captionBoxes.length; k++) {
      if (aabbIntersect(box, captionBoxes[k], 0.1)) {
        captionOverlap = true
        break
      }
    }
    if (captionOverlap) hasAnyCaptionOverlap = true

    const overflow = exceedsImageArea || exceedsPageMargin || captionOverlap
    if (overflow) hasAnyOverflow = true

    const ratio = img.width > 0 ? img.height / img.width : 1
    const review = !overflow && !overlap && (ratio > 1.55 || ratio < 0.55 || box.w >= box.cellW - 1.0)

    let color = 'ok'
    if (overlap && overflow) color = 'red'
    else if (overlap) color = 'yellow'
    else if (overflow) color = 'green'
    else if (review) color = 'blue'

    return {
      path: img.path,
      drawW: box.w,
      drawH: box.h,
      color,
      overflow,
      overlap,
      captionOverlap,
      review,
    }
  })

  return {
    items: results,
    imageBoxes,
    captionBoxes,
    hasOverflow: hasAnyOverflow,
    hasOverlap: hasAnyOverlap,
    hasCaptionOverlap: hasAnyCaptionOverlap,
  }
}

/**
 * 求解当前页的最佳自适应缩放比例（30% ~ 140%）
 * 消除图-图重叠、图-文压字及超出安全区冲突。
 * 返回 { ok, scale, reason }；不再把仍有冲突的 50% 伪称最佳结果。
 */
export function computeOptimalPageScale(params) {
  const isConflictFree = (scalePercent) => {
    const report = detectPageConflicts({
      ...params,
      pageScale: scalePercent / 100,
    })
    return !report.hasOverflow && !report.hasOverlap && !report.hasCaptionOverlap
  }

  // 若 100% 本身无冲突，基准即为 100%
  if (isConflictFree(100)) {
    return { ok: true, scale: 100, reason: '' }
  }

  // 二分查找 [30, 99] 中最大的无冲突比例
  let low = 30
  let high = 99
  let best = null

  while (low <= high) {
    const mid = Math.floor((low + high) / 2)
    if (isConflictFree(mid)) {
      best = mid
      low = mid + 1
    } else {
      high = mid - 1
    }
  }

  if (best === null) {
    return {
      ok: false,
      scale: null,
      reason: '即使缩小到 30% 仍无法消除重叠/溢出，请减少每页张数、切换横向或改用适应页面',
    }
  }
  return { ok: true, scale: best, reason: '' }
}

export function layoutOptionLabel(value, flow) {
  const options = {
    1: ['1 张', '1 张'],
    '1x2': ['2 张（左右）', '2 张（上下）'],
    '2x1': ['2 张（上下）', '2 张（上下）'],
    '1x3': ['3 张（左右）', '3 张（上下）'],
    4: ['4 张', '4 张（上下）'],
    '2x3': ['6 张', '6 张（上下）'],
    '3x3': ['9 张', '9 张（上下）'],
    custom: ['自定义', '自定义张数（上下）'],
  }
  return options[value]?.[flow ? 1 : 0] || value
}
