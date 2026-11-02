import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('vue', async original => ({ ...(await original()), onMounted: vi.fn(), onUnmounted: vi.fn(), useSSRContext: () => ({ modules: new Set() }) }))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({ useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }) }))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn(), tauriCallQuiet: vi.fn(), openPath: vi.fn(), userFacingError: error => String(error), showLoading: () => 'test-import', hideLoading: vi.fn(), emitOperationUpdate: vi.fn() }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn() }, ElNotification: vi.fn(), ElMessageBox: { confirm: vi.fn() } }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
import { tauriCallQuiet } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessageBox } from 'element-plus'
import EvidencePdfWorkbench from './EvidencePdfWorkbench.vue'
import { buildEvidencePdfRulePayload, createEvidenceFile } from '../../../shared/pdf-tools/composables/useEvidencePdfSession.js'

describe('evidence mixed import recovery', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.stubGlobal('window', { localStorage: { getItem: () => null, setItem: vi.fn() } })
    tauriCallQuiet.mockImplementation(async command => command === 'convert_word_to_pdf'
      ? { ok: false, error: 'Office unavailable' }
      : { ok: true, data: command === 'get_pdf_page_count' ? 3 : false })
  })
  afterEach(() => vi.unstubAllGlobals())
  it('preserves files and rules, blocks partial execution, and accepts a manual PDF after failure', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    const original = { ...createEvidenceFile('/a.pdf'), pages: 3, removeExistingHeader: true, customNote: 'keep' }
    state.overlayFiles.value = [original]
    await state.loadEvidenceFiles(['/fail.docx', '/b.pdf'])
    expect(state.overlayFiles.value.map(item => item.path)).toEqual(['/a.pdf', '/b.pdf'])
    expect(state.overlayFiles.value[0]).toMatchObject({ removeExistingHeader: true, customNote: 'keep' })
    expect(state.failedDocumentImports.value).toHaveLength(1)
    expect(state.canApplyOverlay.value).toBe(false)
    open.mockResolvedValueOnce('/manual/fail.pdf')
    await state.addManualEvidencePdf(state.failedDocumentImports.value[0])
    expect(state.overlayFiles.value.map(item => item.path)).toEqual(['/a.pdf', '/b.pdf', '/manual/fail.pdf'])
    expect(state.failedDocumentImports.value).toHaveLength(0)
    expect(state.canApplyOverlay.value).toBe(true)
    await state.loadEvidenceFiles(['/manual/fail.pdf'])
    expect(state.overlayFiles.value).toHaveLength(3)
  })
  it('cannot remove, reorder or sort the evidence queue during import or processing', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [createEvidenceFile('/b.pdf'), createEvidenceFile('/a.pdf')]
    for (const busy of [state.importingEvidence, state.overlaying]) {
      busy.value = true
      state.removeOverlayFile(0)
      state.moveOverlayFile(0, 1)
      state.reorderOverlayFiles(0, 1)
      state.sortOverlayFiles({ prop: 'name', order: 'ascending' })
      expect(state.overlayFiles.value.map(file => file.path)).toEqual(['/b.pdf', '/a.pdf'])
      busy.value = false
    }
  })
  it('keeps pending header rows, live preview and export in sync while changing the name prefix and starting number', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['9. 证据文件1-原专利号.pdf', '10. 证据文件2-另一专利号.pdf'].map(name => ({ ...createEvidenceFile(`/case/${name}`), pages: 3 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    state.headerMode.value = 'per_file'
    for (const prefix of ['证据', '对比文件']) {
      state.headerPerFilePrefix.value = prefix
      state.numberingDefaults.value.evidenceStart = 8
      const jobs = buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items
      state.overlayRows.value.forEach((file, index) => {
        const row = state.contentRowsForFile(file).find(row => row.source === 'new' && row.kind === 'header')
        expect(row.text).toBe(`${prefix}${8 + index}`)
        expect(row.text).toBe(jobs[index].header.text)
      })
      expect(state.previewHeaderText.value).toBe(`${prefix}8`)
    }
  })
  it('uses extracted original numbers in the list, preview and payload, with individual edits and a missing-segment guard', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['9. 证据文件1-202210346271.7-CN114843352A.pdf', '10. 对比文件8-专利号.pdf'].map(name => ({ ...createEvidenceFile(`/case/${name}`), pages: 3 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    state.headerMode.value = 'filename_extract'
    state.selectedHeaderGroup.value.filenameExtraction = { separator: '-', start: 1, end: 1, removeOrder: true, replaceFrom: '证据文件', replaceTo: '对比文件' }
    const files = state.overlayRows.value
    let jobs = buildEvidencePdfRulePayload(files, state.currentRules.value).items
    expect(jobs.map(job => job.header.text)).toEqual(['对比文件1', '对比文件8'])
    expect(state.previewHeaderText.value).toBe('对比文件1')
    const row = state.contentRowsForFile(files[0]).find(row => row.source === 'new' && row.kind === 'header')
    expect(row.text).toBe(jobs[0].header.text)
    state.finishNewContentRowEdit(files[0], row, '对比文件一')
    jobs = buildEvidencePdfRulePayload(files, state.currentRules.value).items
    expect(jobs[0].header.text).toBe('对比文件一')
    expect(state.previewHeaderText.value).toBe('对比文件一')
    state.selectedHeaderGroup.value.filenameExtraction.start = 3
    state.selectedHeaderGroup.value.filenameExtraction.end = 3
    expect(state.filenameExtractionErrors.value).toHaveLength(1)
    expect(state.canApplyOverlay.value).toBe(false)
    expect(() => buildEvidencePdfRulePayload(files, state.currentRules.value)).toThrow('只有 2 段')
  })
  it('undoes a dialog edit independently of an earlier inline header edit', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 2 }]
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    const file = state.overlayRows.value[0]
    const row = state.contentRowsForFile(file).find(item => item.source === 'new' && item.kind === 'header')
    state.finishNewContentRowEdit(file, row, '行内修改')
    const before = JSON.parse(JSON.stringify(state.selectedHeaderGroup.value))
    const after = { ...before, fileHeaderOverrides: { '/case/a.pdf': '弹窗修改' } }
    state.recordHeaderGroupEdit({ before, after })
    Object.assign(state.selectedHeaderGroup.value, after)
    state.undoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides['/case/a.pdf']).toBe('行内修改')
    state.undoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toBeUndefined()
    state.redoEdit()
    state.redoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides['/case/a.pdf']).toBe('弹窗修改')
  })
  it('preserves later settings and other file overrides when undoing an earlier row edit', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    const file = state.overlayRows.value[0]
    const row = state.contentRowsForFile(file).find(item => item.source === 'new' && item.kind === 'header')
    state.finishNewContentRowEdit(file, row, '第一项')
    state.selectedHeaderGroup.value.align = 'left'
    state.selectedHeaderGroup.value.fileHeaderOverrides['/case/b.pdf'] = '后改第二项'
    state.undoEdit()
    expect(state.selectedHeaderGroup.value.align).toBe('left')
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({ '/case/b.pdf': '后改第二项' })
    state.redoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({ '/case/a.pdf': '第一项', '/case/b.pdf': '后改第二项' })
  })
  it('does not overwrite a later value for the same file when undoing an older edit', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 2 }]
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    const file = state.overlayRows.value[0]
    const row = state.contentRowsForFile(file).find(item => item.source === 'new' && item.kind === 'header')
    state.finishNewContentRowEdit(file, row, '较早页眉')
    state.selectedHeaderGroup.value.fileHeaderOverrides[file.path] = '最后页眉'
    state.undoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides[file.path]).toBe('最后页眉')
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items[0].header.text).toBe('最后页眉')
  })
  it('asks once with all individually edited files before applying a global header change and can undo it', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    state.selectedHeaderGroup.value.fileHeaderOverrides = { '/case/a.pdf': '单独一', '/case/b.pdf': '单独二' }
    ElMessageBox.confirm.mockResolvedValueOnce('confirm')
    await state.setHeaderRulePatch({ mode: 'per_file' })
    expect(ElMessageBox.confirm).toHaveBeenCalledTimes(1)
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({})
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items.map(item => item.header.text)).toEqual(['证据1', '证据2'])
    state.undoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({ '/case/a.pdf': '单独一', '/case/b.pdf': '单独二' })
    state.redoEdit()
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({})
  })
  it('keeps other file headers when updating a single file rule', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.globalApplyEnabled.value = false
    state.overlayFiles.value.forEach((file, index) => { file.headerGroups[0].fileHeaderOverrides = { [file.path]: `特设${index}` } })
    await state.setHeaderRulePatch({ mode: 'per_file' })
    expect(ElMessageBox.confirm).not.toHaveBeenCalled()
    expect(state.overlayFiles.value[0].headerGroups[0].fileHeaderOverrides).toEqual({})
    expect(state.overlayFiles.value[1].headerGroups[0].fileHeaderOverrides).toEqual({ '/case/b.pdf': '特设1' })
    state.undoEdit()
    expect(state.overlayFiles.value[0].headerGroups[0].fileHeaderOverrides).toEqual({ '/case/a.pdf': '特设0' })
  })
  it('keeps an inline footer or page-number edit on its own file and can apply a later global value', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.insertHeaderFooterEnabled.value = true
    state.globalApplyEnabled.value = true
    state.footerInsertEnabled.value = true
    state.footerTextContent.value = '全局页脚'
    state.footerEnabled.value = true
    const file = state.overlayRows.value[0]
    let row = state.contentRowsForFile(file).find(item => item.kind === 'footerText' && item.source === 'new')
    state.finishNewContentRowEdit(file, row, '单独页脚')
    row = state.contentRowsForFile(file).find(item => item.kind === 'pageNumber' && item.source === 'new')
    state.finishNewContentRowEdit(file, row, '第{page}页')
    let jobs = buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items
    expect(jobs.map(job => job.footer.text)).toEqual(['单独页脚', '全局页脚'])
    expect(jobs.map(job => job.extraOverlays.find(item => item.artifactKind === 'PageNumber').text)).toEqual(['第{page}页', '{page}/{total}'])
    ElMessageBox.confirm.mockResolvedValueOnce('confirm')
    await state.setOtherTextRulePatch('footerText', { text: '新全局页脚' })
    jobs = buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items
    expect(jobs.map(job => job.footer.text)).toEqual(['新全局页脚', '新全局页脚'])
    state.undoEdit()
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items[0].footer.text).toBe('单独页脚')
    ElMessageBox.confirm.mockResolvedValueOnce('confirm')
    await state.setOtherTextRulePatch('pageNumber', { template: '- {page} -' })
    jobs = buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items
    expect(jobs.map(job => job.extraOverlays.find(item => item.artifactKind === 'PageNumber').text)).toEqual(['- {page} -', '- {page} -'])
    state.undoEdit()
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items[0].extraOverlays.find(item => item.artifactKind === 'PageNumber').text).toBe('第{page}页')
  })
  it('confirms and undoes a global evidence-number change affecting individual headers', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    state.selectedHeaderGroup.value.mode = 'per_file'
    state.selectedHeaderGroup.value.fileHeaderOverrides = { '/case/a.pdf': '特设1', '/case/b.pdf': '特设2' }
    ElMessageBox.confirm.mockResolvedValueOnce('confirm')
    await state.setEvidenceStart(21)
    expect(ElMessageBox.confirm).toHaveBeenCalledTimes(1)
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items.map(item => item.header.text)).toEqual(['证据21', '证据22'])
    state.undoEdit()
    expect(state.numberingDefaults.value.evidenceStart).toBe(1)
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({ '/case/a.pdf': '特设1', '/case/b.pdf': '特设2' })
    state.redoEdit()
    expect(state.numberingDefaults.value.evidenceStart).toBe(21)
  })
  it('leaves every individual override intact if the global change is cancelled', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 2 }]
    state.globalApplyEnabled.value = true
    state.selectedHeaderGroup.value.fileHeaderOverrides = { '/case/a.pdf': '保留' }
    ElMessageBox.confirm.mockRejectedValueOnce('cancel')
    await state.setHeaderRulePatch({ mode: 'per_file' })
    expect(state.selectedHeaderGroup.value.mode).toBe('filename')
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides).toEqual({ '/case/a.pdf': '保留' })
  })
  it('asks before switching independent files to global settings and makes the switch undoable', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = ['/case/a.pdf', '/case/b.pdf'].map(path => ({ ...createEvidenceFile(path), pages: 2 }))
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = false
    state.overlayFiles.value[0].headerGroups[0].mode = 'custom'
    state.overlayFiles.value[0].headerGroups[0].text = '单独页眉'
    ElMessageBox.confirm.mockResolvedValueOnce('confirm')
    await state.changeGlobalApply(true)
    expect(ElMessageBox.confirm).toHaveBeenCalledTimes(1)
    expect(state.globalApplyEnabled.value).toBe(true)
    expect(state.overlayFiles.value[0].headerGroups[0].text).toBe('单独页眉')
    state.undoEdit()
    expect(state.globalApplyEnabled.value).toBe(false)
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items[0].header.text).toBe('单独页眉')
    state.redoEdit()
    expect(state.globalApplyEnabled.value).toBe(true)
  })
  it('leaves independent settings active when global application is cancelled', async () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 2 }]
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = false
    state.overlayFiles.value[0].headerGroups[0].mode = 'custom'
    state.overlayFiles.value[0].headerGroups[0].text = '保留独立设置'
    ElMessageBox.confirm.mockRejectedValueOnce('cancel')
    await state.changeGlobalApply(true)
    expect(state.globalApplyEnabled.value).toBe(false)
    expect(state.overlayFiles.value[0].headerGroups[0].text).toBe('保留独立设置')
  })
  it('edits the current rule even if an older list row still references a replaced group', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 2 }]
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    const file = state.overlayRows.value[0]
    const row = state.contentRowsForFile(file).find(item => item.source === 'new' && item.kind === 'header')
    state.headerGroupsModel.value = [{ ...state.selectedHeaderGroup.value, text: '较新规则' }]
    state.finishNewContentRowEdit(file, row, '最后修改')
    expect(state.selectedHeaderGroup.value.fileHeaderOverrides[file.path]).toBe('最后修改')
    expect(buildEvidencePdfRulePayload(state.overlayRows.value, state.currentRules.value).items[0].header.text).toBe('最后修改')
  })
  it('shows the first-page sample from actual ranges and numbering formats', () => {
    const state = EvidencePdfWorkbench.setup({ workflow: 'merge' }, { expose: vi.fn() })
    state.overlayFiles.value = [{ ...createEvidenceFile('/case/a.pdf'), pages: 3 }]
    state.insertHeaderFooterEnabled.value = true
    state.headerInsertEnabled.value = true
    state.globalApplyEnabled.value = true
    state.selectedHeaderGroup.value.pageStart = 2
    expect(state.firstHeaderPreview.value).toBe('')
    state.selectedHeaderGroup.value.pageStart = 1
    expect(state.firstHeaderPreview.value).toBe('a')
    state.footerEnabled.value = true
    state.pageNumberShowTotal.value = false
    state.selectedPageNumberGroup.value.template = '{range}'
    expect(state.firstFooterPreview.value).toBe('1')
  })
})
