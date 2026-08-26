import { isRef, watch } from 'vue'
import { tauriCallSafe } from '../tauriBridge.js'

function cloneValue(value) {
  if (value === undefined) return undefined
  return JSON.parse(JSON.stringify(value))
}

function mergeInto(target, source) {
  if (!source || typeof source !== 'object' || Array.isArray(source)) return
  for (const [key, value] of Object.entries(source)) {
    if (
      value &&
      typeof value === 'object' &&
      !Array.isArray(value) &&
      target[key] &&
      typeof target[key] === 'object' &&
      !Array.isArray(target[key])
    ) {
      mergeInto(target[key], value)
    } else {
      target[key] = cloneValue(value)
    }
  }
}

function readBindings(bindings) {
  return Object.fromEntries(
    Object.entries(bindings).map(([key, binding]) => [key, cloneValue(isRef(binding) ? binding.value : binding)]),
  )
}

function applyBindings(bindings, value) {
  if (!value || typeof value !== 'object') return
  for (const [key, saved] of Object.entries(value)) {
    const binding = bindings[key]
    if (!binding) continue
    if (isRef(binding)) {
      binding.value = cloneValue(saved)
    } else if (binding && typeof binding === 'object' && saved && typeof saved === 'object') {
      mergeInto(binding, saved)
    }
  }
}

export function useWorkspacePreferences(scope, bindings, options = {}) {
  const debounceMs = Math.max(100, Number(options.debounceMs || 500))
  let loaded = false
  let stopped = false
  let timer = null
  let stopWatch = null

  async function save() {
    if (!loaded) return false
    if (timer) window.clearTimeout(timer)
    timer = null
    const result = await tauriCallSafe('set_workspace_preference', {
      scope,
      value: readBindings(bindings),
    })
    return result.ok
  }

  function scheduleSave() {
    if (!loaded || typeof window === 'undefined') return
    if (timer) window.clearTimeout(timer)
    timer = window.setTimeout(save, debounceMs)
  }

  async function start() {
    if (loaded) return
    stopped = false
    const result = await tauriCallSafe('get_workspace_preference', { scope })
    if (stopped) return
    if (result.ok && result.data) applyBindings(bindings, result.data)
    loaded = true
    stopWatch = watch(() => readBindings(bindings), scheduleSave, { deep: true })
  }

  async function stop() {
    stopped = true
    if (stopWatch) stopWatch()
    stopWatch = null
    if (timer) await save()
  }

  return { start, save, stop }
}

export const workspacePreferenceInternals = { mergeInto, readBindings, applyBindings }
