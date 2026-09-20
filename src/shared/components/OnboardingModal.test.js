import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('vue', async importOriginal => ({ ...(await importOriginal()), onMounted: vi.fn(), onUnmounted: vi.fn(), useSSRContext: () => ({ modules: new Set() }) }))
vi.mock('../../stores/app.js', () => ({ useAppStore: () => ({ settings: { onboarding_completed: true }, loadSettings: vi.fn() }) }))
vi.mock('../../core/tauriBridge.js', () => ({ tauriCallSafe: vi.fn() }))
vi.mock('../../core/terminalInstall.js', () => ({ isMac: false, installToolsBatchViaTerminal: vi.fn() }))
import { tauriCallSafe } from '../../core/tauriBridge.js'
import OnboardingModal from './OnboardingModal.vue'

describe('组件安装复检', () => {
  beforeEach(() => vi.clearAllMocks())
  it.each([false, true])('安装命令成功后，复检可用=%s 才决定就绪', async available => {
    tauriCallSafe.mockImplementation(async command => {
      if (command === 'install_external_tool') return { ok: true }
      if (command === 'check_ffmpeg') return { ok: true, data: { has_drawtext: available } }
      return { ok: true, data: { available } }
    })
    const state = OnboardingModal.setup({}, { expose: vi.fn() })
    await state.startSetup()
    expect(state.isAllDone.value).toBe(available)
    expect(state.featureList.every(item => item.ready === available)).toBe(true)
    expect(state.featureList.every(item => item.status === (available ? 'ready' : 'error'))).toBe(true)
  })
})
