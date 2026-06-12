import { defineStore } from 'pinia'
import { ref } from 'vue'
import { tauriCall } from '../core/tauriBridge.js'

export const useTemplateStore = defineStore('template', () => {
  const templates = ref([])
  const currentTemplate = ref(null)

  async function loadTemplates() {
    try {
      templates.value = await tauriCall('list_templates')
    } catch (err) {
      console.error('Failed to load templates:', err)
    }
  }

  async function loadTemplate(templateId) {
    try {
      currentTemplate.value = await tauriCall('get_template_meta', { templateId })
    } catch (err) {
      console.error('Failed to load template:', err)
    }
  }

  return { templates, currentTemplate, loadTemplates, loadTemplate }
})
