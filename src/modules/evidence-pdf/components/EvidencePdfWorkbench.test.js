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
import EvidencePdfWorkbench from './EvidencePdfWorkbench.vue'
import { createEvidenceFile } from '../../../shared/pdf-tools/composables/useEvidencePdfSession.js'

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
})
