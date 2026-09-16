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
})
