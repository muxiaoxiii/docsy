<template>
  <el-container class="app-container">
    <el-aside width="220px" class="app-aside">
      <div class="brand" @click="router.push('/')">
        <img src="./assets/docsy-logo.png" alt="Docsy" class="brand-logo" />
        <span class="brand-name">Docsy</span>
      </div>
      <el-menu
        :default-active="activeMenu"
        @select="onMenuSelect"
        class="sidebar-menu"
      >
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
    </el-container>
  </el-container>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { Setting } from '@element-plus/icons-vue'
import { getMenuItems } from './core/moduleRegistry.js'
import { tauriCallSafe } from './core/tauriBridge.js'

const router = useRouter()
const route = useRoute()

const settings = ref({
  menu_visibility: {},
  menu_order: [],
})
const menuItems = computed(() => getMenuItems(settings.value))

const activeMenu = computed(() => route.name || 'home')

const currentPageTitle = computed(() => {
  const item = menuItems.value.find(m => m.route === route.name)
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

onMounted(() => {
  loadSettings()
  window.addEventListener('docsy-settings-updated', applySettingsEvent)
})

onBeforeUnmount(() => {
  window.removeEventListener('docsy-settings-updated', applySettingsEvent)
})
</script>

<style scoped>
.app-container {
  height: 100vh;
}

.app-aside {
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
  border-right: 1px solid #e4e7ed;
  overflow: hidden;
}

.brand {
  display: flex;
  align-items: center;
  padding: 16px;
  cursor: pointer;
  gap: 8px;
}

.brand-logo {
  width: 32px;
  height: 32px;
}

.brand-name {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
}

.sidebar-menu {
  flex: 1;
  border-right: none;
  overflow-y: auto;
}

.sidebar-footer {
  display: flex;
  justify-content: center;
  padding: 12px 0 16px;
  border-top: 1px solid #e4e7ed;
}

.settings-shortcut.active {
  color: var(--el-color-primary);
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}

.app-header {
  display: flex;
  align-items: center;
  border-bottom: 1px solid #e4e7ed;
  background: #fff;
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.app-main {
  background: #fff;
  overflow-y: auto;
}
</style>
