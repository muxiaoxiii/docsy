import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfMergedImport } from './useEvidencePdfMergedImport.js'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(), userFacingError: vi.fn(error => String(error)),
  showLoading: vi.fn(), hideLoading: vi.fn(), emitOperationUpdate: vi.fn(),
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn() }, ElMessageBox: {} }))

function setup() {
  const state = {
    overlayFiles: ref([]), overlayOutputDir: ref(''), importingMergedPdf: ref(false),
    detectingMergedImport: ref(false), splittingMergedImport: ref(false), mergedImportPlan: ref(null), mergedImportPlans: ref([]),
    selectedMergedImportIndex: ref(0), selectedOverlayIndex: ref(0), previewPage: ref(1),
    truePreview: ref(null), previewMaxPage: ref(10), cleanupHeaderHeightMm: ref(18),
    cleanupFooterHeightMm: ref(18), safeRefreshPreview: vi.fn(),
    splitNamePrefix: ref(''), splitNameSuffix: ref(''), splitNameDateValue: ref(''),
    splitNameSeparator: ref(''), splitNameCustomSeparator: ref(''),
    splitReplacementOutputDir: ref(''), removeBlankPages: ref(true),
    refreshPreview: vi.fn(), applyWorkflowDefaults: vi.fn(),
  }
  return { state, api: useEvidencePdfMergedImport(state) }
}

