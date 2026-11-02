import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfMergedImport } from './useEvidencePdfMergedImport.js'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(), userFacingError: vi.fn(error => String(error)),
  showLoading: vi.fn(), hideLoading: vi.fn(), emitOperationUpdate: vi.fn(),
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('element-plus', () => ({
  ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn(), info: vi.fn() },
  ElMessageBox: { confirm: vi.fn().mockResolvedValue('confirm') },
}))

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
  beforeEach(() => {
    vi.clearAllMocks()
    // 默认：无电子签章，不弹确认框
    tauriCallSafe.mockImplementation(async (command) => {
      if (command === 'detect_pdf_signatures') {
        return { ok: true, data: { hasSignatures: false, count: 0, summary: '未检测到电子签名' } }
      }
      return { ok: false, error: `unexpected command ${command}` }
    })
  })

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
    tauriCallSafe.mockImplementation(async (command) => {
      if (command === 'detect_pdf_signatures') {
        return { ok: true, data: { hasSignatures: false, count: 0, summary: '未检测到电子签名' } }
      }
      if (command === 'split_merged_evidence_pdf') {
        return { ok: true, data: { outputs: [{ name: '证据1', pageStart: 1, pageEnd: 10, removedBlankPages: 2, outputPath: '/out/1.pdf' }], failed: [], warnings: [] } }
      }
      return { ok: false, error: `unexpected ${command}` }
    })
    await api.executeMergedImportPlan()
    expect(state.overlayFiles.value[0].pages).toBe(8)
    expect(tauriCallSafe.mock.calls.find(([cmd]) => cmd === 'split_merged_evidence_pdf')[1].args.cleanupTargets).toMatchObject([
      { region: 'footer', normalizedText: '第 {page} 页', pageStart: 1, pageEnd: 10 },
    ])
    expect(state.mergedImportPlan.value).toBeNull()
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
    tauriCallSafe.mockImplementation(async (command, { args }) => {
      if (command === 'detect_pdf_signatures') {
        return { ok: true, data: { hasSignatures: false, count: 0, summary: '未检测到电子签名' } }
      }
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
    tauriCallSafe.mockImplementation(async (command) => {
      if (command === 'detect_pdf_signatures') {
        return { ok: true, data: { hasSignatures: false, count: 0, summary: '未检测到电子签名' } }
      }
      if (command === 'split_merged_evidence_pdf') {
        return { ok: true, data: {
          outputs: [{ name: '证据2', pageStart: 5, pageEnd: 10, outputPath: '/out/2.pdf' }],
          failed: ['证据1：磁盘写入失败'],
        } }
      }
      return { ok: false, error: `unexpected ${command}` }
    })
    await api.executeMergedImportPlan()
    expect(plan.splitStatus).toBe('partial')
    state.splitNamePrefix.value = '不应改变重试请求'
    tauriCallSafe.mockImplementation(async (command, { args }) => {
      if (command === 'detect_pdf_signatures') {
        return { ok: true, data: { hasSignatures: false, count: 0, summary: '未检测到电子签名' } }
      }
      expect(args.items).toEqual([{ name: '证据1', pageStart: 1, pageEnd: 4, source: 'unknown' }])
      return { ok: true, data: {
        outputs: [{ name: '证据1', pageStart: 1, pageEnd: 4, outputPath: '/out/1.pdf' }],
      } }
    })
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


describe('manual split navigation and editing', () => {
  function manual() {
    const context = setup()
    context.state.previewMaxPage.value = 20
    context.state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 20,
      items: [{ name: '封面', pageStart: 1, pageEnd: 4 }, { name: '正文', pageStart: 5, pageEnd: 20 }] }
    return context
  }
  it('focuses the requested endpoint without resetting the render key or calling the backend', () => {
    const { state, api } = manual()
    const row = state.mergedImportPlan.value.items[1]
    api.selectMergedImportRange(row, 'end')
    expect(state.previewPage.value).toBe(20)
    expect(state.selectedMergedImportIndex.value).toBe(1)
    api.selectMergedImportRange(row, 'end')
    expect(state.previewPage.value).toBe(20)
    api.selectMergedImportRange(row)
    expect(state.previewPage.value).toBe(5)
    expect(state.refreshPreview).not.toHaveBeenCalled()
    expect(tauriCallSafe).not.toHaveBeenCalled()
  })
  it('selecting and renaming a segment keeps the current page and completed preview', () => {
    const { state, api } = manual()
    const row = state.mergedImportPlan.value.items[1]
    state.previewPage.value = 17
    state.truePreview.value = { path: '/cached-page.png' }
    const rendered = state.truePreview.value
    api.selectMergedImportRange(row, null)
    row.name = '修正后的文件名'
    api.selectMergedImportRange(row, null)
    expect(state.selectedMergedImportIndex.value).toBe(1)
    expect(state.previewPage.value).toBe(17)
    expect(state.truePreview.value).toBe(rendered)
    expect(state.refreshPreview).not.toHaveBeenCalled()
    expect(tauriCallSafe).not.toHaveBeenCalled()
    // Re-selecting an endpoint already on screen must also keep the render.
    state.previewPage.value = 20
    api.selectMergedImportRange(row, 'end')
    expect(state.truePreview.value).toBe(rendered)
  })
  it('keeps rows fixed during editing, reports the unassigned tail and fills it only explicitly', () => {
    const { state, api } = manual()
    api.onMergedRangeEndChanged(1, 12)
    expect(state.previewPage.value).toBe(12)
    expect(state.mergedImportPlan.value.items.map(r => [r.pageStart, r.pageEnd])).toEqual([[1, 4], [5, 12]])
    expect(api.mergedImportGaps.value).toEqual([{ pageStart: 13, pageEnd: 20 }])
    api.fillMergedImportGaps()
    expect(state.mergedImportPlan.value.items).toHaveLength(3)
    api.undoMergedImportEdit()
    expect(state.mergedImportPlan.value.items).toHaveLength(2)
    expect(api.canUndoMergedImport.value).toBe(true)
    api.undoMergedImportEdit()
    expect(state.mergedImportPlan.value.items.map(r => [r.pageStart, r.pageEnd])).toEqual([[1, 4], [5, 20]])
    api.redoMergedImportEdit()
    expect(state.previewPage.value).toBe(12)
    expect(state.safeRefreshPreview).not.toHaveBeenCalled()
  })
  it('splits before or after the viewed page, retaining that page and using idempotent boundaries', () => {
    const { state, api } = manual()
    state.previewPage.value = 10
    api.splitMergedImportFromCurrentPage()
    expect(state.mergedImportPlan.value.items.map(r => [r.pageStart, r.pageEnd])).toEqual([[1, 4], [5, 9], [10, 20]])
    expect(state.previewPage.value).toBe(10)
    expect(state.selectedMergedImportIndex.value).toBe(2)
    api.splitMergedImportFromCurrentPage()
    expect(state.mergedImportPlan.value.undoStack).toHaveLength(1)
    api.undoMergedImportEdit()
    state.previewPage.value = 10
    api.splitMergedImportAfterCurrentPage()
    expect(state.mergedImportPlan.value.items.map(r => [r.pageStart, r.pageEnd])).toEqual([[1, 4], [5, 10], [11, 20]])
    expect(state.previewPage.value).toBe(10)
    expect(state.selectedMergedImportIndex.value).toBe(1)
    expect(state.refreshPreview).not.toHaveBeenCalled()
  })
  it('guards the document edges and stops edits while a batch or split is running', () => {
    const { state, api } = manual()
    state.previewPage.value = 1
    expect(api.splitMergedImportFromCurrentPage()).toBe(false)
    state.previewPage.value = 20
    expect(api.splitMergedImportAfterCurrentPage()).toBe(false)
    api.batchRunning.value = true
    state.previewPage.value = 10
    expect(api.splitMergedImportAfterCurrentPage()).toBe(false)
    api.onMergedRangeEndChanged(1, 12)
    expect(state.mergedImportPlan.value.items[1].pageEnd).toBe(20)
    expect(state.mergedImportPlan.value.items).toHaveLength(2)
  })
  it('applies a whole batch as one reversible edit and preserves other segments and the viewed page', () => {
    const { state, api } = manual()
    state.previewPage.value = 14
    const payload = { inputPath: '/scan.pdf', pageStart: 5, pageEnd: 20, items: [
      { name: '甲', pageStart: 5, pageEnd: 10 }, { name: '乙', pageStart: 11, pageEnd: 15 }, { name: '丙', pageStart: 16, pageEnd: 20 },
    ] }
    expect(api.applyMergedRangeBatch(payload)).toBe(true)
    expect(state.mergedImportPlan.value.items.map(r => r.name)).toEqual(['封面', '甲', '乙', '丙'])
    expect(state.selectedMergedImportIndex.value).toBe(2)
    expect(state.previewPage.value).toBe(14)
    expect(state.mergedImportPlan.value.undoStack).toHaveLength(1)
    expect(api.applyMergedRangeBatch(payload)).toBe(false)
    api.undoMergedImportEdit()
    expect(state.mergedImportPlan.value.items.map(r => r.name)).toEqual(['封面', '正文'])
    expect(state.previewPage.value).toBe(14)
    expect(state.refreshPreview).not.toHaveBeenCalled()
  })
})


it('retains the sole interval instead of leaving manual splitting without an editable range', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 20,
    items: [{ name: '全文', pageStart: 1, pageEnd: 20 }] }
  api.removeMergedImportRange(0)
  expect(state.mergedImportPlan.value.items).toEqual([{ name: '全文', pageStart: 1, pageEnd: 20 }])
  expect(api.canUndoMergedImport.value).toBe(false)
})


