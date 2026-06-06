<template>
  <el-container class="pdf-tools">
    <el-aside width="160px" class="pdf-aside">
      <el-menu :default-active="active" @select="active = $event">
        <el-menu-item index="unlock">解锁</el-menu-item>
        <el-menu-item index="merge" disabled>合并（待开发）</el-menu-item>
        <el-menu-item index="split" disabled>拆分（待开发）</el-menu-item>
      </el-menu>
    </el-aside>
    <el-main class="pdf-main">
      <PdfUnlock v-if="active === 'unlock'" />
    </el-main>
  </el-container>
</template>

<script setup>
import { ref, watch } from "vue";
import PdfUnlock from "./PdfUnlock.vue";

const props = defineProps({
  subTab: { type: String, default: null },
});
const active = ref("unlock");

watch(
  () => props.subTab,
  (v) => {
    if (v) active.value = v;
  },
  { immediate: true }
);
</script>

<style scoped>
.pdf-tools {
  height: 100%;
  background: #ffffff;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid #e5e7eb;
}
.pdf-aside {
  border-right: 1px solid #e5e7eb;
}
.pdf-main {
  padding: 24px;
}
</style>
