<template>
  <el-container class="app-container">
    <el-aside width="220px" class="app-aside">
      <div class="brand" @click="router.push('/')">
        <img src="./assets/docsy-logo.png" alt="Docsy" class="brand-logo" />
        <span class="brand-name">Docsy</span>
      </div>
      <el-menu :default-active="activeMenu" @select="onMenuSelect" class="sidebar-menu">
        <template v-for="item in menuItems" :key="item.route">
          <el-menu-item :index="item.route">
            <el-icon v-if="item.icon"><component :is="item.icon" /></el-icon>
            <span>{{ item.label }}</span>
          </el-menu-item>
        </template>
      </el-menu>
      <div class="sidebar-footer">
        <el-tooltip content="设置" placement="right">
          <el-button
            class="settings-shortcut"
            :class="{ active: route.name === 'settings' }"
            circle
            @click="router.push({ name: 'settings' })"
          >
            <el-icon><Setting /></el-icon>
          </el-button>
        </el-tooltip>
      </div>
    </el-aside>
    <el-container>
      <el-header class="app-header">
        <span class="page-title">{{ currentPageTitle }}</span>
      </el-header>
      <el-main class="app-main">
        <router-view />
      </el-main>
      <Transition name="doclet-operation">
        <div v-if="operationVisible" class="doclet-operation-panel">
          <DocletWorkingPet :message="operationMessage" />
        </div>
      </Transition>
    </el-container>
  </el-container>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Setting } from '@element-plus/icons-vue'
import { getMenuItems } from './core/moduleRegistry.js'
import { tauriCallSafe } from './core/tauriBridge.js'
import { listen } from '@tauri-apps/api/event'
import { ElMessageBox } from 'element-plus'
import DocletWorkingPet from './shared/components/DocletWorkingPet.vue'

const router = useRouter()
const route = useRoute()

const settings = ref({
  menu_visibility: {},
  menu_order: [],
})
const menuItems = computed(() => getMenuItems(settings.value))

const activeMenu = computed(() => route.name || 'home')
const operationVisible = ref(false)
const operationMessage = ref('Doclet 正在处理…')
let operationTimer
const pendingOperations = new Map()

const currentPageTitle = computed(() => {
  const item = menuItems.value.find((m) => m.route === route.name)
  return item?.label || route.meta?.title || 'Docsy'
})

function onMenuSelect(index) {
  router.push({ name: index })
}

async function loadSettings() {
  const result = await tauriCallSafe('get_app_settings')
  if (result.ok) {
    settings.value = { ...settings.value, ...result.data }
  }
}

function applySettingsEvent(event) {
  settings.value = { ...settings.value, ...(event.detail || {}) }
}

function startOperation(event) {
  const operationId = event.detail?.id || `unknown:${Date.now()}`
  pendingOperations.set(operationId, event.detail?.label || 'Doclet 正在处理…')
  clearTimeout(operationTimer)
  operationMessage.value = pendingOperations.get(operationId) || 'Doclet 正在处理…'
  operationTimer = window.setTimeout(() => {
    operationVisible.value = true
  }, 350)
}

function finishOperation(event) {
  const operationId = event.detail?.id
  if (operationId) pendingOperations.delete(operationId)
  if (pendingOperations.size) {
    operationMessage.value = Array.from(pendingOperations.values()).at(-1) || 'Doclet 正在处理…'
    return
  }
  clearTimeout(operationTimer)
  operationVisible.value = false
}

let unlistenConversionTimeout = null

onMounted(() => {
  loadSettings()
  window.addEventListener('docsy-settings-updated', applySettingsEvent)
  window.addEventListener('docsy-operation-start', startOperation)
  window.addEventListener('docsy-operation-finish', finishOperation)

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
})

onBeforeUnmount(() => {
  clearTimeout(operationTimer)
  window.removeEventListener('docsy-settings-updated', applySettingsEvent)
  window.removeEventListener('docsy-operation-start', startOperation)
  window.removeEventListener('docsy-operation-finish', finishOperation)
  if (unlistenConversionTimeout) unlistenConversionTimeout()
})
</script>

<style scoped>
.app-container {
  height: 100vh;
  overflow: hidden;
  background: var(--docsy-canvas);
}

.app-container > .el-container {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.app-aside {
  display: flex;
  flex-direction: column;
  background: var(--docsy-sidebar);
  border-right: 1px solid var(--docsy-border-subtle);
  overflow: hidden;
}

.brand {
  display: flex;
  align-items: center;
  min-height: 60px;
  padding: 12px 16px;
  cursor: pointer;
  gap: 10px;
  border-bottom: 1px solid var(--docsy-border-subtle);
}

.brand:hover {
  opacity: 0.8;
  transition: opacity 0.15s ease;
}

.brand-logo {
  width: 34px;
  height: 34px;
  object-fit: contain;
  flex: 0 0 34px;
}

.brand-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--docsy-text-strong);
}

.sidebar-menu {
  flex: 1;
  border-right: none;
  overflow-y: auto;
  padding: 10px 8px;
  background: transparent;
}

.sidebar-menu :deep(.el-menu-item) {
  height: 40px;
  margin: 3px 0;
  border-radius: 6px;
  color: var(--docsy-text);
}

.sidebar-menu :deep(.el-menu-item:hover) {
  background: var(--docsy-sidebar-hover);
}

.sidebar-menu :deep(.el-menu-item.is-active) {
  color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
  font-weight: 600;
}

.sidebar-footer {
  display: flex;
  justify-content: center;
  padding: 12px 0 16px;
  border-top: 1px solid var(--docsy-border-subtle);
  background: rgba(255, 253, 250, 0.42);
}

.settings-shortcut.active {
  color: var(--docsy-primary);
  border-color: var(--docsy-primary);
  background: var(--docsy-primary-soft);
}

.app-header {
  display: flex;
  align-items: center;
  height: 60px;
  padding: 0 20px;
  border-bottom: 1px solid var(--docsy-border-subtle);
  background: var(--docsy-surface);
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--docsy-text-strong);
}

.app-main {
  padding: 0;
  background: var(--docsy-canvas);
  min-height: 0;
  overflow-y: auto;
}

.doclet-operation-panel {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 999;
  pointer-events: none;
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
</style>
