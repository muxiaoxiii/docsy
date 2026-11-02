import { describe, expect, it } from 'vitest'
import { computed, reactive } from 'vue'
import { createFilenameHeaderRule, extractFilenameHeader, filenameHeaderParts } from './filenameHeader.js'

describe('filename headers', () => {
  it('updates a reactive preview when adding, editing and removing an individual correction', () => {
    const file = { name: '9. 证据文件8-专利.pdf', path: '/case/a.pdf' }
    const config = reactive(createFilenameHeaderRule())
    const result = computed(() => extractFilenameHeader(file, config))
    expect(result.value.text).toBe('证据文件8')
    config.overrides[file.path] = '对比文件8'
    expect(result.value.text).toBe('对比文件8')
    config.overrides[file.path] = '对比文件八'
    expect(result.value.text).toBe('对比文件八')
    delete config.overrides[file.path]
    expect(result.value.text).toBe('证据文件8')
    config.end = 3
    expect(result.value.error).toContain('只有 2 段')
    config.end = 1
    expect(result.value.error).toBe('')
  })
  const files = [
    '9. 证据文件1-202210346271.7-CN114843352A-太阳能电池片及其制备方法.pdf',
    '7. 对比文件1-201110339394.X-CN102403375A-太阳能硅片绒面结构及其制绒方法.pdf',
    '10. 证据文件2-201911390646.4-CN111129170A-太阳能电池及太阳能电池模组.pdf',
  ].map((name) => ({ name, path: `/case/${name}` }))
  const rule = { ...createFilenameHeaderRule(), replaceFrom: '证据文件', replaceTo: '对比文件' }
  it('keeps each original evidence number across the three case naming schemes', () => {
    expect(files.map((file) => extractFilenameHeader(file, rule).text)).toEqual(['对比文件1', '对比文件1', '对比文件2'])
    expect(files[0].name).toBe('9. 证据文件1-202210346271.7-CN114843352A-太阳能电池片及其制备方法.pdf')
  })
  it('selects literal delimiter ranges, including a final range and removal of a leading sort number', () => {
    expect(extractFilenameHeader(files[0], { ...rule, start: 2, end: 3 }).text).toBe('202210346271.7-CN114843352A')
    expect(extractFilenameHeader(files[0], { ...rule, start: 4, end: 0 }).text).toBe('太阳能电池片及其制备方法')
    expect(extractFilenameHeader(files[0], { ...rule, removeOrder: false }).text).toBe('9. 对比文件1')
    expect(filenameHeaderParts({ name: '201910586460.X.pdf' }, rule)).toEqual(['201910586460.X'])
    expect(extractFilenameHeader({ name: '01．附页__正文.pdf' }, { separator: '__', start: 1, end: 2 }).text).toBe(
      '附页__正文',
    )
  })
  it('reports missing segments and allows an explicit per-file correction', () => {
    const rule = { ...createFilenameHeaderRule(), start: 3, end: 3 }
    const file = { path: '/case/a.pdf', name: '1. 附页.pdf' }
    expect(extractFilenameHeader(file, rule).error).toContain('只有 1 段')
    expect(extractFilenameHeader(file, { ...rule, overrides: { [file.path]: '正文' } })).toEqual({
      text: '正文',
      error: '',
    })
    expect(extractFilenameHeader(file, { ...rule, overrides: { [file.path]: '' } }).error).toBeTruthy()
    expect(extractFilenameHeader(file, { separator: '' }).error).toBeTruthy()
    expect(extractFilenameHeader(file, { start: 1, end: 1, replaceFrom: '附页' }).error).toBeTruthy()
  })
  it('uses the original Word filename instead of the converted temporary path', () => {
    expect(
      extractFilenameHeader({ path: '/tmp/output.pdf', sourcePath: '/case/9. 证据文件8-合同.docx' }, rule).text,
    ).toBe('对比文件8')
  })
})
