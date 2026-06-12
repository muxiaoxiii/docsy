import { tauriCall } from '../../../core/tauriBridge.js'

export async function save(session) {
  return await tauriCall('save_template', { session })
}

export async function createSession(docxPath) {
  return await tauriCall('create_editor_session', { docxPath })
}

export async function loadSession(templateId) {
  return await tauriCall('load_editor_session', { templateId })
}
