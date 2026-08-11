import { describe, expect, it } from 'vitest'
import { buildFields } from './useFieldNormalization.js'

function makeRow(overrides = {}) {
  return {
    rowId: 'r1',
    enabled: true,
    type: 'text',
    name: '案号',
    label: '案号',
    semanticKey: '案号',
    text: '（2026）京73行初6803号',
    required: false,
    optionalWhenEmpty: false,
    optionalScope: 'position',
    optionalPrefix: '',
    optionalSuffix: '',
    markId: 'm1',
    markRefs: [{ markId: 'm1', start: null, end: null }],
    ...overrides,
  }
}

describe('buildFields 同名同类型字段', () => {
  it('合并为一个 fillAllPositions 字段，保留原类型(不改成自引用 reference)', () => {
    const rows = [
      makeRow({ rowId: 'r1', markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      makeRow({ rowId: 'r2', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(1)
    expect(fields[0].type).toBe('text')
    expect(fields[0].name).toBe('案号')
    expect(fields[0].fillAllPositions).toBe(true)
    expect(fields[0].reference).toBeNull()
    expect(fields[0].marks).toEqual(['m1', 'm2'])
  })

  it('两处同名日期字段同样合并为一个 date 字段', () => {
    const rows = [
      makeRow({ rowId: 'r1', type: 'date', name: '日期', label: '日期', markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      makeRow({ rowId: 'r2', type: 'date', name: '日期', label: '日期', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(1)
    expect(fields[0].type).toBe('date')
    expect(fields[0].fillAllPositions).toBe(true)
  })

  it('单个位置的字段不置 fillAllPositions', () => {
    const fields = buildFields([makeRow()])
    expect(fields).toHaveLength(1)
    expect(fields[0].fillAllPositions).toBe(false)
  })

  it('同名但类型不同的行仍是两个独立字段', () => {
    const rows = [
      makeRow({ rowId: 'r1', type: 'text' }),
      makeRow({ rowId: 'r2', type: 'date', name: '案号', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(2)
    expect(fields.map((f) => f.type).sort()).toEqual(['date', 'text'])
    expect(fields.every((f) => !f.fillAllPositions)).toBe(true)
  })

  it('用户显式设置的 reference 字段保留其引用配置', () => {
    const rows = [
      makeRow({ rowId: 'r1', name: '案号' }),
      makeRow({
        rowId: 'r2',
        type: 'reference',
        name: '案号引用',
        label: '案号引用',
        markId: 'm2',
        markRefs: [{ markId: 'm2', start: null, end: null }],
        referenceSourceMode: 'field',
        referenceSourceField: '案号',
        referenceSourceSemanticKey: '',
        referenceSourceIndex: null,
      }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(2)
    const ref = fields.find((f) => f.type === 'reference')
    expect(ref.reference).toMatchObject({ sourceMode: 'field', sourceField: '案号' })
  })
})
