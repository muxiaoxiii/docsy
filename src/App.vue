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
        <el-menu-item
          v-if="letterEnabled && isMenuVisible('letter')"
          index="letter"
        >所函生成</el-menu-item>
        <el-menu-item
          v-for="t in userTemplates"
          v-show="isMenuVisible(`tpl:${t.id}`)"
          :key="`tpl_${t.id}`"
          :index="`tpl:${t.id}`"
          @contextmenu.prevent="onTemplateContextMenu($event, t)"
        >
          {{ t.name }}
        </el-menu-item>
        <el-menu-item v-if="isMenuVisible('template')" index="template">模板制作</el-menu-item>
        <el-menu-item v-if="isMenuVisible('manage')" index="manage">模板管理</el-menu-item>
        <el-menu-item v-if="isMenuVisible('imagePaddler')" index="imagePaddler">图片排版</el-menu-item>
        <el-menu-item v-if="isMenuVisible('videoExtract')" index="videoExtract">视频抽帧</el-menu-item>
        <el-menu-item v-if="isMenuVisible('pdf')" index="pdf">PDF 工具</el-menu-item>
        <el-menu-item v-if="isMenuVisible('records')" index="records">记录中心</el-menu-item>
        <el-menu-item v-if="isMenuVisible('settings')" index="settings">设置</el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="docsy-header">
        <span class="docsy-header-title">{{ tabTitle }}</span>
      </el-header>
      <el-main class="docsy-main">
        <keep-alive include="LetterView">
          <component
            :is="currentView"
            :sub-tab="subTab"
            :template-id="currentTemplateId"
            :edit-template-id="currentEditTemplateId"
            :key="viewKey"
            @navigate="navigate"
            @templates-changed="refreshAll"
          />
        </keep-alive>
      </el-main>
    </el-container>
  </el-container>

  <!-- 右键菜单 -->
  <div
    v-if="contextMenu.visible"
    class="context-menu"
    :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    @click.stop
  >
    <div class="context-menu-item" @click="renameTemplate">重命名</div>
    <div class="context-menu-item context-menu-danger" @click="deleteTemplate">删除</div>
  </div>
  <!-- 点击其他地方关闭菜单 -->
  <div
    v-if="contextMenu.visible"
    class="context-menu-overlay"
    @click="contextMenu.visible = false"
  />
</template>

<script setup>
import { computed, onMounted, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage, ElMessageBox } from "element-plus";
import HomeView from "./views/HomeView.vue";
import ImagePaddlerView from "./views/ImagePaddlerView.vue";
import LetterView from "./views/LetterView.vue";
import ManageView from "./views/ManageView.vue";
import PdfToolsView from "./views/PdfToolsView.vue";
import PlaceholderView from "./views/PlaceholderView.vue";
import RecordsView from "./views/RecordsView.vue";
import SettingsView from "./views/SettingsView.vue";
import TemplateView from "./views/TemplateView.vue";
import VideoExtractView from "./views/VideoExtractView.vue";
import { logError, logInfo, logWarn } from "./services/appLogger.js";

const activeTab = ref("home");
const subTab = ref(null);
const letterEnabled = ref(true);
const userTemplates = ref([]);
const currentTemplateId = ref("letter");
const menuVisibility = ref({});

const contextMenu = ref({ visible: false, x: 0, y: 0, template: null });
const currentEditTemplateId = ref(null);

function isMenuVisible(key) {
  return menuVisibility.value[key] !== false;
}

const tabs = {
  home: { title: "首页", view: HomeView },
  letter: { title: "所函生成", view: LetterView },
  template: { title: "模板制作", view: TemplateView },
  manage: { title: "模板管理", view: ManageView },
  imagePaddler: { title: "图片排版", view: ImagePaddlerView },
  videoExtract: { title: "视频抽帧", view: VideoExtractView },
  pdf: { title: "PDF 工具", view: PdfToolsView },
  records: { title: "记录中心", view: RecordsView },
  settings: { title: "设置", view: SettingsView },
};

const tabTitle = computed(() => {
  if (activeTab.value.startsWith("tpl:")) {
    const id = activeTab.value.slice(4);
    return userTemplates.value.find((t) => t.id === id)?.name || "模板生成";
  }
  return tabs[activeTab.value]?.title ?? "";
});
const currentView = shallowRef(HomeView);
const viewKey = ref("home");

onMounted(refreshAll);

