import { defineStore } from 'pinia'
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { tauriCallSafe } from '../core/tauriBridge.js'
import { logError } from '../services/appLogger.js'

export const useAppStore = defineStore('app', () => {
  const settings = ref({
    menu_visibility: {},
    menu_order: [],
    libreoffice_path: '',
    tool_manifest_url: '',
    onboarding_completed: false,
  })

  let recoveryNoticeShown = false
  async function loadSettings() {
    const result = await tauriCallSafe('get_app_settings')
    if (result.ok) {
      if (result.data?.recovery_warning && !recoveryNoticeShown) {
        recoveryNoticeShown = true
        ElMessage.warning({ message: result.data.recovery_warning, duration: 10000, showClose: true })
      }
      settings.value = {
        ...settings.value,
        ...result.data,
      }
    } else {
      void logError('app.store', 'load settings failed', { error: result.error })
    }
  }

  async function saveSettings() {
    const result = await tauriCallSafe('set_app_settings', { settings: settings.value })
    if (!result.ok) {
      void logError('app.store', 'save settings failed', { error: result.error })
    }
  }

  async function completeOnboarding() {
    settings.value.onboarding_completed = true
    await saveSettings()
  }

  return { settings, loadSettings, saveSettings, completeOnboarding }
})
