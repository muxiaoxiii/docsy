import { describe, expect, it } from 'vitest'
import {
  bboxOverlayStyle,
  cleanupZoneStyle,
  mmToPercent,
  mmToPt,
  ptToMm,
  textOverlayStyle,
} from './pdfPreviewCoordinates.js'

describe('PDF preview coordinate helpers', () => {
  it('converts millimeters to PDF points and percentages', () => {
    expect(mmToPt(25.4)).toBeCloseTo(72)
    expect(ptToMm(72)).toBeCloseTo(25.4)
    expect(mmToPercent(10, 720)).toBeCloseTo(3.937, 3)
  })

  it('maps cleanup zones to page-relative height', () => {
    expect(cleanupZoneStyle(10, { widthPt: 595.28, heightPt: 720 })).toEqual({
      height: `${mmToPercent(10, 720)}%`,
    })
  })

  it('places header from top and footer from bottom in browser coordinates', () => {
    const pageInfo = { widthPt: 600, heightPt: 800 }
    const header = textOverlayStyle('header', pageInfo, {
      align: 'center',
      marginMm: 10,
      fontSize: 10,
    })
    const footer = textOverlayStyle('footer', pageInfo, {
      align: 'right',
      marginMm: 10,
      fontSize: 9,
    })

    expect(header.left).toBe('calc(50% + 0%)')
    expect(header.top).toBe(`${mmToPercent(10, 800)}%`)
    // 字号用 cqw 随页宽缩放：10pt / 600pt * 100 = 1.6667cqw，保留下限 8px
    expect(header.fontSize).toBe('max(8px, 1.6667cqw)')
    // 水平边距与后端一致：使用 marginMm(10mm) 而非固定 36pt
    expect(footer.right).toBe(`${mmToPercent(10, 600)}%`)
    expect(footer.top).toBe(`${100 - mmToPercent(10, 800)}%`)
    expect(footer.fontSize).toBe('max(8px, 1.5cqw)')
  })

  it('scales overlay font size with the actual page width (异形页)', () => {
    // 异形宽页：同样的 12pt 字号在 1200pt 宽页面上占 1cqw
    const wide = textOverlayStyle('header', { widthPt: 1200, heightPt: 600 }, { fontSize: 12 })
    expect(wide.fontSize).toBe('max(8px, 1cqw)')
    // pageInfo 缺省时回退 A4 兜底（595.28pt 宽）
    const fallback = textOverlayStyle('header', null, { fontSize: 12 })
    expect(fallback.fontSize).toBe(`max(8px, ${Number(((12 / 595.28) * 100).toFixed(4))}cqw)`)
  })

  it('maps detected text bbox to page-relative overlay style', () => {
    expect(
      bboxOverlayStyle({
        x0: 60,
        y0: 20,
        x1: 180,
        y1: 40,
        width: 600,
        height: 800,
      }),
    ).toEqual({
      left: '10%',
      top: '2.5%',
      width: '20%',
      height: '2.5%',
    })
  })
})
