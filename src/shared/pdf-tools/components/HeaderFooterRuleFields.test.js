import { describe, expect, it, vi } from 'vitest'
import { reactive } from 'vue'
vi.mock('vue', async original => ({ ...(await original()), useSSRContext: () => ({ modules: new Set() }) }))
import Fields from './HeaderFooterRuleFields.vue'
import { createDefaultHeaderGroup, createDefaultFooterTextGroup, createDefaultPageNumberGroup } from '../composables/useEvidencePdfSession.js'

describe('header/footer settings', () => {
  it('updates and records the visible fallback group when its selected ID is stale', () => {
    const group = createDefaultHeaderGroup()
    const props = reactive({ headerGroups: [group], selectedHeaderGroupId: 'missing', pageNumberGroups: [], numberingDefaults: {}, files: [] })
    const emit = vi.fn()
    const state = Fields.setup(props, { emit, expose: vi.fn() })
    state.applyHeaderOverrides({ '/a.pdf': '单独页眉' })
    expect(emit).toHaveBeenCalledWith('update:headerGroups', [expect.objectContaining({ id: group.id, fileHeaderOverrides: { '/a.pdf': '单独页眉' } })])
    expect(emit).toHaveBeenCalledWith('header-group-edit', expect.objectContaining({ before: expect.objectContaining({ id: group.id }), after: expect.objectContaining({ fileHeaderOverrides: { '/a.pdf': '单独页眉' } }) }))
  })
  it('only warns about overlapping sections that are actually enabled', () => {
    const props = reactive({ headerGroups: [createDefaultHeaderGroup()], selectedHeaderGroupId: 'h1', headerInsertEnabled: true,
      footerTextGroups: [{ ...createDefaultFooterTextGroup(), text: '页脚文字' }], selectedFooterTextGroupId: 'ft1', footerInsertEnabled: false,
      pageNumberGroups: [createDefaultPageNumberGroup()], selectedPageNumberGroupId: 'pn1', pageNumberEnabled: false,
      numberingDefaults: {}, headerMode: 'filename', files: [] })
    const state = Fields.setup(props, { emit: vi.fn(), expose: vi.fn() })
    expect(state.overlapWarnings.value).toEqual([])
    props.footerInsertEnabled = true
    props.pageNumberEnabled = true
    expect(state.overlapWarnings.value).toHaveLength(1)
    props.footerInsertEnabled = false
    expect(state.overlapWarnings.value).toEqual([])
  })
})
