import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { useEvidencePdfMergedImport } from './useEvidencePdfMergedImport.js'
import { tauriCallSafe } from '../../../core/tauriBridge.js'

vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn(), userFacingError: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn() }, ElMessageBox: {} }))

function setup() {
  const state = {
    overlayFiles: ref([]), overlayOutputDir: ref(''), importingMergedPdf: ref(false),
    detectingMergedImport: ref(false), splittingMergedImport: ref(false), mergedImportPlan: ref(null),
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

  it('多文件不再绕过拆分，并保留当前页段', async () => {
    const { state, api } = setup()
    state.mergedImportPlan.value = { inputPath: '/docs/current.pdf' }
    await api.importMergedPdfAsEvidence(['/docs/one.pdf', '/docs/two.pdf'])
    expect(tauriCallSafe).not.toHaveBeenCalled()
    expect(state.mergedImportPlan.value.inputPath).toBe('/docs/current.pdf')
    expect(state.overlayFiles.value).toEqual([])
  })

  it('检测失败保留可重试的页段，不进入批量处理', async () => {
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: 10 }).mockResolvedValueOnce({ ok: false, error: '缺少 pdftotext' })
    const { state, api } = setup()
    await api.importMergedPdfAsEvidence(['/docs/merged.pdf'])
    await api.detectMergedImportPlan()
    expect(state.mergedImportPlan.value.warnings).toContain('缺少 pdftotext')
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
    expect(tauriCallSafe.mock.calls[0][1].args.cleanupTargets).toMatchObject([{ region: 'footer', normalizedText: '第 {page} 页', pageStart: 1, pageEnd: 10 }])
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
})
