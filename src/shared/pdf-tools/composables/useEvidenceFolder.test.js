import { beforeEach, describe, expect, it, vi } from 'vitest'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { useEvidenceFolder } from './useEvidenceFolder.js'

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('../../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn(), userFacingError: (message) => message }))
vi.mock('element-plus', () => ({ ElMessage: { success: vi.fn(), warning: vi.fn(), error: vi.fn() } }))

describe('证据扫描工作流', () => {
  beforeEach(() => vi.clearAllMocks())
  it('换目录立即清除旧分组，扫描失败不能用旧分组生成', async () => {
    const state = useEvidenceFolder()
    state.evidenceGroups.value = [{ name: '旧组', files: [] }]
    open.mockResolvedValue('/new')
    tauriCallSafe.mockResolvedValue({ ok: false, error: '无法扫描' })
    await state.selectEvidenceFolder()
    await state.buildEvidence()
    expect(state.evidenceGroups.value).toEqual([])
    expect(tauriCallSafe).toHaveBeenCalledTimes(1)
    expect(state.scanning.value).toBe(false)
  })
  it('零产出不能报告成功', async () => {
    const state = useEvidenceFolder()
    state.evidenceGroups.value = [{ name: '组', files: [] }]
    tauriCallSafe.mockResolvedValue({ ok: true, data: { results: [], failedConversions: [{ name: '坏.docx' }] } })
    await state.buildEvidence()
    expect(ElMessage.success).not.toHaveBeenCalled()
    expect(ElMessage.warning).toHaveBeenCalled()
    expect(state.conversionFailures.value).toHaveLength(1)
    expect(state.building.value).toBe(false)
  })
  it('按用户顺序生成分组并提供输出路径', async () => {
    const state = useEvidenceFolder()
    state.evidenceFolder.value = '/root'
    state.evidenceGroups.value = [{ name: '组', files: [{ path: '2.pdf' }, { path: '1.pdf' }] }]
    tauriCallSafe.mockResolvedValue({ ok: true, data: { results: [{}], evidenceDir: '/root/_evidence_output' } })
    await state.buildEvidence()
    expect(tauriCallSafe.mock.calls[0][1].args.groups[0].files.map(file => file.path)).toEqual(['2.pdf', '1.pdf'])
    expect(state.evidenceOutputDir.value).toBe('/root/_evidence_output')
  })
  it('手动 PDF 替换失败的 Word 并保留位置，不重新导入已经列出的同一 PDF', async () => {
    const state = useEvidenceFolder()
    state.evidenceGroups.value = [{ id: 'g', name: '组', files: [{ path: '/a.pdf', fileType: 'pdf' }, { path: '/b.docx', fileType: 'word' }, { path: '/manual/b.pdf', fileType: 'pdf' }] }]
    state.conversionFailures.value = [{ groupId: 'g', path: '/b.docx' }]
    open.mockResolvedValueOnce('/manual/b.pdf')
    await state.replaceFailedConversion(state.conversionFailures.value[0])
    expect(state.evidenceGroups.value[0].files.map(file => file.path)).toEqual(['/a.pdf', '/manual/b.pdf'])
    expect(state.evidenceGroups.value[0].files[1].fileType).toBe('pdf')
    expect(state.conversionFailures.value).toEqual([])
  })
  it('previews images inside each folder, keeps document order, and passes the two duplex boundaries independently', async () => {
    const review = vi.fn().mockResolvedValue('/root/_evidence_output/images.pdf')
    const state = useEvidenceFolder({ reviewImages: review })
    state.evidenceFolder.value = '/root'; state.outputMode.value = 'all'; state.duplexFiles.value = true; state.duplexGroups.value = false
    state.evidenceGroups.value = [{ name: 'root', id: 'r', files: [{ path: '/root/a.pdf', fileType: 'pdf' }, { path: '/root/b.png', fileType: 'image' }, { path: '/root/c.jpg', fileType: 'image' }, { path: '/root/d.docx', fileType: 'word' }] }]
    tauriCallSafe.mockResolvedValue({ ok: true, data: { results: [{}], evidenceDir: '/root/_evidence_output' } })
    await state.buildEvidence()
    expect(review).toHaveBeenCalledWith(['/root/b.png', '/root/c.jpg'], { outputDir: '/root/_evidence_output' })
    expect(tauriCallSafe.mock.calls[0][1].args).toMatchObject({ outputMode: 'all', duplexFiles: true, duplexGroups: false })
    expect(state.evidenceGroups.value[0].files.map(file => file.path)).toEqual(['/root/a.pdf', '/root/_evidence_output/images.pdf', '/root/d.docx'])
    state.outputMode.value = 'per_folder'; state.duplexGroups.value = true
    await state.buildEvidence()
    expect(tauriCallSafe.mock.calls[1][1].args.duplexGroups).toBe(false)
  })
  it('cancelling image review never starts a partial merge or removes original image entries', async () => {
    const state = useEvidenceFolder({ reviewImages: vi.fn().mockResolvedValue(null) })
    state.evidenceFolder.value = '/root'; state.evidenceGroups.value = [{ name: 'root', id: 'r', files: [{ path: '/root/a.png', fileType: 'image' }, { path: '/root/b.pdf', fileType: 'pdf' }] }]
    await state.buildEvidence()
    expect(tauriCallSafe).not.toHaveBeenCalled()
    expect(state.evidenceGroups.value[0].files.map(file => file.path)).toEqual(['/root/a.png', '/root/b.pdf'])
    expect(state.building.value).toBe(false)
  })

})
