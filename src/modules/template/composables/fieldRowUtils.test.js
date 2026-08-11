import { describe, expect, it } from 'vitest'
import {
  displayPartyValue,
  formatDateValue,
  inputValueForField,
  parseDateParts,
  resolveReferenceValueFromSource,
} from './fieldRowUtils.js'

describe('parseDateParts 容错', () => {
  it('正常解析数字日期', () => {
    expect(parseDateParts('2026-08-05')).toEqual({ y: 2026, m: 8, d: 5 })
    expect(parseDateParts('2026年8月5日')).toEqual({ y: 2026, m: 8, d: 5 })
    expect(parseDateParts('留空')).toEqual({ y: 0, m: 0, d: 0 })
  })

  it('中文大写日期等不可解析输入返回 null(而不是 {0,0,0} 空白)', () => {
    expect(parseDateParts('二零二六年八月五日')).toBeNull()
  })

  it('formatDateValue 对不可解析值保留原字符串', () => {
    expect(formatDateValue('二零二六年八月五日', 'cn_full')).toBe('二零二六年八月五日')
    expect(formatDateValue('2026-08-05', 'cn_full')).toBe('二零二六年八月五日')
    expect(formatDateValue('2026-08-05', 'iso')).toBe('2026-8-5')
  })
})

describe('inputValueForField 历史回填', () => {
  it('party_list 保留每项后缀', () => {
    const field = { type: 'party_list' }
    const value = ['王五', { name: '张三', suffix: '律师' }, { name: '李四', suffix: '实习律师' }]
    expect(inputValueForField(field, value)).toEqual([
      { text: '王五', suffix: '' },
      { text: '张三', suffix: '律师' },
      { text: '李四', suffix: '实习律师' },
    ])
  })

  it('非 party_list 字段原样透传', () => {
    expect(inputValueForField({ type: 'date' }, '2026-08-05')).toBe('2026-08-05')
    expect(inputValueForField({ type: 'text' }, 'abc')).toBe('abc')
  })
})

describe('resolveReferenceValueFromSource 引用解析', () => {
  const values = {
    案号: '（2026）京73行初6803号',
    当事人: [{ name: '张三', suffix: '律师' }, '李四'],
  }

  it('自动来源(未选择)解析为空', () => {
    expect(
      resolveReferenceValueFromSource({ mode: 'auto', sourceField: '', sourceSemanticKey: '', sourceIndex: null }, values),
    ).toBe('')
  })

  it('固定字段来源实时取源字段当前值', () => {
    const source = { mode: 'field', sourceField: '案号', sourceSemanticKey: '', sourceIndex: null }
    expect(resolveReferenceValueFromSource(source, values)).toBe('（2026）京73行初6803号')
    expect(resolveReferenceValueFromSource(source, { ...values, 案号: '（2027）新1号' })).toBe('（2027）新1号')
  })

  it('party_list 来源保留后缀(全量与按序号)', () => {
    const all = { mode: 'field', sourceField: '当事人', sourceSemanticKey: '', sourceIndex: null }
    expect(resolveReferenceValueFromSource(all, values)).toBe('张三律师、李四')
    const first = { ...all, sourceIndex: 0 }
    expect(resolveReferenceValueFromSource(first, values)).toBe('张三律师')
    const second = { ...all, sourceIndex: 1 }
    expect(resolveReferenceValueFromSource(second, values)).toBe('李四')
  })

  it('语义来源按 semanticKey 取值', () => {
    const source = { mode: 'semantic', sourceField: '', sourceSemanticKey: '案号', sourceIndex: null }
    expect(resolveReferenceValueFromSource(source, values)).toBe('（2026）京73行初6803号')
  })
})

describe('displayPartyValue', () => {
  it('对象保留后缀，字符串原样', () => {
    expect(displayPartyValue({ name: '张三', suffix: '律师' })).toBe('张三律师')
    expect(displayPartyValue({ text: '张三', suffix: '律师' })).toBe('张三律师')
    expect(displayPartyValue('李四')).toBe('李四')
  })
})
