import { defineStore } from 'pinia'
import { ref } from 'vue'
import { tauriCall } from '../core/tauriBridge.js'

export const useAppStore = defineStore('app', () => {
  const settings = ref({
    history_max: 50,
    menu_visibility: {},
    libreoffice_path: '',
  })

  async function loadSettings() {
    try {
      settings.value = await tauriCall('get_app_settings')
    } catch (err) {
      console.error('Failed to load settings:', err)
    }
  }

  async function saveSettings() {
    try {
      await tauriCall('set_app_settings', { settings: settings.value })
    } catch (err) {
      console.error('Failed to save settings:', err)
    }
  }

  return { settings, loadSettings, saveSettings }
})
