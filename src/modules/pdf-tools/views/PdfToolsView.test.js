import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('vue', async importOriginal => ({
  ...(await importOriginal()), onMounted: vi.fn(), onBeforeUnmount: vi.fn(),
  useSSRContext: () => ({ modules: new Set() }),
}))
vi.mock('../../../core/composables/useWindowFileDrop.js', () => ({ useWindowFileDrop: vi.fn() }))
vi.mock('../../../core/composables/useWorkspacePreferences.js', () => ({ useWorkspacePreferences: () => ({ start: vi.fn(), stop: vi.fn() }) }))
vi.mock('../../../core/tauriBridge.js', () => ({
  tauriCallSafe: vi.fn(), getPdfPageCount: vi.fn(), openPath: vi.fn(), userFacingError: error => String(error),
}))
vi.mock('element-plus', () => ({ ElMessage: { error: vi.fn(), warning: vi.fn(), success: vi.fn() }, ElNotification: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }))
import { open } from '@tauri-apps/plugin-dialog'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import PdfToolsView from './PdfToolsView.vue'

describe('PDF 防复制界面链路', () => {
  beforeEach(() => vi.clearAllMocks())

  it('按后端字体字段显示检测结果，检测中的文件不能执行', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/a.pdf', name: 'a.pdf', hasAntiOcr: null }]
    expect(state.antiOcrReadyCount.value).toBe(0)
    tauriCallSafe.mockResolvedValue({ ok: true, data: { has_protection: true, has_backup: true, total_fonts: 4, protected_fonts: 2 } })
    await state.inspectAntiOcrFiles(['/a.pdf'])
    expect(state.antiOcrProtectedCount.value).toBe(1)
    expect(state.antiOcrFiles.value[0].statusText).toContain('2/4 个字体')
  })

  it('移除操作使用刚生成的副本，不再错误地处理原件', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/a.pdf', name: 'a.pdf', hasAntiOcr: false }]
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/a_anti_copy-2.pdf', detection: { has_protection: true, has_backup: true } } })
    await state.batchAntiOcrApply()
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/a_restored.pdf', detection: { has_protection: false, has_backup: false } } })
    await state.batchAntiOcrRemove()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('remove_anti_copy', { input: '/a_anti_copy-2.pdf', output: '/a_restored.pdf' })
    expect(state.antiOcrFiles.value[0].path).toBe('/a.pdf')
    expect(state.antiOcrFiles.value[0].outputPath).toBe('/a_restored.pdf')
    expect(state.antiOcrProtectedCount.value).toBe(0)
  })

  it('不能恢复没有原始映射备份的外部防复制文件', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.antiOcrFiles.value = [{ path: '/external.pdf', hasAntiOcr: true, hasBackup: false }]
    await state.batchAntiOcrRemove()
    expect(tauriCallSafe).not.toHaveBeenCalled()
  })
})

describe('PDF 队列的统一执行方式', () => {
  beforeEach(() => vi.clearAllMocks())

  it('添加加密 PDF 仅检测，确认执行后才解锁，并保留结果入口', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    tauriCallSafe.mockResolvedValue({ ok: true, data: { encrypted: true } })
    state.addUnlockFiles(['/a.pdf'])
    for (let i = 0; i < 12; i++) await Promise.resolve()
    expect(tauriCallSafe.mock.calls.map(([cmd]) => cmd)).toEqual(['inspect_pdf'])
    expect(state.unlockReadyCount.value).toBe(1)
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_unlocked.pdf', skipped: false } })
    await state.batchUnlock()
    expect(state.unlockFiles.value[0].outputPath).toBe('/a_unlocked.pdf')
    expect(tauriCallSafe).toHaveBeenLastCalledWith('unlock_pdf', { input: '/a.pdf' })
  })

  it('压缩仅在点击执行时运行，完成后更改设置不会自动重跑', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.loadCompressFiles(['/a.pdf'])
    expect(tauriCallSafe).not.toHaveBeenCalled()
    state.compressImageReencode.value = true
    state.compressLevel.value = 2
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_small.pdf', input_size: 100, output_size: 70 } })
    await state.runCompressQueue()
    expect(tauriCallSafe).toHaveBeenCalledWith('compress_pdf', { input: '/a.pdf', outputDir: null, level: 2 })
    state.compressLevel.value = 3
    await Promise.resolve()
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
    expect(state.compressFiles.value[0].status).toBe('pending')
    await state.runCompressQueue()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('compress_pdf', { input: '/a.pdf', outputDir: null, level: 3 })
  })

  it('处理中禁止清空、移除或追加；失败后仍可手动重试', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.loadCompressFiles(['/a.pdf', '/b.pdf'])
    let resolve
    tauriCallSafe.mockImplementationOnce(() => new Promise(r => { resolve = r }))
      .mockResolvedValue({ ok: true, data: { output_path: '/b_small.pdf' } })
    const running = state.runCompressQueue()
    state.clearCompressFiles()
    state.removeCompressFile(1)
    state.loadCompressFiles(['/c.pdf'])
    expect(state.compressFiles.value.map(f => f.path)).toEqual(['/a.pdf', '/b.pdf'])
    resolve({ ok: false, error: '读取失败' })
    await running
    expect(state.compressFiles.value[0].status).toBe('failed')
    tauriCallSafe.mockResolvedValue({ ok: true, data: { output_path: '/a_small.pdf' } })
    await state.runCompressQueue()
    expect(state.compressFiles.value.map(f => f.status)).toEqual(['done', 'done'])
    expect(tauriCallSafe.mock.calls.filter(([cmd]) => cmd === 'compress_pdf')).toHaveLength(3)
  })
})

