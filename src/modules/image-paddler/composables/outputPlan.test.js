import { describe, expect, it } from 'vitest'
import { buildOutputPlan, migratePaddlerPreferences } from './outputPlan.js'

describe('output plan and preference migration', () => {
  it('groups interleaved directories in first-seen order', () => {
    const paths = ['/A/1.png', '/B/1.png', '/A/2.png', '/B/2.png']
    const plan = buildOutputPlan(paths.map(path => ({ path })), 2, 'per_folder')
    expect(plan.map(f => f.pages.map(p => p.images.map(i => i.path)))).toEqual([
      [['/A/1.png', '/A/2.png']], [['/B/1.png', '/B/2.png']],
    ])
    expect(plan[1].pages[0].pageIndex).toBe(1)
  })
  it('uses raw legacy fields before defaults and never promotes a local scale to global', () => {
    const raw = { settings: { fixed_width_mm: 123, scale_mode: 'fixed_width' }, pageScales: { 0: 1.2 } }
    const migrated = migratePaddlerPreferences(raw)
    expect(migrated.settings.size_mode).toBe('manual')
    expect(migrated.settings.fixed_width_mm).toBe(123)
    expect(migrated.globalScalePercent).toBeUndefined()
    expect(migrated.pageScales).toEqual({})
    expect(raw.pageScales).toEqual({ 0: 1.2 })
  })
})
