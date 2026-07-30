import { defineStore } from 'pinia'
import { ref } from 'vue'
import { tauriCallSafe } from '../core/tauriBridge.js'
import { logError } from '../services/appLogger.js'

export const useAppStore = defineStore('app', () => {
  const settings = ref({
    menu_visibility: {},
    menu_order: [],
    libreoffice_path: '',
    tool_manifest_url: '',
  })

  async function loadSettings() {
    const result = await tauriCallSafe('get_app_settings')
    if (result.ok) {
      settings.value = result.data
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

  return { settings, loadSettings, saveSettings }
})
