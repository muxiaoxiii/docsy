import { describe, expect, it, vi } from 'vitest'
import { reactive } from 'vue'
vi.mock('vue', async (original) => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import Dialog from './FilenameHeaderDialog.vue'

describe('filename header rule draft', () => {
  it('keeps edits isolated until applying, restores individual overrides and prevents incomplete extraction', () => {
    const props = reactive({
      files: [{ name: '9. 证据文件8-专利号.pdf', path: '/case/a.pdf' }],
      rule: { separator: '-', start: 1, end: 1, overrides: {} },
    })
    const emit = vi.fn(),
      state = Dialog.setup(props, { emit, expose: vi.fn() })
    state.open()
    state.useComparisonNames()
    expect(state.rows.value[0].text).toBe('对比文件8')
    state.draft.value.end = 3
    expect(state.invalidCount.value).toBe(1)
    state.apply()
    expect(emit).not.toHaveBeenCalled()
    state.draft.value.end = 1
    expect(state.invalidCount.value).toBe(0)
    state.setOverride('/case/a.pdf', '单项修正')
    expect(state.rows.value[0].text).toBe('单项修正')
    expect(state.hasOverride('/case/a.pdf')).toBe(true)
    expect(props.rule.overrides).toEqual({})
    state.removeOverride('/case/a.pdf')
    expect(state.rows.value[0].text).toBe('对比文件8')
    state.visible.value = false
    state.open()
    expect(state.draft.value.replaceFrom).toBe('')
    state.useComparisonNames()
    state.apply()
    expect(emit).toHaveBeenLastCalledWith(
      'apply',
      expect.objectContaining({ replaceFrom: '证据文件', replaceTo: '对比文件' }),
    )
    expect(props.rule).not.toHaveProperty('replaceFrom')
    expect(state.visible.value).toBe(false)
  })
})