async function refreshAll() {
  try {
    letterEnabled.value = await invoke("is_template_enabled", {
      templateId: "letter",
    });
  } catch (err) {
    logWarn("app.state", "refresh_letter_enabled.failed", { error: err });
    letterEnabled.value = true;
  }
  try {
    userTemplates.value = await invoke("list_user_templates");
  } catch (err) {
    logWarn("app.state", "refresh_user_templates.failed", { error: err });
    userTemplates.value = [];
  }
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
  activeTab.value = key;
  subTab.value = null;
  currentEditTemplateId.value = null;
  if (key.startsWith("tpl:")) {
    currentTemplateId.value = key.slice(4);
    currentView.value = LetterView;
    viewKey.value = key;
  } else if (key === "letter") {
    currentTemplateId.value = "letter";
    currentView.value = LetterView;
    viewKey.value = "letter";
  } else {
    currentView.value = tabs[key].view;
    viewKey.value = key;
  }
}

function navigate(target) {
  logInfo("app.navigation", "navigate", { target });
  const [tab, sub = null] = String(target).split(":");
  if (tab === "tpl") {
    onSelect(`tpl:${sub}`);
    return;
  }
  if (tab === "edit") {
    // 编辑模板：跳转到 TemplateView 并传入模板 ID
    currentEditTemplateId.value = sub;
    currentView.value = TemplateView;
    viewKey.value = `edit:${sub}`;
    return;
  }
  if (!tabs[tab]) return;
  activeTab.value = tab;
  subTab.value = sub;
  currentEditTemplateId.value = null;
  currentView.value = tabs[tab].view;
  viewKey.value = tab;
}

function onTemplateContextMenu(event, tpl) {
  contextMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    template: tpl,
  };
}

async function renameTemplate() {
  const tpl = contextMenu.value.template;
  contextMenu.value.visible = false;
  if (!tpl) return;

  try {
    const { value: newName } = await ElMessageBox.prompt(
      "输入新的模板名称",
      "重命名",
      {
        inputValue: tpl.name,
        confirmButtonText: "确定",
        cancelButtonText: "取消",
        inputValidator: (v) => (v?.trim() ? true : "名称不能为空"),
      }
    );
    if (!newName || newName.trim() === tpl.name) return;

    await invoke("rename_user_template", { id: tpl.id, newName: newName.trim() });
    logInfo("template.manage", "rename.success", { id: tpl.id });
    await refreshAll();
    ElMessage.success("已重命名");
  } catch (err) {
    if (err && err !== "cancel") {
      logError("template.manage", "rename.failed", { id: tpl.id, error: err });
      ElMessage.error(`重命名失败：${err}`);
    }
  }
}

async function deleteTemplate() {
  const tpl = contextMenu.value.template;
  contextMenu.value.visible = false;
  if (!tpl) return;

  try {
    await ElMessageBox.confirm(
      `确定永久删除模板「${tpl.name}」吗？此操作无法恢复。`,
      "删除模板",
      { type: "warning", confirmButtonText: "删除", cancelButtonText: "取消" }
    );
    await invoke("delete_user_template", { id: tpl.id });
    logInfo("template.manage", "delete.success", { id: tpl.id });
    await refreshAll();
    if (activeTab.value === `tpl:${tpl.id}`) {
      onSelect("home");
    }
    ElMessage.success("已删除");
  } catch (err) {
    if (err && err !== "cancel") {
      logError("template.manage", "delete.failed", { id: tpl.id, error: err });
    }
  }
}


</script>

<style scoped>
.docsy-shell {
  height: 100vh;
}
.docsy-aside {
  background: #ffffff;
  border-right: 1px solid #e5e7eb;
  display: flex;
  flex-direction: column;
}
.docsy-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 18px;
  font-size: 18px;
  font-weight: 600;
  border-bottom: 1px solid #e5e7eb;
}
.docsy-logo {
  width: 28px;
  height: 28px;
  object-fit: contain;
  flex: 0 0 auto;
}
.docsy-title {
  color: #4f8cff;
}
.docsy-menu {
  border-right: none;
  flex: 1;
}
.docsy-header {
  background: #ffffff;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  align-items: center;
}
.docsy-header-title {
  font-size: 16px;
  font-weight: 600;
}
.docsy-main {
  background: #f7f8fa;
}
</style>

<style>
/* 全局样式：右键菜单（不能 scoped） */
.context-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
}
.context-menu {
  position: fixed;
  z-index: 10000;
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  padding: 4px 0;
  min-width: 120px;
}
.context-menu-item {
  padding: 8px 16px;
  font-size: 13px;
  color: #1f2937;
  cursor: pointer;
  transition: background 0.1s;
}
.context-menu-item:hover {
  background: #f3f4f6;
}
.context-menu-danger {
  color: #ef4444;
}
.context-menu-danger:hover {
  background: #fef2f2;
}
</style>
