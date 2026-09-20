import { describe, expect, it, vi } from 'vitest'
import { reactive, ref } from 'vue'

vi.mock('../tauriBridge.js', () => ({ tauriCallSafe: vi.fn() }))

import { useWorkspacePreferences, workspacePreferenceInternals } from './useWorkspacePreferences.js'

import { tauriCallSafe } from '../tauriBridge.js'
import { migratePaddlerPreferences } from '../../modules/image-paddler/composables/outputPlan.js'

describe('workspace preference bindings', () => {
  it('applies saved refs and nested reactive settings', () => {
    const mode = ref('old')
    const settings = reactive({ quality: 80, timestamp: { enabled: false, color: 'white' } })
    workspacePreferenceInternals.applyBindings(
      { mode, settings },
      { mode: 'new', settings: { quality: 95, timestamp: { enabled: true } } },
    )
    expect(mode.value).toBe('new')
    expect(settings).toEqual({ quality: 95, timestamp: { enabled: true, color: 'white' } })
  })

  it('serializes refs and objects without Vue proxies', () => {
    const values = workspacePreferenceInternals.readBindings({ tab: ref('merge'), settings: reactive({ margin: 12 }) })
    expect(values).toEqual({ tab: 'merge', settings: { margin: 12 } })
  })
})


it('loads old manual width before defaults and preserves explicit new global scale', async () => {
  const settings = reactive({ size_mode: 'smart', scale_mode: 'fixed_width', fixed_width_mm: 160 })
  const pageScales = ref({})
  const globalScalePercent = ref(100)
  tauriCallSafe.mockResolvedValue({ ok: true, data: { settings: { fixed_width_mm: 123, scale_mode: 'fixed_width' }, pageScales: { 0: 1.2 }, globalScalePercent: 90 } })
  const preference = useWorkspacePreferences('image-paddler.workspace', { settings, pageScales, globalScalePercent }, { migrate: migratePaddlerPreferences })
  await preference.start()
  expect(settings.size_mode).toBe('manual')
  expect(settings.fixed_width_mm).toBe(123)
  expect(pageScales.value).toEqual({})
  expect(globalScalePercent.value).toBe(90)
  await preference.stop()
})

it('does not overwrite backend preferences when the initial load fails', async () => {
  const mode = ref('user-value')
  tauriCallSafe.mockReset()
  tauriCallSafe.mockResolvedValue({ ok: false, error: 'db locked' })
  const preference = useWorkspacePreferences('video-extract.workspace', { mode })
  await preference.start()
  mode.value = 'local-edit'
  await preference.stop()
  const commands = tauriCallSafe.mock.calls.map(call => call[0])
  expect(commands).toContain('get_workspace_preference')
  expect(commands).not.toContain('set_workspace_preference')
})