it('keeps keyed rows and object references through reordering, boundary edits and undo', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [
    { name: '证据1', pageStart: 1, pageEnd: 10 }, { name: '证据2', pageStart: 11, pageEnd: 20 }, { name: '证据3', pageStart: 21, pageEnd: 30 },
  ] }
  const rows = state.mergedImportPlan.value.items, target = rows[1], keys = rows.map(api.splitRangeRowKey)
  rows.reverse()
  api.onMergedRangeEndChanged(target, 30)
  expect(rows.map(row => row.name)).toEqual(['证据3', '证据2', '证据1'])
  expect(rows.map(api.splitRangeRowKey)).toEqual([...keys].reverse())
  expect(rows[0].pageStart).toBe(21)
  expect(rows[2].pageEnd).toBe(10)
  expect(api.mergedImportWarnings.value.some(message => message.includes('重叠'))).toBe(true)
  api.undoMergedImportEdit()
  expect(rows[1]).toBe(target)
  expect(rows[1].pageEnd).toBe(20)
})

it('uses the same idempotent add operation at an existing boundary and preserves current-page preview', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [
    { name: '证据7', pageStart: 1, pageEnd: 20 }, { name: '证据9', pageStart: 21, pageEnd: 30 },
  ] }
  state.previewPage.value = 10
  api.addMergedImportRangeFromPage()
  expect(state.mergedImportPlan.value.items.map(row => [row.pageStart, row.pageEnd])).toEqual([[1, 9], [10, 20], [21, 30]])
  const original = state.mergedImportPlan.value.items[0], key = api.splitRangeRowKey(original)
  api.addMergedImportRangeFromPage()
  expect(state.mergedImportPlan.value.items).toHaveLength(3)
  expect(api.splitRangeRowKey(original)).toBe(key)
  expect(state.previewPage.value).toBe(10)
})


