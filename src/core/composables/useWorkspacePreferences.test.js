import { describe, expect, it, vi } from 'vitest'
import { reactive, ref } from 'vue'

vi.mock('../tauriBridge.js', () => ({ tauriCallSafe: vi.fn() }))

import { workspacePreferenceInternals } from './useWorkspacePreferences.js'

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
