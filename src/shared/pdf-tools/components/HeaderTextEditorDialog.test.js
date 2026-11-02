import { describe, expect, it, vi } from 'vitest'
import { reactive } from 'vue'
vi.mock('vue', async (original) => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import Dialog from './HeaderTextEditorDialog.vue'
import { createDefaultHeaderGroup } from '../composables/useEvidencePdfSession.js'

function setup(
  group,
  files = [
    { path: '/a.pdf', name: '9. 证据文件1-专利.pdf' },
    { path: '/b.pdf', name: '10. 证据文件2-专利.pdf' },
  ],
  extra = {},
) {
  const props = reactive({
    files,
    group,
    numberingDefaults: { evidenceStart: 21 },
    dateValue: '',
    indexOffset: 0,
    ...extra,
  })
  const emit = vi.fn(),
    state = Dialog.setup(props, { emit, expose: vi.fn() })
  state.open()
  return { state, props, emit }
}

describe('editing complete per-file headers', () => {
  it('starts with the full resolved prefix and suffix and only saves changed files', () => {
    const group = { ...createDefaultHeaderGroup(), mode: 'per_file', perFilePrefix: '证据', prefix: '【', suffix: '】' }
    const { state, emit } = setup(group)
    expect(state.rows.value.map((row) => row.value)).toEqual(['【证据21】', '【证据22】'])
    state.updateText(state.rows.value[0], '已编辑')
    expect(state.changedCount.value).toBe(1)
    state.apply()
    expect(emit).toHaveBeenCalledWith('apply', { '/a.pdf': '已编辑' })
    expect(group.fileHeaderOverrides).toBeUndefined()
  })
  it('restores an individual override to the live rule, preserves untouched overrides and honours original list indices', () => {
    const group = {
      ...createDefaultHeaderGroup(),
      mode: 'per_file',
      perFilePrefix: '证据',
      prefix: '【',
      suffix: '】',
      fileHeaderOverrides: { '/a.pdf': '单独名称', '/b.pdf': '另一个' },
    }
    const { state, emit } = setup(group, [{ path: '/a.pdf', name: 'a.pdf' }], { indexOffset: 3 })
    expect(state.rows.value[0].value).toBe('单独名称')
    expect(state.rows.value[0].base).toBe('【证据24】')
    state.restoreRow(state.rows.value[0])
    state.apply()
    expect(emit).toHaveBeenCalledWith('apply', { '/b.pdf': '另一个' })
  })
  it('can remove an explicit override that happens to equal the current generated text', () => {
    const group = {
      ...createDefaultHeaderGroup(),
      mode: 'custom',
      text: '相同页眉',
      fileHeaderOverrides: { '/a.pdf': '相同页眉' },
    }
    const { state, emit } = setup(group)
    state.restoreRow(state.rows.value[0])
    expect(state.changedCount.value).toBe(1)
    state.apply()
    expect(emit).toHaveBeenCalledWith('apply', {})
  })
  it('selects from each filename without its extension, irrespective of the current header source', () => {
    const group = { ...createDefaultHeaderGroup(), mode: 'per_file', perFilePrefix: '证据', prefix: '【', suffix: '】' }
    const { state } = setup(group, [
      { path: '/a.pdf', name: '甲-1.pdf' },
      { path: '/b.pdf', name: '乙-2.pdf' },
    ])
    const first = state.rows.value[0]
    state.toggleSelection(first)
    expect(first.value).toBe('【证据21】')
    expect(first.selectionSource).toBe('甲-1')
    expect(first.selected).toEqual([])
    state.setGranularity(first, 'character')
    state.toggleToken(first, 2)
    state.toggleToken(first, 0)
    expect(first.value).toBe('甲1')
    state.applyPositions(0)
    expect(state.rows.value.map((row) => row.value)).toEqual(['甲1', '乙2'])
  })
  it('keeps the edited header intact when opening or changing the filename split mode', () => {
    const { state } = setup({ ...createDefaultHeaderGroup(), mode: 'custom', text: '原页眉' })
    const row = state.rows.value[0]
    state.updateText(row, '手动修改')
    state.toggleSelection(row)
    expect(row.selectionSource).toBe('9. 证据文件1-专利')
    expect(row.value).toBe('手动修改')
    state.setGranularity(row, 'character')
    expect(row.value).toBe('手动修改')
    state.toggleToken(row, 3)
    expect(row.value).toBe('证')
    state.setGranularity(row, 'word')
    expect(row.value).toBe('证')
    expect(row.selected).toEqual([])
  })
  it('shows positional batch errors beside the originating filename without changing other headers', () => {
    const { state } = setup({ ...createDefaultHeaderGroup(), mode: 'custom', text: '原页眉' }, [
      { path: '/a.pdf', name: '甲乙丙.pdf' }, { path: '/b.pdf', name: '丁.pdf' },
    ])
    const first = state.rows.value[0]
    state.toggleSelection(first)
    state.setGranularity(first, 'character')
    state.toggleToken(first, 2)
    state.applyPositions(0)
    expect(first.selectionError).toContain('片段数量不足')
    expect(state.rows.value[1].value).toBe('原页眉')
    expect(state.batchOpen.value).toBe(false)
  })
  it('rejects unsafe batch ranges as a whole and supports filename extraction plus replacement', () => {
    const group = { ...createDefaultHeaderGroup(), mode: 'custom', text: '现有页眉' }
    const { state } = setup(group)
    Object.assign(state.batch.value, {
      mode: 'extract',
      source: 'filename',
      separator: '-',
      start: 1,
      end: 3,
      removeOrder: true,
    })
    state.applyBatch()
    expect(state.rows.value.map((row) => row.value)).toEqual(['现有页眉', '现有页眉'])
    expect(state.batchError.value).toContain('未应用')
    Object.assign(state.batch.value, { end: 1, find: '证据文件', replace: '对比文件' })
    state.applyBatch()
    expect(state.rows.value.map((row) => row.value)).toEqual(['对比文件1', '对比文件2'])
  })
  it('treats cancel as a draft and does not apply an empty changed header', () => {
    const { state, emit } = setup({ ...createDefaultHeaderGroup(), mode: 'custom', text: '原页眉' })
    state.updateText(state.rows.value[0], '')
    expect(state.hasInvalid.value).toBe(true)
    state.apply()
    expect(emit).not.toHaveBeenCalled()
    state.visible.value = false
    state.open()
    expect(state.rows.value[0].value).toBe('原页眉')
  })
})