it('merges the selected physical neighbour after reordering and can undo reorder and merge separately', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [
    { name: '甲', pageStart: 1, pageEnd: 10 }, { name: '乙', pageStart: 11, pageEnd: 20 }, { name: '丙', pageStart: 21, pageEnd: 30 },
  ] }
  const rows = state.mergedImportPlan.value.items, a = rows[0], b = rows[1], c = rows[2]
  api.selectMergedImportRange(c, 'end')
  api.reorderMergedImportRanges(2, 0)
  expect(state.previewPage.value).toBe(30)
  expect(api.selectedMergedImportRange.value).toBe(c)
  api.mergeCurrentMergedImportRangeIntoPrevious()
  expect(rows.map(row => row.name)).toEqual(['甲', '乙'])
  expect(b.pageEnd).toBe(30)
  expect(a.pageEnd).toBe(10)
  api.undoMergedImportEdit()
  expect(rows.map(row => row.name)).toEqual(['丙', '甲', '乙'])
  expect(b.pageEnd).toBe(20)
  api.undoMergedImportEdit()
  expect(rows.map(row => row.name)).toEqual(['甲', '乙', '丙'])
  expect(state.previewPage.value).toBe(30)
})

it('inserts directly after the selected row using the surrounding gap, independently of the preview page', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [
    { name: '甲', pageStart: 1, pageEnd: 9 }, { name: '乙', pageStart: 20, pageEnd: 25 }, { name: '丙', pageStart: 26, pageEnd: 30 },
  ] }
  state.selectedMergedImportIndex.value = 0
  state.previewPage.value = 28
  const original = [...state.mergedImportPlan.value.items]
  expect(api.addMergedImportRange()).toBe(true)
  const rows = state.mergedImportPlan.value.items
  expect(rows.map(row => [row.pageStart, row.pageEnd])).toEqual([[1, 9], [10, 19], [20, 25], [26, 30]])
  expect(state.selectedMergedImportIndex.value).toBe(1)
  expect(state.previewPage.value).toBe(28)
  expect(rows[0]).toBe(original[0])
  expect(rows[2]).toBe(original[1])
  expect(rows[3]).toBe(original[2])
  api.undoMergedImportEdit()
  expect(rows.map(row => row.name)).toEqual(['甲', '乙', '丙'])
  expect(state.previewPage.value).toBe(28)
})

it('inserts an explicit draft when adjacent rows leave no free pages, without changing them or jumping the preview', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [
    { name: '甲', pageStart: 1, pageEnd: 9 }, { name: '乙', pageStart: 10, pageEnd: 30 },
  ] }
  state.previewPage.value = 25
  expect(api.insertMergedImportRangeAfter(0)).toBe(true)
  const rows = state.mergedImportPlan.value.items
  expect(rows.map(row => [row.pageStart, row.pageEnd])).toEqual([[1, 9], [null, 9], [10, 30]])
  api.selectMergedImportRange(rows[1], 'start')
  expect(state.previewPage.value).toBe(25)
  expect(api.mergedImportWarnings.value.some(message => message.includes('缺少名称或起止页'))).toBe(true)
  api.removeMergedImportRange(1)
  expect(rows.map(row => [row.pageStart, row.pageEnd])).toEqual([[1, 9], [10, 30]])
})

it('takes the document end for a newly inserted final row', () => {
  const { state, api } = setup()
  state.mergedImportPlan.value = { inputPath: '/scan.pdf', totalPages: 30, items: [{ name: '甲', pageStart: 1, pageEnd: 20 }] }
  api.insertMergedImportRangeAfter(0)
  expect(state.mergedImportPlan.value.items.map(row => [row.pageStart, row.pageEnd])).toEqual([[1, 20], [21, 30]])
})
