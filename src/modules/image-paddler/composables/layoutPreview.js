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

export function detectPageConflicts({
  images = [],
  grid = { rows: 1, cols: 1 },
  cellWidth = 186,
  imageCellHeight = 100,
  fixedWidthMm = 160,
  pageScale = 1.0,
  scaleMode = 'fixed_width',
  dpi = 300,
}) {
  const effectiveW = effectivePageWidth(fixedWidthMm, pageScale, cellWidth)
  const isSingleColStack = grid.cols === 1 && grid.rows > 1
  let hasAnyOverflow = false
  let hasAnyOverlap = false

  const results = images.map((img) => {
    let drawW, drawH
    if (scaleMode === 'fixed_width') {
      const ratio = img.width > 0 ? img.height / img.width : 1
      drawW = effectiveW
      drawH = drawW * ratio
    } else {
      const safeDpi = dpi === 0 ? 300 : Math.min(1200, Math.max(72, dpi))
      const nativeW = (img.width * 25.4) / safeDpi
      const nativeH = (img.height * 25.4) / safeDpi
      const fitScale = Math.min(cellWidth / nativeW, imageCellHeight / nativeH) * pageScale
      const scale = scaleMode === 'original' ? Math.min(fitScale, 1.0) : fitScale
      drawW = nativeW * scale
      drawH = nativeH * scale
    }

    const overflow = drawH > imageCellHeight + 0.5 || drawW > cellWidth + 0.5
    if (overflow) hasAnyOverflow = true

    // 在单列多图（如 2x1）垂直排列时，若图高超过各自格高，可能穿透重叠
    const overlap = isSingleColStack && drawH > imageCellHeight + 2.0
    if (overlap) hasAnyOverlap = true

    const ratio = img.width > 0 ? img.height / img.width : 1
    const review = !overflow && !overlap && (ratio > 1.55 || ratio < 0.55 || drawW >= cellWidth - 1.0)

    let color = 'ok'
    if (overlap && overflow) color = 'red'
    else if (overlap) color = 'yellow'
    else if (overflow) color = 'green'
    else if (review) color = 'blue'

    return {
      path: img.path,
      drawW,
      drawH,
      color,
      overflow,
      overlap,
      review,
    }
  })

  return {
    items: results,
    hasOverflow: hasAnyOverflow,
    hasOverlap: hasAnyOverlap,
  }
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
