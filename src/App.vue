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
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { getMenuItems } from './core/moduleRegistry.js'

const router = useRouter()
const route = useRoute()

const menuItems = getMenuItems()

const activeMenu = computed(() => route.name || 'home')

const currentPageTitle = computed(() => {
  const item = menuItems.find(m => m.route === route.name)
  return item?.label || 'Docsy'
})

function onMenuSelect(index) {
  router.push({ name: index })
}
</script>

<style scoped>
.app-container {
  height: 100vh;
}

.app-aside {
  background: #f5f7fa;
  border-right: 1px solid #e4e7ed;
  overflow-y: auto;
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
  border-right: none;
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
