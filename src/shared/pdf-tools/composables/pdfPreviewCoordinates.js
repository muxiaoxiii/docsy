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

export function textOverlayStyle(kind, pageInfo = DEFAULT_PAGE_INFO, config = {}, pageNumber = 1) {
  const widthPt = pageInfo?.widthPt || DEFAULT_PAGE_INFO.widthPt
  const heightPt = pageInfo?.heightPt || DEFAULT_PAGE_INFO.heightPt
  let align = config.align || 'center'
  let offsetXMm = config.offsetXMm || 0

  // 双面打印对开页码：长边翻页 (long-edge) 或短边翻页 (short-edge) 且为偶数页时镜像对开
  const isEvenPage = Number(pageNumber || 1) % 2 === 0
  const duplex = config.duplexMode || 'none'
  if (isEvenPage && (duplex === 'long-edge' || duplex === 'short-edge')) {
    if (align === 'left') align = 'right'
    else if (align === 'right') align = 'left'
    offsetXMm = -offsetXMm
  }

  const offsetPercent = mmToPercent(offsetXMm, widthPt)
  // 水平基准边距：优先使用独立的 marginXMm，未设置时默认 10mm；
  // 严格与垂直的 marginMm（距底/距顶）解耦，彻底避免调整距底时沿 45 度对角线跑偏！
  const edgePercent = mmToPercent(config.marginXMm ?? 10, widthPt)
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