describe('PDF 合并支持 DOCX 与 DOC 文档', () => {
  beforeEach(() => vi.clearAllMocks())

  it('支持添加并识别 docx、doc 和 pdf 文件', () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/docs/a.docx', '/docs/b.doc', '/docs/c.pdf'])
    expect(state.mergeFiles.value).toHaveLength(3)
    expect(state.mergeFiles.value.map(f => f.path)).toEqual(['/docs/a.docx', '/docs/b.doc', '/docs/c.pdf'])
    expect(state.isWordPath('/docs/a.docx')).toBe(true)
    expect(state.isWordPath('/docs/b.doc')).toBe(true)
    expect(state.isWordPath('/docs/c.pdf')).toBe(false)
  })

  it('单项 Word 文件在合并列表中可直接转为 PDF', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/folder/contract.docx'])
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/folder/contract.pdf' } })

    await state.doMerge()

    expect(tauriCallSafe).toHaveBeenCalledWith('convert_word_to_pdf', {
      input: '/folder/contract.docx',
    })
    expect(state.mergeOutputPath.value).toBe('/folder/contract.pdf')
    expect(state.mergeFiles.value[0].outputPath).toBe('/folder/contract.pdf')
    expect(state.mergeFiles.value[0].statusText).toBe('PDF 已就绪')
  })

  it('列表中的 Word 项可通过 convertSingleMergeFile 独立转为 PDF', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/folder/note.doc'])
    tauriCallSafe.mockResolvedValueOnce({ ok: true, data: { output_path: '/folder/note.pdf' } })

    await state.convertSingleMergeFile(state.mergeFiles.value[0])

    expect(tauriCallSafe).toHaveBeenCalledWith('convert_word_to_pdf', {
      input: '/folder/note.doc',
    })
    expect(state.mergeFiles.value[0].outputPath).toBe('/folder/note.pdf')
  })

  it('多项 Word 转为 PDF 后按原顺序合并', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/docs/cover.docx', '/docs/page.pdf', '/docs/appendix.doc'])
    open.mockResolvedValueOnce('/out')
    tauriCallSafe.mockImplementation(async (command, args) => ({ ok: true, data: { output_path: command === 'merge_pdfs' ? '/out/merged.pdf' : args.input.replace(/\.(docx|doc)$/, '.pdf') } }))
    await state.doMerge()
    expect(tauriCallSafe).toHaveBeenCalledWith('merge_pdfs', {
      inputs: ['/docs/cover.pdf', '/docs/page.pdf', '/docs/appendix.pdf'], output: '/out/merged.pdf', duplexSeparate: false,
    })
    expect(state.mergeOutputPath.value).toBe('/out/merged.pdf')
  })

  it('转换失败后保留列表，手动 PDF 替换原项后继续合并且不重复转换 Word', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/docs/fail.docx', '/docs/page.pdf'])
    open.mockResolvedValueOnce('/out')
    tauriCallSafe.mockResolvedValue({ ok: false, error: 'Word / WPS 转换失败' })
    await state.doMerge()
    expect(state.mergeFiles.value.map(item => item.path)).toEqual(['/docs/fail.docx', '/docs/page.pdf'])
    expect(state.mergeFiles.value[0].conversionError).toContain('另存为 PDF')
    expect(tauriCallSafe.mock.calls.some(([command]) => command === 'merge_pdfs')).toBe(false)
    open.mockResolvedValueOnce('/manual/fail.pdf')
    await state.selectManualMergePdf(state.mergeFiles.value[0])
    tauriCallSafe.mockResolvedValue({ ok: true, data: '/out/merged.pdf' })
    await state.doMerge()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('merge_pdfs', { inputs: ['/manual/fail.pdf', '/docs/page.pdf'], output: '/out/merged.pdf', duplexSeparate: false })
    expect(tauriCallSafe.mock.calls.filter(([command]) => command === 'convert_word_to_pdf')).toHaveLength(1)
    expect(state.mergeOutputDir.value).toBe('/out')
    expect(open).toHaveBeenCalledTimes(2)
  })

  it('图片先进入预览，确认后的 PDF 保持混排位置，取消时不添加图片', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    const review = vi.fn().mockResolvedValueOnce('/docs/images_A4.pdf').mockResolvedValueOnce(null)
    state.imageImportDialog.value = { open: review }
    await state.addMergeFiles(['/docs/before.pdf', '/docs/a.png', '/docs/b.jpg', '/docs/after.docx'])
    expect(review).toHaveBeenCalledWith(['/docs/a.png', '/docs/b.jpg'])
    expect(state.mergeFiles.value.map(item => item.path)).toEqual(['/docs/before.pdf', '/docs/images_A4.pdf', '/docs/after.docx'])
    await state.addMergeFiles(['/docs/c.png'])
    expect(state.mergeFiles.value).toHaveLength(3)
  })

  it('拖入文件在 merge 标签页下允许 docx 与 doc', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.activeTab.value = 'merge'

    await state.handleDroppedPdfPaths(['/test/a.docx', '/test/b.doc', '/test/c.txt', '/test/d.pdf'])

    expect(state.mergeFiles.value.map(f => f.path)).toEqual([
      '/test/a.docx',
      '/test/b.doc',
      '/test/d.pdf',
    ])
  })
})
