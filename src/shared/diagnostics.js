/**
 * 前端诊断模块 — 结构化日志 + 状态快照
 *
 * 用法:
 *   import { log, snapshot, exportReport } from '@/shared/diagnostics'
 *   log.info('template.build', '模板保存成功', { fields: 5 })
 *   log.warn('pdf.overlay', '处理较慢', { duration_ms: 3200 })
 *   snapshot('template', () => ({ path: templatePath.value, fields: fieldRows.value.length }))
 */

import { tauriCallSafe } from '../core/tauriBridge.js'

const LEVEL_ORDER = { debug: 0, info: 1, warn: 2, error: 3 }
let minLevel = LEVEL_ORDER.debug

// In-memory ring buffer for recent entries (for export)
const ringBuffer = []
const MAX_RING = 500

function pushRing(entry) {
  ringBuffer.push(entry)
  if (ringBuffer.length > MAX_RING) ringBuffer.shift()
}

// State snapshot providers
const snapshotProviders = new Map()

function writeEntry(level, target, message, context = null, operationId = null) {
  if (LEVEL_ORDER[level] < minLevel) return
  const entry = {
    ts: new Date().toISOString(),
    level,
    target,
    message,
    context,
    op: operationId,
  }
  pushRing(entry)

  // Fire-and-forget to Rust backend (structured log file)
  tauriCallSafe('write_frontend_log', {
    level,
    target,
    message,
    context: context ? JSON.stringify(context) : null,
  })
}

export const log = {
  debug(target, message, context) {
    writeEntry('debug', target, message, context)
  },
  info(target, message, context) {
    writeEntry('info', target, message, context)
  },
  warn(target, message, context) {
    writeEntry('warn', target, message, context)
  },
  error(target, message, context, operationId) {
    writeEntry('error', target, message, context, operationId)
  },
}

/**
 * 注册模块状态快照提供者
 * @param {string} name - 模块名
 * @param {() => object} providerFn - 返回当前状态的函数
 */
export function registerSnapshotProvider(name, providerFn) {
  snapshotProviders.set(name, providerFn)
}

/**
 * 收集所有快照
 */
export function collectSnapshots() {
  const snapshots = {}
  for (const [name, fn] of snapshotProviders) {
    try {
      snapshots[name] = fn()
    } catch (err) {
      snapshots[name] = { error: err.message }
    }
  }
  return snapshots
}

/**
 * 导出诊断报告
 */
export async function exportReport() {
  const snapshots = collectSnapshots()
  const result = await tauriCallSafe('export_diagnostic_report', { frontendSnapshots: snapshots })
  if (!result.ok) {
    log.error('diagnostics', '导出诊断报告失败', { error: result.error })
    return null
  }
  return result.data?.path || null
}

/**
 * 设置最小日志级别
 */
export function setMinLevel(level) {
  minLevel = LEVEL_ORDER[level] ?? LEVEL_ORDER.debug
}

/**
 * 获取最近的日志条目（用于调试）
 */
export function getRecentEntries(count = 50) {
  return ringBuffer.slice(-count)
}
