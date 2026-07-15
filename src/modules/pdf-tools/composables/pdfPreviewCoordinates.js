export const DEFAULT_PAGE_INFO = {
  widthPt: 595.28,
  heightPt: 841.89,
}

const EDGE_MARGIN_PT = 36
const MIN_PREVIEW_FONT_PX = 8
const PREVIEW_FONT_SCALE = 1.5

export function mmToPt(mm) {
  return Number(mm || 0) * 72 / 25.4
}

export function ptToMm(pt) {
  return Number(pt || 0) * 25.4 / 72
}

export function ptToPercent(pt, dimensionPt) {
  if (!dimensionPt) return 0
  return (Number(pt || 0) / dimensionPt) * 100
}

export function mmToPercent(mm, dimensionPt) {
  return ptToPercent(mmToPt(mm), dimensionPt)
}

export function cleanupZoneStyle(heightMm, pageInfo = DEFAULT_PAGE_INFO) {
  return {
    height: `${mmToPercent(heightMm, pageInfo.heightPt || DEFAULT_PAGE_INFO.heightPt)}%`,
  }
}

export function textOverlayStyle(kind, pageInfo = DEFAULT_PAGE_INFO, config = {}) {
  const widthPt = pageInfo.widthPt || DEFAULT_PAGE_INFO.widthPt
  const heightPt = pageInfo.heightPt || DEFAULT_PAGE_INFO.heightPt
  const align = config.align || 'center'
  const offsetPercent = mmToPercent(config.offsetXMm || 0, widthPt)
  const edgePercent = ptToPercent(EDGE_MARGIN_PT, widthPt)
  const yPercent = kind === 'header'
    ? mmToPercent(config.marginMm, heightPt)
    : 100 - mmToPercent(config.marginMm, heightPt)

  const horizontal = horizontalStyle(align, edgePercent, offsetPercent)
  return {
    ...horizontal,
    top: `${yPercent}%`,
    fontSize: `${Math.max(MIN_PREVIEW_FONT_PX, Number(config.fontSize || 0) * PREVIEW_FONT_SCALE)}px`,
    color: config.color || '#111827',
  }
}

export function bboxOverlayStyle(bbox = {}) {
  const width = bbox.width || DEFAULT_PAGE_INFO.widthPt
  const height = bbox.height || DEFAULT_PAGE_INFO.heightPt
  return {
    left: `${ptToPercent(bbox.x0 || 0, width)}%`,
    top: `${ptToPercent(bbox.y0 || 0, height)}%`,
    width: `${ptToPercent((bbox.x1 || 0) - (bbox.x0 || 0), width)}%`,
    height: `${ptToPercent((bbox.y1 || 0) - (bbox.y0 || 0), height)}%`,
  }
}

function horizontalStyle(align, edgePercent, offsetPercent) {
  if (align === 'left') {
    return {
      left: `${edgePercent + offsetPercent}%`,
      transform: 'translateY(-50%)',
    }
  }
  if (align === 'right') {
    return {
      right: `${edgePercent - offsetPercent}%`,
      transform: 'translateY(-50%)',
    }
  }
  return {
    left: `calc(50% + ${offsetPercent}%)`,
    transform: 'translate(-50%, -50%)',
  }
}
