import { beforeEach, describe, expect, it, vi } from 'vitest'

// ── Browser/tauri environment stubs (must exist before module import) ────────
const hoisted = vi.hoisted(() => {
  const manifests = {}
  const contents = {}
  globalThis.window = {
    localStorage: { getItem: () => null, setItem: () => {}, removeItem: () => {} },
    addEventListener: () => {},
    removeEventListener: () => {},
    setTimeout: () => 0,
    clearTimeout: () => {},
    getSelection: () => ({ removeAllRanges: () => {} }),
  }
  return { manifests, contents }
})

vi.mock('element-plus', () => ({
  ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  ElMessageBox: { confirm: vi.fn(), prompt: vi.fn() },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => '/t/out.docx'),
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
  writeTextFile: vi.fn(async () => {}),
}))

vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(async (cmd, payload) => {
    if (cmd === 'inspect_docsytpl') {
      const manifest = hoisted.manifests[payload?.path]
      return manifest ? { ok: true, data: manifest } : { ok: false, error: 'not found' }
    }
    if (cmd === 'inspect_docsytpl_content') {
      return { ok: true, data: hoisted.contents[payload?.path] || { documentText: '', documentRuns: [] } }
    }
    if (cmd === 'inspect_docx_template') return { ok: true, data: { marks: [], documentText: '', documentRuns: [] } }
    if (cmd === 'get_template_history_context') {
      return { ok: true, data: { lastValues: {}, fieldSuggestions: {}, semanticSuggestions: {}, associationSuggestions: {} } }
    }
    if (cmd === 'list_template_generation_runs') return { ok: true, data: [] }
    if (cmd === 'render_docx_template') return { ok: true, data: '/t/out.docx' }
    return { ok: true, data: null }
  }),
  openPath: vi.fn(async () => ({ ok: true })),
  userFacingError: (error, fallback) => fallback || String(error),
}))

const { useTemplateState } = await import('./useTemplateState.js')
const { tauriCallSafe } = await import('../../../core/tauriBridge.js')
const { open } = await import('@tauri-apps/plugin-dialog')

function makeField(overrides = {}) {
  return {
    id: 'fld_text_x',
    name: '字段X',
    label: '字段X',
    semanticKey: '字段X',
    type: 'text',
    required: false,
    marks: ['m1'],
    markRefs: [{ markId: 'm1', start: null, end: null }],
    options: [],
    fillAllPositions: false,
    dateFormat: '',
    reference: null,
    ...overrides,
  }
}

function makeManifest(fields) {
  return { name: '测试模板', fields, filenameTemplate: null }
}

const caseField = makeField({
  id: 'fld_text_case',
  name: '案号',
  label: '案号',
  semanticKey: '案号',
})
const fixedRefField = makeField({
  id: 'fld_reference_fixed',
  name: '引用1',
  label: '引用1',
  semanticKey: '案号',
  type: 'reference',
  marks: ['m2'],
  markRefs: [{ markId: 'm2', start: null, end: null }],
  reference: { sourceMode: 'field', sourceField: '案号', sourceSemanticKey: '', sourceIndex: null },
})
const autoRefField = makeField({
  id: 'fld_reference_auto',
  name: '引用2',
  label: '引用2',
  semanticKey: '',
  type: 'reference',
  marks: ['m3'],
  markRefs: [{ markId: 'm3', start: null, end: null }],
  reference: null,
})
const dateField = makeField({
  id: 'fld_date_d',
  name: '日期',
  label: '日期',
  semanticKey: '日期',
  type: 'date',
  dateFormat: 'cn_full',
  marks: ['m4'],
  markRefs: [{ markId: 'm4', start: null, end: null }],
})
const partyField = makeField({
  id: 'fld_party_list_p',
  name: '当事人',
  label: '当事人',
  semanticKey: '当事人',
  type: 'party_list',
  marks: ['m5'],
  markRefs: [{ markId: 'm5', start: null, end: null, optionalRule: { enabled: true, removeEmptyPrefix: '', removeEmptySuffix: '律师' } }],
})

async function openTpl(state, path, fields) {
  hoisted.manifests[path] = makeManifest(fields)
  await state.openHistoryTemplate(path)
}

function renderCalls() {
  return tauriCallSafe.mock.calls.filter(([cmd]) => cmd === 'render_docx_template')
}

beforeEach(() => {
  tauriCallSafe.mockClear()
  for (const key of Object.keys(hoisted.manifests)) delete hoisted.manifests[key]
  for (const key of Object.keys(hoisted.contents)) delete hoisted.contents[key]
})

describe('useTemplateState 填写预览', () => {
  it('按稳定 tag 匹配已保存模板，run 序号变化后仍显示正文', async () => {
    const field = makeField({
      id: 'fld_party_list_79d05910',
      name: '受托人',
      label: '受托人',
      type: 'party_list',
      marks: ['old-r13', 'old-r14'],
      markRefs: [
        { markId: 'old-r13', tag: 'fld_party_list_79d05910.ref.1' },
        { markId: 'old-r14', tag: 'fld_party_list_79d05910.ref.2' },
      ],
    })
    hoisted.contents['/t/letter.docsytpl'] = {
      documentText: '',
      documentRuns: [
        { id: 'new-r15', paragraphIndex: 0, text: '委托' },
        { id: 'new-r16', paragraphIndex: 0, text: '{{fld_party_list_79d05910.ref.1}}' },
        { id: 'new-r17', paragraphIndex: 0, text: '律师、' },
        { id: 'new-r18', paragraphIndex: 0, text: '{{fld_party_list_79d05910.ref.2}}' },
        { id: 'new-r19', paragraphIndex: 0, text: '律师。' },
      ],
    }
    const state = useTemplateState()
    await openTpl(state, '/t/letter.docsytpl', [field])
    state.formValues['fld_party_list_79d05910'] = [
      { text: '李月春', suffix: '' },
      { text: '王明', suffix: '' },
    ]
    state.toggleFillPreview()
    await state.reloadFillPreview()

    expect(state.fillPreviewText.value).toBe('委托李月春律师、王明律师。')
    expect(state.fillPreviewText.value).not.toContain('{{')
  })
})

