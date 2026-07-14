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
    expect(header.fontSize).toBe('15px')
    expect(footer.right).toBe('6%')
    expect(footer.top).toBe(`${100 - mmToPercent(10, 800)}%`)
  })

  it('maps detected text bbox to page-relative overlay style', () => {
    expect(bboxOverlayStyle({
      x0: 60,
      y0: 20,
      x1: 180,
      y1: 40,
      width: 600,
      height: 800,
    })).toEqual({
      left: '10%',
      top: '2.5%',
      width: '20%',
      height: '2.5%',
    })
  })
})
