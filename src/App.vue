<template>
  <el-container class="app-container">
    <el-aside
      :width="sidebarCollapsed ? '78px' : 'var(--docsy-sidebar-width)'"
      class="app-aside"
      :class="{ 'is-collapsed': sidebarCollapsed }"
    >
      <button type="button" class="brand" aria-label="返回首页" @click="router.push('/')">
        <img src="./assets/docsy-logo.png" alt="Docsy" class="brand-logo" />
        <span class="brand-copy">
          <span class="brand-name">Docsy</span>
          <span class="brand-caption">Local Toolkit</span>
        </span>
      </button>
      <div class="sidebar-section-label">工作空间</div>
      <el-menu :default-active="activeMenu" @select="onMenuSelect" class="sidebar-menu">
        <template v-for="(item, index) in menuItems" :key="item.route">
          <el-menu-item :index="item.route" :aria-label="item.label" :title="item.label">
            <span class="menu-index">{{ String(index + 1).padStart(2, '0') }}</span>
            <span
              class="menu-icon"
              :style="{ '--menu-icon-url': `url(&quot;${menuIconByRoute[item.route]}&quot;)` }"
              aria-hidden="true"
            ></span>
            <span class="menu-label">{{ item.label }}</span>
            <span class="menu-arrow">›</span>
          </el-menu-item>
        </template>
      </el-menu>
      <div class="sidebar-footer">
        <el-tooltip content="关于" placement="right">
          <el-button
            class="footer-btn"
            :class="{ active: route.name === 'about' }"
            @click="router.push({ name: 'about' })"
          >
            <el-icon><InfoFilled /></el-icon>
            <span>关于</span>
          </el-button>
        </el-tooltip>
        <el-tooltip content="设置" placement="right">
          <el-button
            class="footer-btn"
            :class="{ active: route.name === 'settings' }"
            @click="router.push({ name: 'settings' })"
          >
            <el-icon><Setting /></el-icon>
            <span>设置</span>
          </el-button>
        </el-tooltip>
      </div>
    </el-aside>
    <el-container>
      <el-header class="app-header">
        <div class="header-leading">
          <button
            type="button"
            class="sidebar-toggle"
            :aria-label="sidebarCollapsed ? '展开工作空间导航' : '收起工作空间导航'"
            :title="sidebarCollapsed ? '展开工作空间' : '收起工作空间'"
            @click="toggleSidebar"
          >
            {{ sidebarCollapsed ? '›' : '‹' }}
          </button>
          <div class="page-heading">
            <span class="page-title">{{ currentPageTitle }}</span>
            <span class="page-context">本地文档处理工作台</span>
          </div>
        </div>
        <span class="app-version">v{{ version }}</span>
      </el-header>
      <el-main class="app-main">
        <router-view v-slot="{ Component, route: currentRoute }">
          <keep-alive :max="2">
            <component :is="Component" v-if="currentRoute.meta.keepAlive" :key="currentRoute.name" />
          </keep-alive>
          <component :is="Component" v-if="!currentRoute.meta.keepAlive" :key="currentRoute.name" />
        </router-view>
      </el-main>
      <Transition name="doclet-operation">
        <div v-if="operationVisible" class="doclet-operation-panel">
          <DocletWorkingPet :message="operationMessage" :elapsed="operationElapsed" />
          <button v-if="showCancel" class="doclet-cancel-btn" @click="cancelCurrentOperation" title="取消当前操作">
            取消
          </button>
        </div>
      </Transition>
    </el-container>
  </el-container>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Setting, InfoFilled } from '@element-plus/icons-vue'
import { getMenuItems } from './core/moduleRegistry.js'
import { confirmAppClose, listActiveOperations, tauriCallSafe } from './core/tauriBridge.js'
import { listen } from '@tauri-apps/api/event'
import { ElMessageBox } from 'element-plus'
import DocletWorkingPet from './shared/components/DocletWorkingPet.vue'
import { useAppStore } from './stores/app.js'
import evidenceIconUrl from './assets/icons/evidence.svg?url'
import documentsIconUrl from './assets/icons/documents.svg?url'
import markdownWordIconUrl from './assets/icons/markdown-word.svg?url'
import imageLayoutIconUrl from './assets/icons/image-layout.svg?url'
import videoFramesIconUrl from './assets/icons/video-frames.svg?url'
import templateIconUrl from './assets/icons/template.svg?url'

const router = useRouter()
const route = useRoute()
const appStore = useAppStore()