describe('useTemplateState reference 字段', () => {
  it('固定来源引用在渲染时实时解析源字段当前值(不恒空、不过期)', async () => {
    const state = useTemplateState()
    await openTpl(state, '/t/x.docsytpl', [caseField, fixedRefField])

    state.formValues['fld_text_case'] = '（2026）京73行初6803号'
    await state.renderTemplate()
    expect(renderCalls()).toHaveLength(1)
    let args = renderCalls()[0][1].args
    expect(args.values['fld_reference_fixed']).toBe('（2026）京73行初6803号')

    // 源字段改动后再次渲染，引用值必须跟着变(无过期快照)
    state.formValues['fld_text_case'] = '（2027）沪01民初1号'
    await state.renderTemplate()
    args = renderCalls()[1][1].args
    expect(args.values['fld_reference_fixed']).toBe('（2027）沪01民初1号')
  })

  it('onReferenceSelectionChange 写入 referenceSelections(下拉回显)并解析当前值', async () => {
    const state = useTemplateState()
    await openTpl(state, '/t/x.docsytpl', [caseField, autoRefField])
    state.formValues['fld_text_case'] = '（2026）京73行初6803号'

    const field = state.renderableTemplateFields.value.find((f) => f.id === 'fld_reference_auto')
    state.onReferenceSelectionChange(field, 'field::案号::')
    expect(state.referenceSelections['fld_reference_auto']).toBe('field::案号::')
    expect(state.formValues['fld_reference_auto']).toBe('（2026）京73行初6803号')

    // 渲染按选择实时解析:源字段变 → 引用值变
    state.formValues['fld_text_case'] = '（2027）新2号'
    await state.renderTemplate()
    const args = renderCalls()[0][1].args
    expect(args.values['fld_reference_auto']).toBe('（2027）新2号')

    // 清空选择 → 回显清除、渲染为空
    state.onReferenceSelectionChange(field, '')
    expect('fld_reference_auto' in state.referenceSelections).toBe(false)
    await state.renderTemplate()
    expect(renderCalls()[1][1].args.values['fld_reference_auto']).toBe('')
  })
})

describe('useTemplateState 历史记录', () => {
  it('渲染时另存渲染前的原始值(日期不被 cn_full 变形)', async () => {
    const state = useTemplateState()
    await openTpl(state, '/t/x.docsytpl', [dateField, fixedRefField, caseField])
    state.formValues['fld_date_d'] = '2026-08-05'
    state.formValues['fld_text_case'] = '（2026）京73行初6803号'

    await state.renderTemplate()
    const args = renderCalls()[0][1].args
    expect(args.values['fld_date_d']).toBe('二零二六年八月五日')
    expect(args.historyValues['fld_date_d']).toBe('2026-08-05')
    expect(args.historyValues['fld_reference_fixed']).toBe('（2026）京73行初6803号')
  })

  it('历史回填 party_list 保留每项后缀', async () => {
    hoisted.manifests['/t/y.docsytpl'] = makeManifest([partyField])
    const state = useTemplateState()
    await state.applyHistoryRun({
      templatePath: '/t/y.docsytpl',
      fieldValues: { fld_party_list_p: [{ name: '张三', suffix: '律师' }, '李四'] },
    })
    expect(state.formValues['fld_party_list_p']).toEqual([
      { text: '张三', suffix: '律师' },
      { text: '李四', suffix: '' },
    ])
  })

  it('历史回填旧条目(渲染后的中文大写日期)保留原字符串', async () => {
    hoisted.manifests['/t/z.docsytpl'] = makeManifest([dateField])
    const state = useTemplateState()
    await state.applyHistoryRun({
      templatePath: '/t/z.docsytpl',
      fieldValues: { fld_date_d: '二零二六年八月五日' },
    })
    expect(state.formValues['fld_date_d']).toBe('二零二六年八月五日')
  })
})

describe('useTemplateState 撤销栈', () => {
  it('openTemplatePackage 清空撤销栈', async () => {
    const state = useTemplateState()
    state.undoStack.value = [{ label: '旧操作', snapshot: [] }]
    await openTpl(state, '/t/x.docsytpl', [caseField])
    expect(state.undoStack.value).toEqual([])
  })

  it('selectSourceDocx 清空撤销栈', async () => {
    open.mockResolvedValueOnce('/t/a.docx')
    const state = useTemplateState()
    state.undoStack.value = [{ label: '旧操作', snapshot: [] }]
    await state.selectSourceDocx()
    expect(state.undoStack.value).toEqual([])
  })

  it('editTemplateFromLibrary 清空撤销栈', async () => {
    hoisted.manifests['/t/e.docsytpl'] = makeManifest([caseField])
    const state = useTemplateState()
    state.undoStack.value = [{ label: '旧操作', snapshot: [] }]
    await state.editTemplateFromLibrary({ path: '/t/e.docsytpl', name: '测试模板' })
    expect(state.undoStack.value).toEqual([])
  })
})
