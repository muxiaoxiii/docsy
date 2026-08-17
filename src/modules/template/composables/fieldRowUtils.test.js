import { describe, expect, it, vi } from 'vitest'
import {
  createPartyTempRowStore,
  displayPartyValue,
  fieldSlotKey,
  filenamePreviewText,
  formatDateValue,
  inputValueForField,
  markRefsForTextRange,
  parseDateParts,
  referenceSelectionFor,
  resolveReferenceValueFromSource,
} from './fieldRowUtils.js'

describe('markRefsForTextRange 坐标传播', () => {
  it('再次拆分已去除连接符的 run 时保留原始起点', () => {
    const row = {
      markId: 'word/document.xml-p2-r14',
      text: '李月春律师',
      markRefs: [{ markId: 'word/document.xml-p2-r14', start: 1, end: 6 }],
      markSegments: [{ markId: 'word/document.xml-p2-r14', text: '李月春律师' }],
    }

    expect(markRefsForTextRange(row, 0, 3)).toEqual([{ markId: 'word/document.xml-p2-r14', start: 1, end: 4 }])
    expect(markRefsForTextRange(row, 3, 5)).toEqual([{ markId: 'word/document.xml-p2-r14', start: 4, end: 6 }])
  })
})

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
      resolveReferenceValueFromSource(
        { mode: 'auto', sourceField: '', sourceSemanticKey: '', sourceIndex: null },
        values,
      ),
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

describe('filenamePreviewText 多项字段', () => {
  it('对象值使用姓名和后缀，不把内部 JSON 写进文件名', () => {
    const tokens = [{ type: 'field', value: '受托人' }]
    const fields = [{ id: 'lawyer', name: '受托人', type: 'party_list' }]
    expect(
      filenamePreviewText(tokens, {
        fields,
        formValues: { lawyer: { text: '李月春', suffix: '律师' } },
      }),
    ).toBe('李月春律师.docx')
  })

  it('多项对象按顿号连接并保留各自后缀', () => {
    const tokens = [{ type: 'field', value: '受托人' }]
    const fields = [{ id: 'lawyer', name: '受托人', type: 'party_list' }]
    expect(
      filenamePreviewText(tokens, {
        fields,
        formValues: {
          lawyer: [
            { text: '李月春', suffix: '律师' },
            { text: '王明', suffix: '实习律师' },
          ],
        },
      }),
    ).toBe('李月春律师、王明实习律师.docx')
  })
})

describe('formatDateValue iso 留空', () => {
  it('全部留空时输出空串而不是 "-  -"', () => {
    expect(formatDateValue('留空', 'iso')).toBe('')
    expect(formatDateValue('', 'iso')).toBe('')
  })

  it('cn/blank 留空仍保留「年月日」骨架供手写', () => {
    expect(formatDateValue('留空', 'cn')).toBe('    年  月  日')
    expect(formatDateValue('留空', 'blank')).toBe('    年  月  日')
  })

  it('iso 正常值不受影响', () => {
    expect(formatDateValue('2026-08-05', 'iso')).toBe('2026-8-5')
  })
})

describe('referenceSelectionFor 引用 slot key 统一', () => {
  const field = { id: 'fld_ref_a', name: '引用A', posIndex: 0 }

  it('fieldSlotKey：主位置不带 #0，跟随位置带 #pos', () => {
    expect(fieldSlotKey(field)).toBe('fld_ref_a')
    expect(fieldSlotKey({ ...field, posIndex: 2 })).toBe('fld_ref_a#2')
  })

  it('主位置能读到 id#0 写法（保存引用来源路径）', () => {
    const selections = { 'fld_ref_a#0': 'field::法院::' }
    expect(referenceSelectionFor(selections, field)).toBe('field::法院::')
  })

  it('主位置能读到 fieldFormKey 写法（填写时选择路径）', () => {
    const selections = { fld_ref_a: 'field::法院::' }
    expect(referenceSelectionFor(selections, field)).toBe('field::法院::')
  })

  it('slot 级 key 优先于字段级 key', () => {
    const follower = { ...field, posIndex: 1 }
    const selections = { 'fld_ref_a#1': 'field::法院::', fld_ref_a: 'field::案号::' }
    expect(referenceSelectionFor(selections, follower)).toBe('field::法院::')
  })

  it('空串（auto 未选择）视为无选择并继续回退', () => {
    const selections = { 'fld_ref_a#0': '', fld_ref_a: 'field::法院::' }
    expect(referenceSelectionFor(selections, field)).toBe('field::法院::')
    expect(referenceSelectionFor({}, field)).toBe('')
  })
})