describe('合并证据拆分导入', () => {
  beforeEach(() => vi.clearAllMocks())

  it('导入不自动扫描，手动检测后生成拆分页段', async () => {
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: 10 }).mockResolvedValueOnce({
      ok: true, data: { cleanupCandidates: [{ source: 'artifact', text: '证据1' }], totalPages: 10, pagesAnalyzed: 10, items: [
        { name: '证据1', pageStart: 1, pageEnd: 4 },
        { name: '证据2', pageStart: 5, pageEnd: 10 },
      ] },
    })
    const { state, api } = setup()
    await api.importMergedPdfAsEvidence(['/docs/merged.pdf'])
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
    expect(state.mergedImportPlan.value.items).toHaveLength(1)
    await api.detectMergedImportPlan()
    expect(tauriCallSafe).toHaveBeenNthCalledWith(2, 'inspect_merged_evidence_pdf', expect.any(Object))
    expect(state.mergedImportPlan.value.items).toHaveLength(2)
    expect(state.mergedImportPlan.value.cleanupCandidates[0].selected).toBe(false)
    expect(state.overlayFiles.value).toEqual([])
    expect(state.detectingMergedImport.value).toBe(false)
  })

  it('多文件导入生成独立方案，追加时保留已有页段且不自动检测', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/docs/current.pdf'])
    state.mergedImportPlan.value.items[0].name = '已修改'
    await api.importMergedPdfAsEvidence(['/docs/one.pdf', '/docs/two.pdf'])
    expect(tauriCallSafe).toHaveBeenCalledTimes(3)
    expect(state.mergedImportPlans.value).toHaveLength(3)
    expect(state.mergedImportPlans.value.find(plan => plan.inputPath === '/docs/current.pdf').items[0].name).toBe('已修改')
    expect(state.overlayFiles.value).toEqual([])
  })

  it('检测失败保留可重试的页段，不进入批量处理', async () => {
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: 10 }).mockResolvedValueOnce({ ok: false, error: '缺少 pdftotext' })
    const { state, api } = setup()
    await api.importMergedPdfAsEvidence(['/docs/merged.pdf'])
    await api.detectMergedImportPlan()
    expect(state.mergedImportPlan.value.warnings).toContain('缺少 pdftotext')
    expect(state.mergedImportPlan.value.detectionAttempted).toBe(true)
    expect(state.mergedImportPlan.value.items).toHaveLength(1)
    expect(state.overlayFiles.value).toEqual([])
  })

  it('拆分结果页数扣除空白页且不自动检测元素', async () => {
    const { state, api } = setup()
    state.mergedImportPlan.value = { inputPath: '/docs/source.pdf', outputDir: '/out', totalPages: 10, items: [{ name: '证据1', pageStart: 1, pageEnd: 10, source: 'artifact' }], cleanupCandidates: [
      { selected: true, region: 'footer', normalizedText: '第 {page} 页', pageRange: { start: 1, end: 10 } },
      { selected: false, region: 'header', normalizedText: '证据1', pageRange: { start: 1, end: 10 } },
    ] }
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { outputs: [{ name: '证据1', pageStart: 1, pageEnd: 10, removedBlankPages: 2, outputPath: '/out/1.pdf' }], failed: [], warnings: [] } })
    await api.executeMergedImportPlan()
    expect(state.overlayFiles.value[0].pages).toBe(8)
    expect(tauriCallSafe.mock.calls[0][1].args.cleanupTargets).toEqual([{ region: 'footer', normalizedText: '第 {page} 页', pageStart: 1, pageEnd: 10 }])
    expect(state.mergedImportPlan.value).toBeNull()
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
  })

  it('检测期间禁止取消页段和执行拆分', async () => {
    const { state, api } = setup()
    state.mergedImportPlan.value = { inputPath: '/docs/source.pdf' }
    state.detectingMergedImport.value = true
    api.cancelMergedImportPlan()
    await api.executeMergedImportPlan()
    expect(state.mergedImportPlan.value).not.toBeNull()
    expect(tauriCallSafe).not.toHaveBeenCalled()
  })

  it('逐份检测，后一份失败不覆盖前一份的结果', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    let finishFirst
    const paths = []
    tauriCallSafe.mockImplementation((_command, { args }) => {
      paths.push(args.inputPath)
      if (paths.length === 1) return new Promise(resolve => { finishFirst = resolve })
      return Promise.resolve({ ok: false, error: '第二份损坏' })
    })
    const run = api.detectAllMergedImports()
    expect(paths).toEqual(['/a/report.pdf'])
    api.activateMergedImportPlan(state.mergedImportPlans.value[1])
    expect(state.mergedImportPlan.value.inputPath).toBe('/a/report.pdf')
    finishFirst({ ok: true, data: { totalPages: 10, pagesAnalyzed: 10, items: [{ name: '证据甲', pageStart: 1, pageEnd: 10 }] } })
    await run
    expect(paths).toEqual(['/a/report.pdf', '/b/report.pdf'])
    expect(state.mergedImportPlans.value[0].items[0].name).toBe('证据甲')
    expect(state.mergedImportPlans.value[1].detectionError).toBe('第二份损坏')
    expect(api.batchRunning.value).toBe(false)
  })

  it('切换同名文件时隔离命名、清除标记与撤销历史', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    const [first, second] = state.mergedImportPlans.value
    state.splitNamePrefix.value = '甲'
    state.removeBlankPages.value = false
    first.cleanupCandidates = [{ selected: true }]
    api.splitMergedImportAtPage(5)
    expect(api.canUndoMergedImport.value).toBe(true)
    api.activateMergedImportPlan(second)
    expect(state.splitNamePrefix.value).toBe('')
    expect(state.removeBlankPages.value).toBe(true)
    expect(api.canUndoMergedImport.value).toBe(false)
    expect(second.cleanupCandidates).toBeUndefined()
    state.splitNamePrefix.value = '乙'
    api.activateMergedImportPlan(first)
    expect(state.splitNamePrefix.value).toBe('甲')
    expect(state.removeBlankPages.value).toBe(false)
    expect(first.cleanupCandidates[0].selected).toBe(true)
    api.undoMergedImportEdit()
    expect(first.items).toHaveLength(1)
    expect(second.items).toHaveLength(1)
  })

  it('只顺序拆分已核对文件，按源文件保存输出与清除参数', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf', '/c/report.pdf'])
    const [first, second, third] = state.mergedImportPlans.value
    state.splitNamePrefix.value = '甲'
    api.activateMergedImportPlan(second)
    state.splitNamePrefix.value = '乙'
    api.activateMergedImportPlan(third)
    first.reviewed = second.reviewed = true
    first.detectionAttempted = second.detectionAttempted = true
    first.cleanupCandidates = [{ selected: true, region: 'header', normalizedText: '甲', pageRange: { start: 1, end: 10 } }]
    const requests = []
    tauriCallSafe.mockImplementation(async (_command, { args }) => {
      requests.push(args)
      return { ok: true, data: { outputs: [{ ...args.items[0], outputPath: args.outputDir + '/证据.pdf' }] } }
    })
    await api.splitReviewedMergedImports()
    expect(requests.map(args => args.inputPath)).toEqual(['/a/report.pdf', '/b/report.pdf'])
    expect(requests.map(args => args.outputDir)).toEqual(['/a/report-分项', '/b/report-分项'])
    expect(requests.map(args => args.items[0].name)).toEqual(['甲目录', '乙目录'])
    expect(requests[0].cleanupTargets).toHaveLength(1)
    expect(requests[1].cleanupTargets).toHaveLength(0)
    expect(state.overlayFiles.value.map(file => file.sourceInputPath)).toEqual(['/a/report.pdf', '/b/report.pdf'])
    expect(first.splitStatus).toBe('complete')
    expect(second.splitStatus).toBe('complete')
    expect(third.splitRequest).toBeUndefined()
  })

  it('部分失败后仅重试未输出页段，保持首次提交的命名和清除设置', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf'])
    const plan = state.mergedImportPlan.value
    plan.items = [
      { name: '证据1', pageStart: 1, pageEnd: 4 },
      { name: '证据2', pageStart: 5, pageEnd: 10 },
    ]
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: {
      outputs: [{ name: '证据2', pageStart: 5, pageEnd: 10, outputPath: '/out/2.pdf' }],
      failed: ['证据1：磁盘写入失败'],
    } })
    await api.executeMergedImportPlan()
    expect(plan.splitStatus).toBe('partial')
    state.splitNamePrefix.value = '不应改变重试请求'
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: {
      outputs: [{ name: '证据1', pageStart: 1, pageEnd: 4, outputPath: '/out/1.pdf' }],
    } })
    await api.executeMergedImportPlan()
    expect(tauriCallSafe.mock.lastCall[1].args.items).toEqual([{ name: '证据1', pageStart: 1, pageEnd: 4, source: 'unknown' }])
    expect(plan.outputs.map(output => output.pageStart)).toEqual([1, 5])
    expect(state.overlayFiles.value).toHaveLength(2)
    expect(plan.splitStatus).toBe('complete')
  })

  it('阻止不同源文件共用输出目录', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    for (const plan of state.mergedImportPlans.value) plan.outputDir = '/out'
    tauriCallSafe.mockClear()
    await api.executeMergedImportPlan()
    expect(tauriCallSafe).not.toHaveBeenCalled()
    expect(state.mergedImportPlan.value.splitError).toContain('独立的输出目录')
  })

  it('停止队列后允许当前文件结束，但不启动下一份', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    let finish
    tauriCallSafe.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    tauriCallSafe.mockClear()
    const run = api.detectAllMergedImports()
    api.requestStopMergedBatch()
    finish({ ok: false, error: '用户取消' })
    await run
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
    expect(state.mergedImportPlans.value[1].detectionAttempted).toBeUndefined()
    expect(api.batchRunning.value).toBe(false)
  })

  it('页数读取失败不伪造页数，保留其他文件并允许单独重试', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValueOnce({ ok: false, error: '暂时无法读取' })
      .mockResolvedValueOnce({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    const [first, second] = state.mergedImportPlans.value
    expect(first.totalPages).toBe(0)
    expect(first.items).toEqual([])
    expect(second.totalPages).toBe(10)
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: 20 }).mockResolvedValueOnce({
      ok: true, data: { totalPages: 20, pagesAnalyzed: 20, items: [{ name: '证据1', pageStart: 1, pageEnd: 20 }] },
    })
    await api.detectMergedImportPlan()
    expect(first.totalPages).toBe(20)
    expect(first.detectionError).toBe('')
    expect(second.totalPages).toBe(10)
  })

  it('移除分组仅移出当前列表，不调用文件删除或影响其他方案', async () => {
    const { state, api } = setup()
    tauriCallSafe.mockResolvedValue({ ok: true, data: 10 })
    await api.importMergedPdfAsEvidence(['/a/report.pdf', '/b/report.pdf'])
    const [first, second] = state.mergedImportPlans.value
    state.overlayFiles.value = [{ path: '/out/a.pdf', sourceInputPath: first.inputPath }, { path: '/out/b.pdf', sourceInputPath: second.inputPath }]
    tauriCallSafe.mockClear()
    api.removeMergedImportPlan(first)
    expect(state.mergedImportPlans.value).toEqual([second])
    expect(state.mergedImportPlan.value).toBe(second)
    expect(state.overlayFiles.value.map(file => file.path)).toEqual(['/out/b.pdf'])
    expect(tauriCallSafe).not.toHaveBeenCalled()
  })
})
