/**
 * 开发用用户操作追踪器
 *
 * 记录用户在应用中的点击、输入、选择等操作，
 * 与后端日志配合用于比对"用户做了什么 → 系统做了什么 → 结果是什么"。
 *
 * 仅在 __DEV__ = true 时生效，生产构建自动移除。
 */

import { logDebug, logInfo } from './appLogger.js'

const DEV = import.meta.env.DEV

let _sessionLog = []
let _sessionId = `dev_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`

function record(action, target, detail) {
  if (!DEV) return
  const entry = {
    ts: new Date().toISOString(),
    seq: _sessionLog.length,
    action,
    target,
    detail,
  }
  _sessionLog.push(entry)
  logDebug('dev.tracker', action, { seq: entry.seq, target, ...detail })
}

/**
 * 安装全局操作追踪
 */
export function installDevTracker() {
  if (!DEV) return

  // 点击追踪
  document.addEventListener(
    'click',
    (e) => {
      const el = e.target
      const tag = el.tagName?.toLowerCase() || '?'
      const text = el.textContent?.trim().slice(0, 50) || ''
      const classes = el.className || ''
      const button = el.closest('button, [role=button], .el-button')
      const input = el.closest('input, textarea, .el-input, .el-select')

      if (button) {
        record('click_button', button.textContent?.trim().slice(0, 50), {
          tag,
          buttonText: button.textContent?.trim().slice(0, 80),
          buttonClass: button.className?.slice(0, 100),
        })
      } else if (input) {
        record('click_input', input.placeholder || input.className?.slice(0, 50), {
          tag,
          inputType: input.type || input.tagName,
        })
      } else if (tag === 'a' || el.closest('a')) {
        const link = el.closest('a')
        record('click_link', link?.href || text, { text })
      } else {
        record('click_element', `${tag}.${classes?.slice(0, 30)}`, { text })
      }
    },
    true,
  )

  // 输入追踪（防抖，只记录最终值）
  const inputTimers = new Map()
  document.addEventListener(
    'input',
    (e) => {
      const el = e.target
      if (!el || el.tagName === 'BODY') return
      const key = el
      if (inputTimers.has(key)) clearTimeout(inputTimers.get(key))
      inputTimers.set(
        key,
        setTimeout(() => {
          inputTimers.delete(key)
          const value = el.value ?? ''
          const displayValue = value.length > 100 ? `${value.slice(0, 100)}...[len=${value.length}]` : value
          record('input_change', el.placeholder || el.className?.slice(0, 50), {
            tag: el.tagName,
            inputType: el.type || '',
            value: displayValue,
          })
        }, 500),
      )
    },
    true,
  )

  // 选择追踪
  document.addEventListener(
    'change',
    (e) => {
      const el = e.target
      if (!el) return
      if (el.tagName === 'SELECT' || el.classList?.contains('el-select')) {
        record('select_change', el.placeholder || 'select', {
          value: el.value,
          selectedText: el.options?.[el.selectedIndex]?.text || '',
        })
      } else if (el.type === 'checkbox' || el.type === 'radio') {
        record('toggle', el.name || el.id || 'checkbox', {
          checked: el.checked,
          value: el.value,
        })
      } else if (el.type === 'file') {
        const files = Array.from(el.files || []).map((f) => f.name)
        record('file_select', el.id || 'file-input', { files })
      }
    },
    true,
  )

  // 表单提交追踪
  document.addEventListener(
    'submit',
    (e) => {
      const form = e.target
      record('form_submit', form?.action || form?.id || 'form', {
        method: form?.method,
      })
    },
    true,
  )

  void logInfo('dev.tracker', 'installed', { sessionId: _sessionId })
}

/**
 * 获取当前会话的操作日志
 */
export function getDevSessionLog() {
  return [..._sessionLog]
}

/**
 * 导出操作日志为文本
 */
export function exportDevLog() {
  return _sessionLog
    .map((e) => `[${e.ts}] #${e.seq} ${e.action} | ${e.target} | ${JSON.stringify(e.detail)}`)
    .join('\n')
}

/**
 * 清空操作日志
 */
export function clearDevLog() {
  _sessionLog = []
}
