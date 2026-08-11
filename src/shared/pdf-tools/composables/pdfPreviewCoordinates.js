import { mmToPt, ptToMm } from '../../../core/unitConversion.js'

export { mmToPt, ptToMm }

// 最后的兜底页尺寸（A4）。仅在既无预览组件上报、又无文件检测页尺寸时使用，
// 异形页下比例会失真，正常路径不应走到这里（见 useEvidencePdfPreview.previewPageInfo）。
export const DEFAULT_PAGE_INFO = {
  widthPt: 595.28,
  heightPt: 841.89,
}

const MIN_PREVIEW_FONT_PX = 8

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
  const widthPt = pageInfo?.widthPt || DEFAULT_PAGE_INFO.widthPt
  const heightPt = pageInfo?.heightPt || DEFAULT_PAGE_INFO.heightPt
  const align = config.align || 'center'
  const offsetPercent = mmToPercent(config.offsetXMm || 0, widthPt)
  // 与后端 compute_x 一致：水平边距用用户设置的 marginMm，不再用固定 36pt
  const edgePercent = mmToPercent(config.marginMm || 0, widthPt)
  const yPercent =
    kind === 'header' ? mmToPercent(config.marginMm, heightPt) : 100 - mmToPercent(config.marginMm, heightPt)

  const horizontal = horizontalStyle(align, edgePercent, offsetPercent)
  return {
    ...horizontal,
    top: `${yPercent}%`,
    fontSize: previewFontSize(config.fontSize, widthPt),
    fontFamily: previewFontFamily(config.fontFamily),
    color: config.color || '#111827',
  }
}

// 字号与后端一致：后端按页宽以绝对 pt 绘制，预览 overlay 容器与实际页面同尺寸
// 且声明了 container-type: inline-size（见 PdfJsPreview.vue .docsy-overlay），
// 因此用 cqw（容器内联尺寸百分比）让字号随预览页宽等比缩放：
// fontSize(pt) / widthPt * 100 cqw === 页宽占比，与缩放无关。
// max(8px, …) 保留原来的最小可读字号下限。
function previewFontSize(fontSize, widthPt) {
  const cqw = Number(((Number(fontSize || 0) / widthPt) * 100).toFixed(4))
  return `max(${MIN_PREVIEW_FONT_PX}px, ${cqw}cqw)`
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

function previewFontFamily(fontFamily = 'auto') {
  switch (fontFamily) {
    case 'songti':
      return '"Songti SC", SimSun, serif'
    case 'heiti':
      return '"Heiti SC", "Microsoft YaHei", SimHei, sans-serif'
    case 'kaiti':
      return '"Kaiti SC", KaiTi, serif'
    case 'fangsong':
      return 'FangSong, STFangsong, serif'
    case 'times':
      return '"Times New Roman", Times, serif'
    case 'courier':
      return '"Courier New", Courier, monospace'
    case 'helvetica':
      return 'Helvetica, Arial, sans-serif'
    default:
      return 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
  }
}
