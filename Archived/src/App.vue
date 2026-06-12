<template>
  <el-container class="docsy-shell">
    <el-aside width="200px" class="docsy-aside">
      <div class="docsy-brand">
        <img class="docsy-logo" src="./assets/docsy-logo.png" alt="" />
        <span class="docsy-title">Docsy</span>
      </div>
      <el-menu
        :default-active="activeTab"
        class="docsy-menu"
        @select="onSelect"
      >
        <el-menu-item index="home">首页</el-menu-item>
        <el-menu-item v-if="isMenuVisible('imagePaddler')" index="imagePaddler">
          图片排版
        </el-menu-item>
        <el-menu-item v-if="isMenuVisible('videoExtract')" index="videoExtract">
          视频抽帧
        </el-menu-item>
        <el-menu-item v-if="isMenuVisible('pdf')" index="pdf">
          PDF 工具
        </el-menu-item>
        <el-menu-item v-if="isMenuVisible('settings')" index="settings">
          设置
        </el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="docsy-header">
        <span class="docsy-header-title">{{ tabTitle }}</span>
      </el-header>
      <el-main class="docsy-main">
        <component
          :is="currentView"
          :sub-tab="subTab"
          :key="viewKey"
          @navigate="navigate"
          @settings-changed="refreshSettings"
        />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup>
import { computed, onMounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import HomeView from "./views/HomeView.vue";
import ImagePaddlerView from "./views/ImagePaddlerView.vue";
import PdfToolsView from "./views/PdfToolsView.vue";
import SettingsView from "./views/SettingsView.vue";
import VideoExtractView from "./views/VideoExtractView.vue";
import { logInfo, logWarn } from "./services/appLogger.js";

const activeTab = ref("home");
const subTab = ref(null);
const menuVisibility = ref({});
const currentView = shallowRef(HomeView);
const viewKey = ref("home");

const tabs = {
  home: { title: "首页", view: HomeView },
  imagePaddler: { title: "图片排版", view: ImagePaddlerView },
  videoExtract: { title: "视频抽帧", view: VideoExtractView },
  pdf: { title: "PDF 工具", view: PdfToolsView },
  settings: { title: "设置", view: SettingsView },
};

const tabTitle = computed(() => tabs[activeTab.value]?.title ?? "");

onMounted(refreshSettings);

function isMenuVisible(key) {
  return menuVisibility.value[key] !== false;
}

async function refreshSettings() {
  try {
    const s = await invoke("get_app_settings");
    menuVisibility.value = s.menu_visibility || {};
  } catch (err) {
    logWarn("app.state", "refresh_settings.failed", { error: err });
    menuVisibility.value = {};
  }
}

function onSelect(key) {
  logInfo("app.navigation", "select", { key });
  if (!tabs[key]) return;
  activeTab.value = key;
  subTab.value = null;
  currentView.value = tabs[key].view;
  viewKey.value = key;
}

function navigate(target) {
  logInfo("app.navigation", "navigate", { target });
  const [tab, sub = null] = String(target).split(":");
  if (!tabs[tab]) return;
  activeTab.value = tab;
  subTab.value = sub;
  currentView.value = tabs[tab].view;
  viewKey.value = tab;
}
</script>

<style scoped>
.docsy-shell {
  min-height: 100vh;
}
.docsy-aside {
  background: #ffffff;
  border-right: 1px solid #e5e7eb;
}
.docsy-brand {
  height: 56px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 16px;
  border-bottom: 1px solid #e5e7eb;
}
.docsy-logo {
  width: 30px;
  height: 30px;
  object-fit: contain;
}
.docsy-title {
  font-weight: 700;
  color: #111827;
}
.docsy-menu {
  border-right: 0;
}
.docsy-header {
  height: 56px;
  display: flex;
  align-items: center;
  background: #ffffff;
  border-bottom: 1px solid #e5e7eb;
}
.docsy-header-title {
  font-size: 16px;
  font-weight: 600;
}
.docsy-main {
  background: #f9fafb;
  min-height: calc(100vh - 56px);
}
</style>