describe('createPartyTempRowStore 临时行不丢', () => {
  it('空数组时返回稳定的缓存临时行（重渲染不丢已输入内容）', () => {
    const store = createPartyTempRowStore(() => {})
    const first = store.rowsFor('f1', [])
    first[0].text = '张三'
    expect(store.rowsFor('f1', [])[0].text).toBe('张三')
  })

  it('首次输入后 commit 提交为正式值，之后走正式值', () => {
    const commit = vi.fn()
    const store = createPartyTempRowStore(commit)
    store.rowsFor('f1', [])[0].text = '张三'
    store.commit('f1', [])
    expect(commit).toHaveBeenCalledWith('f1', [{ text: '张三', suffix: '' }])
    const real = [{ text: '张三', suffix: '' }]
    expect(store.rowsFor('f1', real)).toBe(real)
  })

  it('临时行为空时不提交（不写入空行）', () => {
    const commit = vi.fn()
    const store = createPartyTempRowStore(commit)
    store.rowsFor('f1', [])
    store.commit('f1', [])
    expect(commit).not.toHaveBeenCalled()
  })

  it('非数组值不展示临时行，保持原行为', () => {
    const store = createPartyTempRowStore(() => {})
    expect(store.rowsFor('f1', undefined)).toEqual([])
  })
})

describe('filenamePreviewText 文件名实时预览', () => {
  const fields = [{ id: 'fld_text_fy', name: '法院', type: 'text' }]

  it('field token 按字段名解析出当前填写值', () => {
    const tokens = [
      { type: 'field', value: '法院' },
      { type: 'literal', value: '-裁定书' },
    ]
    const text = filenamePreviewText(tokens, {
      formValues: { fld_text_fy: '北京知识产权法院' },
      fields,
      manifestName: '模板A',
    })
    expect(text).toBe('北京知识产权法院-裁定书.docx')
  })

  it('未填写时显示占位符，填了以后实时替换', () => {
    const tokens = [{ type: 'field', value: '法院' }]
    expect(filenamePreviewText(tokens, { formValues: {}, fields })).toBe('[法院].docx')
  })

  it('模板名 preset 用 manifest 名称', () => {
    const tokens = [{ type: 'preset', value: '模板名' }]
    expect(filenamePreviewText(tokens, { manifestName: '模板A' })).toBe('模板A.docx')
    expect(filenamePreviewText(tokens, {})).toBe('模板.docx')
  })

  it('只使用可见的字面连接符，不隐式增加分隔符', () => {
    const tokens = [
      { type: 'preset', value: '模板名' },
      { type: 'literal', value: '-' },
      { type: 'field', value: '法院' },
    ]
    expect(
      filenamePreviewText(tokens, {
        manifestName: '所函',
        formValues: { fld_text_fy: '北京知识产权法院' },
        fields,
      }),
    ).toBe('所函-北京知识产权法院.docx')

    expect(
      filenamePreviewText(
        [
          { type: 'preset', value: '模板名' },
          { type: 'field', value: '法院' },
        ],
        {
          manifestName: '所函',
          formValues: { fld_text_fy: '北京知识产权法院' },
          fields,
        },
      ),
    ).toBe('所函北京知识产权法院.docx')
  })

  it('预览与真实保存使用相同的非法文件名字符替换', () => {
    expect(filenamePreviewText([{ type: 'literal', value: '案号：2026/08?17' }], {})).toBe('案号：2026_08_17.docx')
  })

  it('空 token 列表返回空串', () => {
    expect(filenamePreviewText([], {})).toBe('')
  })
})
