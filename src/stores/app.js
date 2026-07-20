import { defineStore } from 'pinia'
import { ref } from 'vue'
import { tauriCall } from '../core/tauriBridge.js'
import { logError } from '../services/appLogger.js'

export const useAppStore = defineStore('app', () => {
  const settings = ref({
    menu_visibility: {},
    menu_order: [],
    libreoffice_path: '',
    tool_manifest_url: '',
  })

  async function loadSettings() {
    try {
      settings.value = await tauriCall('get_app_settings')
    } catch (err) {
      void logError('app.store', 'load settings failed', { error: err })
    }
  }

  async function saveSettings() {
    try {
      await tauriCall('set_app_settings', { settings: settings.value })
    } catch (err) {
      void logError('app.store', 'save settings failed', { error: err })
    }
  }

  return { settings, loadSettings, saveSettings }
})