const menuItems = computed(() => getMenuItems(appStore.settings))
const menuIconByRoute = {
  'evidence-pdf': evidenceIconUrl,
  'pdf-tools': documentsIconUrl,
  'markdown-convert': markdownWordIconUrl,
  'image-paddler': imageLayoutIconUrl,
  'video-extract': videoFramesIconUrl,
  template: templateIconUrl,
}

const activeMenu = computed(() => route.name || 'home')
const sidebarCollapsed = ref(window.localStorage.getItem('docsy.sidebar.collapsed') === '1')
const operationVisible = ref(false)
const version = import.meta.env.PACKAGE_VERSION || ''
const operationMessage = ref('Doclet 正在处理…')
const operationElapsed = ref('')
const showCancel = ref(false)
let cancelTimer
let operationTimer
let elapsedTimer
const pendingOperations = new Map() // id -> { label, startTime }

const currentPageTitle = computed(() => {
  const item = menuItems.value.find((m) => m.route === route.name)
  return item?.label || route.meta?.title || 'Docsy'
})

function onMenuSelect(index) {
  router.push({ name: index })
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
  window.localStorage.setItem('docsy.sidebar.collapsed', sidebarCollapsed.value ? '1' : '0')
}

function applySettingsEvent(event) {
  appStore.settings = { ...appStore.settings, ...(event.detail || {}) }
}

