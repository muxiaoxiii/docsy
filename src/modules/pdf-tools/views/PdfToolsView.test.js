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
import { getPdfPageCount, tauriCallSafe } from '../../../core/tauriBridge.js'
import PdfToolsView from './PdfToolsView.vue'

describe('接收新文件保留已有工作', () => {
  beforeEach(() => vi.clearAllMocks())
  it('clearing the split list invalidates a pending file read and resets its results', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    let finish
    getPdfPageCount.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
    const pending = state.loadSplitFile('/slow.pdf')
    state.splitRanges.value = [{ name: 'old', pageStart: 1, pageEnd: 5 }]
    state.splitResultPaths.value = ['/old-output.pdf']
    state.clearSplitFile()
    finish({ ok: true, data: 10 }); await pending
    expect(state.splitFile.value).toBe('')
    expect(state.splitRanges.value).toEqual([])
    expect(state.splitResultPaths.value).toEqual([])
  })
  it('读取失败时保留当前拆分页段和提取设置', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    getPdfPageCount.mockResolvedValue({ ok: true, data: 12 })
    await state.loadSplitFile('/old.pdf')
    state.splitRanges.value = [{ name: '已编辑', pageStart: 2, pageEnd: 9 }]
    await state.loadExtractFile('/old.pdf')
    state.extractPageText.value = '2-5'
    getPdfPageCount.mockResolvedValue({ ok: false, error: '文件不存在' })
    expect(await state.loadSplitFile('/missing.pdf')).toBe(false)
    expect(await state.loadExtractFile('/missing.pdf')).toBe(false)
    expect(state.splitFile.value).toBe('/old.pdf')
    expect(state.splitRanges.value).toEqual([{ name: '已编辑', pageStart: 2, pageEnd: 9 }])
    expect(state.extractFile.value).toBe('/old.pdf')
    expect(state.extractPageText.value).toBe('2-5')
  })
  it('慢请求不能覆盖后来选择的文件', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    let finish
    getPdfPageCount.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
      .mockResolvedValueOnce({ ok: true, data: 8 })
    const first = state.loadExtractFile('/slow.pdf')
    await state.loadExtractFile('/new.pdf')
    finish({ ok: true, data: 10 }); await first
    expect(state.extractFile.value).toBe('/new.pdf')
    expect(state.extractTotalPages.value).toBe(8)
  })
})

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
    await state.selectMergeOutputDir()
    tauriCallSafe.mockImplementation(async (command, args) => ({ ok: true, data: { output_path: command === 'merge_pdfs' ? '/out/merged.pdf' : args.input.replace(/\.(docx|doc)$/, '.pdf') } }))
    await state.doMerge()
    expect(tauriCallSafe).toHaveBeenCalledWith('merge_pdfs', {
      inputs: ['/docs/cover.pdf', '/docs/page.pdf', '/docs/appendix.pdf'], output: '/out/合并文件.pdf', duplexSeparate: false,
    })
    expect(state.mergeOutputPath.value).toBe('/out/merged.pdf')
  })

  it('转换失败后保留列表，手动 PDF 替换原项后继续合并且不重复转换 Word', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.addMergeFiles(['/docs/fail.docx', '/docs/page.pdf'])
    open.mockResolvedValueOnce('/out')
    await state.selectMergeOutputDir()
    tauriCallSafe.mockResolvedValue({ ok: false, error: 'Word / WPS 转换失败' })
    await state.doMerge()
    expect(state.mergeFiles.value.map(item => item.path)).toEqual(['/docs/fail.docx', '/docs/page.pdf'])
    expect(state.mergeFiles.value[0].conversionError).toContain('另存为 PDF')
    expect(tauriCallSafe.mock.calls.some(([command]) => command === 'merge_pdfs')).toBe(false)
    open.mockResolvedValueOnce('/manual/fail.pdf')
    await state.selectManualMergePdf(state.mergeFiles.value[0])
    tauriCallSafe.mockResolvedValue({ ok: true, data: '/out/merged.pdf' })
    await state.doMerge()
    expect(tauriCallSafe).toHaveBeenLastCalledWith('merge_pdfs', { inputs: ['/manual/fail.pdf', '/docs/page.pdf'], output: '/out/合并文件.pdf', duplexSeparate: false })
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

it('uses the common input folder and a custom filename without asking for a directory', async () => {
  vi.clearAllMocks()
  const state = PdfToolsView.setup({}, { expose: vi.fn() })
  await state.addMergeFiles(['/case/a.pdf','/case/b.pdf'])
  expect(state.mergeOutputDir.value).toBe('/case')
  state.mergeFileName.value = '提交材料'
  tauriCallSafe.mockResolvedValue({ok:true,data:'/case/提交材料-2.pdf'})
  await state.doMerge()
  expect(open).not.toHaveBeenCalled()
  expect(tauriCallSafe).toHaveBeenLastCalledWith('merge_pdfs',{inputs:['/case/a.pdf','/case/b.pdf'],output:'/case/提交材料.pdf',duplexSeparate:false})
  expect(state.mergeOutputPath.value).toBe('/case/提交材料-2.pdf')
})

it('recalculates automatic directories but preserves an explicit choice while adding and removing files', async () => {
  vi.clearAllMocks()
  const state = PdfToolsView.setup({}, { expose: vi.fn() })
  await state.addMergeFiles(['/case/a.pdf','/case/b.pdf'])
  await state.addMergeFiles(['/other/c.pdf'])
  expect(state.mergeOutputDir.value).toBe('')
  state.removeMergeFile(2)
  expect(state.mergeOutputDir.value).toBe('/case')
  open.mockResolvedValueOnce('/chosen')
  await state.selectMergeOutputDir()
  await state.addMergeFiles(['/other/c.pdf'])
  state.removeMergeFile(2)
  expect(state.mergeOutputDir.value).toBe('/chosen')
  state.clearMergeFiles()
  await state.addMergeFiles(['/new/a.pdf','/new/b.pdf'])
  expect(state.mergeOutputDir.value).toBe('/new')
})

it('retains the folder selected at execution for mixed-source files and rejects path-like names before export', async () => {
  vi.clearAllMocks()
  const state = PdfToolsView.setup({}, { expose: vi.fn() })
  await state.addMergeFiles(['/one/a.pdf', '/two/b.pdf'])
  state.mergeFileName.value = '../a.pdf'
  await state.doMerge()
  expect(open).not.toHaveBeenCalled()
  expect(tauriCallSafe).not.toHaveBeenCalled()
  state.mergeFileName.value = '汇总.pdf'
  open.mockResolvedValueOnce('/chosen')
  tauriCallSafe.mockResolvedValue({ ok: true, data: '/chosen/汇总.pdf' })
  await state.doMerge()
  state.removeMergeFile(1)
  await state.addMergeFiles(['/one/c.pdf'])
  expect(state.mergeOutputDir.value).toBe('/chosen')
})

describe('长图处理界面链路', () => {
  beforeEach(() => vi.clearAllMocks())

  it('添加长图文件后生成预览并更新尺寸与预计页数', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    tauriCallSafe.mockResolvedValue({
      ok: true,
      data: {
        sourceWidth: 1080,
        sourceHeight: 7500,
        totalPages: 4,
        previewPage: 1,
        previewDataUrl: 'data:image/png;base64,mock',
      },
    })
    state.addLongSliceFiles(['/photos/long_screen.png'])
    expect(state.longSliceFiles.value.length).toBe(1)
    await state.refreshPreview()
    expect(tauriCallSafe).toHaveBeenCalledWith('preview_long_slice', expect.objectContaining({
      args: expect.objectContaining({
        inputPath: '/photos/long_screen.png',
        orientation: 'portrait',
        marginMm: 5,
        contentWidthMode: 'fit',
      }),
      previewPage: 1,
    }))
    expect(state.longSliceFiles.value[0].dimText).toBe('1080 × 7500')
    expect(state.longSliceFiles.value[0].statusText).toBe('预计 4 页')
  })

  it('批量执行长图切片并输出对应 A4 文件路径', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.longSliceFiles.value = [
      { path: '/screen.png', name: 'screen.png', statusText: '待处理', statusType: 'info' },
    ]
    tauriCallSafe.mockResolvedValueOnce({
      ok: true,
      data: {
        outputPath: '/screen_A4.pdf',
        totalPages: 4,
        sourceWidth: 1080,
        sourceHeight: 7500,
      },
    })
    await state.batchProcessLongSlice()
    expect(tauriCallSafe).toHaveBeenCalledWith('process_long_slice', expect.objectContaining({
      args: expect.objectContaining({
        inputPath: '/screen.png',
      }),
    }))
    expect(state.longSliceFiles.value[0].outputPath).toBe('/screen_A4.pdf')
    expect(state.longSliceFiles.value[0].statusText).toBe('已生成 4 页')
    expect(state.longSliceFiles.value[0].statusType).toBe('success')
  })

  it('快速切换文件时忽略过期的预览响应', async () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    let resolveFirst
    let resolveSecond
    tauriCallSafe
      .mockImplementationOnce(() => new Promise(resolve => { resolveFirst = resolve }))
      .mockImplementationOnce(() => new Promise(resolve => { resolveSecond = resolve }))
    state.addLongSliceFiles(['/first.png', '/second.png'])
    state.onSelectLongSliceItem(1)

    resolveSecond({
      ok: true,
      data: {
        sourceWidth: 200,
        sourceHeight: 600,
        sourcePageIndex: 2,
        sourcePageCount: 3,
        totalPages: 2,
        previewPage: 1,
        previewDataUrl: 'second-preview',
      },
    })
    await Promise.resolve()
    await Promise.resolve()
    expect(state.currentLongSlicePreview.value.previewDataUrl).toBe('second-preview')

    resolveFirst({
      ok: true,
      data: {
        sourceWidth: 100,
        sourceHeight: 1000,
        totalPages: 9,
        previewPage: 1,
        previewDataUrl: 'stale-preview',
      },
    })
    await Promise.resolve()
    await Promise.resolve()
    expect(state.currentLongSlicePreview.value.previewDataUrl).toBe('second-preview')
    expect(state.currentLongSlicePreview.value.sourcePageIndex).toBe(2)
    expect(state.longSliceFiles.value[1].dimText).toBe('200 × 600')
    expect(state.previewLoading.value).toBe(false)
  })

  it('默认显示当前源文件目录，并保留可修改的自定义输出目录', () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    state.longSliceFiles.value = [
      { path: '/case/first.png', name: 'first.png' },
      { path: '/other/second.png', name: 'second.png' },
    ]
    expect(state.longSliceEffectiveOutputDir.value).toBe('/case')
    state.selectedLongSliceIndex.value = 1
    expect(state.longSliceEffectiveOutputDir.value).toBe('/other')

    state.longSliceOutputDir.value = '/chosen'
    expect(state.longSliceEffectiveOutputDir.value).toBe('/chosen')
    expect(state.getLongSliceArgs('/other/second.png').outputDir).toBe('/chosen')
    state.longSliceOutputDir.value = ''
    expect(state.longSliceEffectiveOutputDir.value).toBe('/other')
    expect(state.getLongSliceArgs('/other/second.png').outputDir).toBe(null)
  })

  it('页脚复用完整模板、实际示例与总页数开关', () => {
    const state = PdfToolsView.setup({}, { expose: vi.fn() })
    expect(state.longSliceFooterPresets.value.length).toBeGreaterThanOrEqual(10)
    state.longSliceFooterNumberStyle.value = 'Page {page} of {total}'
    state.longSliceFooterDigitStyle.value = 'roman-upper'
    expect(state.longSliceFooterPresetOptions.value[0].label).toContain('I/V')
    expect(state.longSliceFooterPreview.value).toBe('Page I of V')
    const args = state.getLongSliceArgs('/case/long.png')
    expect(args.footerPageNumberStyle).toBe('Page {page} of {total}')
    expect(args.footerDigitStyle).toBe('roman-upper')

    state.longSliceFooterShowTotal.value = false
    state.onLongSliceFooterShowTotalChange()
    expect(state.longSliceFooterNumberStyle.value).toBe('Page {page}')
    expect(state.longSliceFooterPreview.value).toBe('Page I')
    state.longSliceFooterShowTotal.value = true
    state.onLongSliceFooterShowTotalChange()
    expect(state.longSliceFooterNumberStyle.value).toBe('Page {page} of {total}')
    expect(state.longSliceFooterPreview.value).toBe('Page I of V')
  })

  it('连续调整设置只触发一次延迟预览', async () => {
    vi.useFakeTimers()
    try {
      const state = PdfToolsView.setup({}, { expose: vi.fn() })
      state.longSliceFiles.value = [{ path: '/case/long.png', name: 'long.png' }]
      tauriCallSafe.mockResolvedValue({
        ok: true,
        data: {
          sourceWidth: 100,
          sourceHeight: 1000,
          totalPages: 2,
          previewPage: 1,
          previewDataUrl: 'latest-preview',
        },
      })
      state.schedulePreviewRefresh()
      state.schedulePreviewRefresh()
      state.schedulePreviewRefresh()
      expect(tauriCallSafe).not.toHaveBeenCalled()
      await vi.advanceTimersByTimeAsync(249)
      expect(tauriCallSafe).not.toHaveBeenCalled()
      await vi.advanceTimersByTimeAsync(1)
      expect(tauriCallSafe).toHaveBeenCalledTimes(1)
      expect(tauriCallSafe).toHaveBeenCalledWith('preview_long_slice', expect.objectContaining({ previewPage: 1 }))
      expect(state.previewLoading.value).toBe(false)
      expect(state.currentLongSlicePreview.value.previewDataUrl).toBe('latest-preview')
    } finally {
      vi.useRealTimers()
    }
  })
})
