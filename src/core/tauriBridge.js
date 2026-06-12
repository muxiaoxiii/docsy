import { invoke } from '@tauri-apps/api/core'

export async function tauriCall(command, args = {}) {
  try {
    return await invoke(command, args)
  } catch (err) {
    console.error(`[tauri] ${command} failed:`, err)
    throw err
  }
}

export async function tauriCallSafe(command, args = {}) {
  try {
    const result = await invoke(command, args)
    return { ok: true, data: result }
  } catch (err) {
    return { ok: false, error: String(err) }
  }
}
