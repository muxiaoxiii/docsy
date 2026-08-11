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
    // 默认为"用户手动定的名"，保持原有同名合并语义的测试不变；
    // 自动推断场景在各自测试里显式关掉这两个标记。
    _nameManuallySet: true,
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

describe('buildFields 自动推断的同名合并', () => {
  const autoRow = (overrides = {}) =>
    makeRow({ _nameManuallySet: false, _allowSameNameMerge: false, ...overrides })

  it('两处自动推断的同名日期自动加序号（日期、日期2），各自独立填值', () => {
    const rows = [
      autoRow({ rowId: 'r1', type: 'date', name: '日期', label: '日期', semanticKey: '日期', markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      autoRow({ rowId: 'r2', type: 'date', name: '日期', label: '日期', semanticKey: '日期', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(2)
    expect(fields.map((f) => f.name)).toEqual(['日期', '日期2'])
    expect(fields.map((f) => f.label)).toEqual(['日期', '日期2'])
    expect(fields.every((f) => !f.fillAllPositions)).toBe(true)
    expect(fields[0].marks).toEqual(['m1'])
    expect(fields[1].marks).toEqual(['m2'])
    expect(fields[0].id).not.toBe(fields[1].id)
  })

  it('三处同名依次编号为 日期、日期2、日期3', () => {
    const rows = ['r1', 'r2', 'r3'].map((rowId, i) =>
      autoRow({ rowId, type: 'date', name: '日期', label: '日期', markId: `m${i + 1}`, markRefs: [{ markId: `m${i + 1}`, start: null, end: null }] }),
    )
    const fields = buildFields(rows)
    expect(fields.map((f) => f.name)).toEqual(['日期', '日期2', '日期3'])
  })

  it('序号避开已占用的名字（已有 日期2 时新同名编为 日期3）', () => {
    const rows = [
      autoRow({ rowId: 'r1', type: 'date', name: '日期', label: '日期', markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      autoRow({ rowId: 'r2', type: 'date', name: '日期2', label: '日期2', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
      autoRow({ rowId: 'r3', type: 'date', name: '日期', label: '日期', markId: 'm3', markRefs: [{ markId: 'm3', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields.map((f) => f.name)).toEqual(['日期', '日期2', '日期3'])
  })

  it('勾选组同名行不加序号，仍合并为一个字段的多个选项', () => {
    const rows = [
      autoRow({ rowId: 'r1', type: 'radio_group', name: '授权', label: '授权', optionId: 'o1', optionLabel: '一般授权', markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      autoRow({ rowId: 'r2', type: 'radio_group', name: '授权', label: '授权', optionId: 'o2', optionLabel: '特别授权', markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(1)
    expect(fields[0].options).toHaveLength(2)
  })

  it('模板库回读的同名同类型行(_allowSameNameMerge)仍合并为 fillAllPositions', () => {
    const rows = [
      autoRow({ rowId: 'r1', _allowSameNameMerge: true, markId: 'm1', markRefs: [{ markId: 'm1', start: null, end: null }] }),
      autoRow({ rowId: 'r2', _allowSameNameMerge: true, markId: 'm2', markRefs: [{ markId: 'm2', start: null, end: null }] }),
    ]
    const fields = buildFields(rows)
    expect(fields).toHaveLength(1)
    expect(fields[0].fillAllPositions).toBe(true)
    expect(fields[0].marks).toEqual(['m1', 'm2'])
  })
})
