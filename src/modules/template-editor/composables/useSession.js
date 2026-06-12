import { ref } from 'vue'
import { createSession, loadSession, save as saveSession } from './useTemplateSave.js'

export function useSession() {
  const session = ref(null)
  const loading = ref(false)
  const saving = ref(false)

  async function loadFromDocx(docxPath) {
    loading.value = true
    try {
      session.value = await createSession(docxPath)
    } finally {
      loading.value = false
    }
  }

  async function loadFromTemplate(templateId) {
    loading.value = true
    try {
      session.value = await loadSession(templateId)
    } finally {
      loading.value = false
    }
  }

  async function save() {
    if (!session.value) return null
    saving.value = true
    try {
      const id = await saveSession(session.value)
      session.value.template_id = id
      return id
    } finally {
      saving.value = false
    }
  }

  function clear() {
    session.value = null
  }

  return {
    session,
    loading,
    saving,
    loadFromDocx,
    loadFromTemplate,
    save,
    clear,
  }
}