function formatElapsed(ms) {
  const seconds = Math.floor(ms / 1000)
  if (seconds < 60) return `${seconds}秒`
  const minutes = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${minutes}分${secs}秒`
}

function updateElapsedTime() {
  const oldest = Array.from(pendingOperations.values()).at(0)
  if (!oldest) {
    operationElapsed.value = ''
    return
  }
  const ms = Date.now() - oldest.startTime
  operationElapsed.value = ms > 3000 ? formatElapsed(ms) : ''
}

function startOperation(event) {
  const operationId = event.detail?.id || `unknown:${Date.now()}`
  pendingOperations.set(operationId, {
    label: event.detail?.label || 'Doclet 正在处理…',
    startTime: Date.now(),
  })
  clearTimeout(operationTimer)
  const entry = pendingOperations.get(operationId)
  operationMessage.value = entry?.label || 'Doclet 正在处理…'
  operationTimer = window.setTimeout(() => {
    operationVisible.value = true
    window.clearInterval(elapsedTimer)
    elapsedTimer = window.setInterval(updateElapsedTime, 1000)
    // Show cancel button after 30 seconds
    clearTimeout(cancelTimer)
    cancelTimer = window.setTimeout(() => {
      showCancel.value = true
    }, 30000)
  }, 350)
}

function finishOperation(event) {
  const operationId = event.detail?.id
  if (operationId) pendingOperations.delete(operationId)
  if (pendingOperations.size) {
    const entry = Array.from(pendingOperations.values()).at(-1)
    operationMessage.value = entry?.label || 'Doclet 正在处理…'
    return
  }
  clearTimeout(operationTimer)
  window.clearInterval(elapsedTimer)
  clearTimeout(cancelTimer)
  operationVisible.value = false
  operationElapsed.value = ''
  showCancel.value = false
}

function updateOperation(event) {
  const label = event.detail?.label
  if (label) {
    operationMessage.value = label
  }
}

async function cancelCurrentOperation() {
  try {
    // 查询 Rust 侧活跃操作，按 ID 取消
    const activeResult = await tauriCallSafe('list_active_operations')
    if (activeResult.ok && activeResult.data?.length > 0) {
      // 取消第一个活跃操作（已运行最久的）
      const target = activeResult.data[0]
      await tauriCallSafe('cancel_operation', { operationId: target.operationId })
    }
  } catch {
    // Ignore errors — the operation may have already finished
  }
  // 清除 UI 状态（pendingOperations 是前端动画追踪，与 Rust 操作管理独立）
  pendingOperations.clear()
  clearTimeout(operationTimer)
  window.clearInterval(elapsedTimer)
  clearTimeout(cancelTimer)
  operationVisible.value = false
  operationElapsed.value = ''
  showCancel.value = false
}

let unlistenConversionTimeout = null
let unlistenDownloadProgress = null
let unlistenOperationProgress = null
let unlistenCloseRequested = null
let closeConfirmationOpen = false

async function handleCloseRequested(event) {
  if (closeConfirmationOpen) return
  closeConfirmationOpen = true
  try {
    let backendOperations = []
    try {
      backendOperations = await listActiveOperations()
    } catch {
      // The native event payload still covers registered subprocesses when the
      // operation list cannot be queried during shutdown.
    }
    const hasRunningWork =
      pendingOperations.size > 0 || backendOperations.length > 0 || Boolean(event.payload?.backendActive)

    if (hasRunningWork) {
      await ElMessageBox.confirm(
        '当前仍有文件正在处理。中止并关闭可能导致尚未完成的输出文件不可用，是否继续关闭？',
        '处理尚未完成',
        {
          confirmButtonText: '中止并关闭',
          cancelButtonText: '继续处理',
          type: 'warning',
          closeOnClickModal: false,
          closeOnPressEscape: false,
        },
      )
    }

    await confirmAppClose()
  } catch {
    // Cancelling the dialog intentionally keeps both the window and the
    // running operation alive.
  } finally {
    closeConfirmationOpen = false
  }
}

onMounted(() => {
  // Platform detection for OS-specific CSS (backdrop-filter on macOS only)
  document.documentElement.dataset.os = /mac/i.test(navigator.platform || navigator.userAgent) ? 'macos' : 'windows'

  appStore.loadSettings()
  window.addEventListener('docsy-settings-updated', applySettingsEvent)
  window.addEventListener('docsy-operation-start', startOperation)
  window.addEventListener('docsy-operation-finish', finishOperation)
  window.addEventListener('docsy-operation-update', updateOperation)

  // Listen for conversion timeout events from the backend
  listen('docsy-conversion-timeout', async () => {
    try {
      await ElMessageBox.confirm('文档转换耗时较长，可能是大文件或 Office 响应慢。是否继续等待？', '转换超时', {
        confirmButtonText: '继续等待',
        cancelButtonText: '取消转换',
        type: 'warning',
      })
      // User chose to continue
      await tauriCallSafe('respond_conversion_timeout', { continueWaiting: true })
    } catch {
      // User chose to cancel
      await tauriCallSafe('respond_conversion_timeout', { continueWaiting: false })
    }
  }).then((unlisten) => {
    unlistenConversionTimeout = unlisten
  })

  // Listen for tool download progress events
  listen('docsy-tool-download-progress', (event) => {
    const { bytes, elapsed_ms } = event.payload || {}
    if (!bytes) return
    const mb = (bytes / (1024 * 1024)).toFixed(1)
    const speed = elapsed_ms > 0 ? (bytes / 1024 / (elapsed_ms / 1000)).toFixed(0) : '?'
    operationMessage.value = `正在下载工具… ${mb} MB (${speed} KB/s)`
  }).then((unlisten) => {
    unlistenDownloadProgress = unlisten
  })

  // Long PDF compression reports phases from Rust without changing the
  // operation lifetime. Keep the global Doclet panel informative while a
  // large file is being analyzed or written.
  listen('docsy-operation-progress', (event) => {
    const label = event.payload?.label
    if (label) operationMessage.value = label
  }).then((unlisten) => {
    unlistenOperationProgress = unlisten
  })

  listen('docsy-close-requested', handleCloseRequested).then((unlisten) => {
    unlistenCloseRequested = unlisten
  })
})

onBeforeUnmount(() => {
  clearTimeout(operationTimer)
  window.clearInterval(elapsedTimer)
  clearTimeout(cancelTimer)
  window.removeEventListener('docsy-settings-updated', applySettingsEvent)
  window.removeEventListener('docsy-operation-start', startOperation)
  window.removeEventListener('docsy-operation-finish', finishOperation)
  window.removeEventListener('docsy-operation-update', updateOperation)
  if (unlistenConversionTimeout) unlistenConversionTimeout()
  if (unlistenDownloadProgress) unlistenDownloadProgress()
  if (unlistenOperationProgress) unlistenOperationProgress()
  if (unlistenCloseRequested) unlistenCloseRequested()
})
</script>

<style scoped>
.app-container {
  --docsy-sidebar-width: clamp(212px, 18vw, 236px);
  --docsy-shell-header-height: clamp(62px, 8.4dvh, 76px);
  --docsy-brand-height: clamp(64px, 8.4dvh, 76px);
  --docsy-menu-item-height: clamp(42px, 5.1dvh, 46px);
  --docsy-menu-item-gap: clamp(2px, 0.35dvh, 3px);
  --docsy-footer-height: clamp(46px, 5.8dvh, 52px);
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
  background: var(--docsy-canvas);
}

.app-container > .el-container {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.app-aside {
  position: relative;
  display: flex;
  flex-direction: column;
  color: rgba(255, 255, 255, 0.72);
  background: var(--docsy-sidebar);
  border-right: 1px solid rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.app-aside.is-collapsed .brand {
  justify-content: center;
  padding-inline: 10px;
}

.app-aside.is-collapsed .brand-copy,
.app-aside.is-collapsed .sidebar-section-label,
.app-aside.is-collapsed .menu-label,
.app-aside.is-collapsed .menu-arrow,
.app-aside.is-collapsed .sidebar-footer .footer-btn span {
  display: none;
}

.app-aside.is-collapsed .sidebar-menu {
  padding-block: clamp(10px, 1.8dvh, 14px);
  padding-inline: 10px;
}

.app-aside.is-collapsed .sidebar-menu :deep(.el-menu-item) {
  grid-template-columns: 1fr;
  justify-items: center;
  padding: 0 !important;
}

.app-aside.is-collapsed .menu-index {
  display: none;
}

.app-aside.is-collapsed .menu-icon {
  display: block;
}

.app-aside.is-collapsed .sidebar-footer .footer-btn {
  gap: 0;
}

.app-aside::after {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  content: '';
  opacity: 0.14;
  background-image: linear-gradient(rgba(255, 255, 255, 0.08) 1px, transparent 1px);
  background-size: 100% 32px;
  mask-image: linear-gradient(to bottom, black, transparent 72%);
}

.brand {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  min-height: var(--docsy-brand-height);
  width: 100%;
  padding-block: clamp(13px, 2.1dvh, 17px);
  padding-inline: 18px;
  color: inherit;
  cursor: pointer;
  gap: 12px;
  background: transparent;
  border: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.13);
  text-align: left;
}

.brand {
  transition: background-color 150ms var(--ease-out);
}

.brand:focus-visible {
  outline-color: rgba(255, 255, 255, 0.72);
  outline-offset: -4px;
}

.brand-logo {
  width: 38px;
  height: 38px;
  object-fit: contain;
  flex: 0 0 38px;
  filter: drop-shadow(0 5px 12px rgba(0, 0, 0, 0.15));
  transition: transform 260ms var(--ease-out);
}

.brand-copy {
  display: grid;
  gap: 1px;
}

.brand-name {
  color: #fffdf8;
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: 19px;
  font-weight: 760;
  letter-spacing: 0.01em;
}

.brand-caption {
  color: rgba(255, 255, 255, 0.44);
  font-size: 9px;
  font-weight: 650;
  letter-spacing: 0.13em;
  text-transform: uppercase;
}

.sidebar-section-label {
  position: relative;
  z-index: 1;
  padding-block: clamp(14px, 2.5dvh, 23px) clamp(6px, 1dvh, 8px);
  padding-inline: 20px;
  color: rgba(255, 255, 255, 0.35);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.16em;
}

.sidebar-menu {
  position: relative;
  z-index: 1;
  flex: 1;
  border-right: none;
  overflow-y: auto;
  padding: 0 10px;
  background: transparent;
}

.sidebar-menu :deep(.el-menu-item) {
  display: grid;
  grid-template-columns: 30px 1fr auto;
  height: var(--docsy-menu-item-height);
  margin-block: var(--docsy-menu-item-gap);
  padding: 0 12px !important;
  border-radius: var(--docsy-radius);
  color: rgba(255, 255, 255, 0.66);
  line-height: var(--docsy-menu-item-height);
}

.sidebar-menu :deep(.el-menu-item:hover) {
  color: #fff;
  background: rgba(255, 255, 255, 0.07);
}

.sidebar-menu :deep(.el-menu-item.is-active) {
  color: #fff;
  background: rgba(255, 255, 255, 0.11);
  font-weight: 600;
}

.sidebar-menu :deep(.el-menu-item.is-active::before) {
  position: absolute;
  top: clamp(8px, 1.1dvh, 10px);
  bottom: clamp(8px, 1.1dvh, 10px);
  left: -10px;
  width: 3px;
  content: '';
  background: var(--docsy-accent-light);
  border-radius: 0 3px 3px 0;
}

.menu-index {
  color: rgba(255, 255, 255, 0.34);
  font-family: ui-monospace, 'SFMono-Regular', Menlo, monospace;
  font-size: 10px;
}

.menu-icon {
  display: none;
  width: 27px;
  height: 27px;
  background: currentColor;
  mask-image: var(--menu-icon-url);
  mask-position: center;
  mask-repeat: no-repeat;
  mask-size: contain;
  -webkit-mask-image: var(--menu-icon-url);
  -webkit-mask-position: center;
  -webkit-mask-repeat: no-repeat;
  -webkit-mask-size: contain;
}

.sidebar-menu :deep(.el-menu-item.is-active) .menu-index {
  color: var(--docsy-accent-light);
}

.menu-arrow {
  color: rgba(255, 255, 255, 0.3);
  font-size: 15px;
  opacity: 0;
  transform: translateX(-3px);
  transition:
    opacity 150ms,
    transform 150ms var(--ease-out);
}

.sidebar-menu :deep(.el-menu-item:hover) .menu-arrow,
.sidebar-menu :deep(.el-menu-item.is-active) .menu-arrow {
  opacity: 1;
  transform: translateX(0);
}

.sidebar-footer {
  position: relative;
  z-index: 1;
  display: flex;
  border-top: 1px solid rgba(255, 255, 255, 0.13);
  background: transparent;
}

.sidebar-footer .footer-btn {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  height: var(--docsy-footer-height);
  margin: 0;
  gap: 7px;
  color: rgba(255, 255, 255, 0.48);
  background: transparent;
  border: 0;
  border-radius: 0;
  font-size: 12px;
}

.sidebar-footer .footer-btn + .footer-btn {
  border-left: 1px solid rgba(255, 255, 255, 0.13);
}

.sidebar-footer .footer-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.06);
}

.footer-btn.active {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--docsy-shell-header-height);
  padding: 0 30px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: rgba(251, 247, 239, 0.94);
}

.header-leading {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 12px;
}

.sidebar-toggle {
  display: grid;
  flex: 0 0 30px;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 8px;
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-elevated);
  cursor: pointer;
  font-size: 22px;
  line-height: 1;
  place-items: center;
}

.sidebar-toggle:hover {
  color: var(--docsy-primary);
  border-color: color-mix(in srgb, var(--docsy-primary) 42%, var(--docsy-border-subtle));
}

.page-heading {
  display: flex;
  align-items: baseline;
  gap: 11px;
}

.page-title {
  font-family:
    ui-rounded,
    'SF Pro Rounded',
    -apple-system,
    'PingFang SC',
    sans-serif;
  font-size: 15px;
  font-weight: 720;
  color: var(--docsy-text-strong);
}

.page-context {
  color: var(--docsy-text-muted);
  font-size: 11px;
}

.app-version {
  padding: 5px 8px;
  color: var(--docsy-text-muted);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: 999px;
  font-family: ui-monospace, 'SFMono-Regular', Menlo, monospace;
  font-size: 10px;
}

.app-main {
  padding: 0;
  background: var(--docsy-canvas);
  background-image:
    linear-gradient(rgba(79, 65, 48, 0.035) 1px, transparent 1px),
    linear-gradient(90deg, rgba(79, 65, 48, 0.035) 1px, transparent 1px);
  background-size: 28px 28px;
  min-height: 0;
  overflow-y: auto;
}

.doclet-operation-panel {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 999;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
}

.doclet-cancel-btn {
  padding: 4px 12px;
  font-size: 12px;
  color: var(--docsy-text-muted);
  background: var(--docsy-surface-elevated);
  border: 1px solid var(--docsy-border-subtle);
  border-radius: var(--docsy-radius);
  cursor: pointer;
  transition:
    color 0.15s,
    border-color 0.15s;
}

.doclet-cancel-btn:hover {
  color: var(--docsy-danger);
  border-color: var(--docsy-danger);
}

.doclet-operation-enter-active,
.doclet-operation-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}

.doclet-operation-enter-from,
.doclet-operation-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

@media (max-width: 900px) {
  .app-aside {
    width: 78px !important;
  }

  .brand {
    justify-content: center;
    padding-inline: 10px;
  }

  .brand-copy,
  .sidebar-section-label,
  .menu-label,
  .menu-arrow,
  .sidebar-footer .footer-btn span {
    display: none;
  }

  .sidebar-menu {
    padding-block: clamp(10px, 1.8dvh, 14px);
    padding-inline: 10px;
  }

  .sidebar-menu :deep(.el-menu-item) {
    grid-template-columns: 1fr;
    justify-items: center;
    padding: 0 !important;
  }

  .menu-index {
    display: none;
  }

  .menu-icon {
    display: block;
  }

  .sidebar-footer .footer-btn {
    gap: 0;
  }

  .app-header {
    padding: 0 18px;
  }

  .page-context {
    display: none;
  }
}

@media (hover: hover) and (pointer: fine) {
  .brand:hover {
    background: rgba(255, 255, 255, 0.055);
  }

  .brand:hover .brand-logo {
    transform: translateY(-2px) rotate(-5deg);
  }
}
</style>
