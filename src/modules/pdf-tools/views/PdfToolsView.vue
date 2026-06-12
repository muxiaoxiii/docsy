<template>
  <div class="pdf-tools-view">
    <el-tabs v-model="activeTab" tab-position="left" class="pdf-tabs">
      <el-tab-pane label="解锁" name="unlock">
        <div class="tab-content">
          <h3>PDF 解锁</h3>
          <p class="hint">拖入加密的 PDF 文件进行解锁</p>
          <el-button type="primary" @click="handleUnlock">选择文件</el-button>
        </div>
      </el-tab-pane>
      <el-tab-pane label="合并" name="merge">
        <div class="tab-content">
          <h3>PDF 合并</h3>
          <p class="hint">将多个 PDF 合并为一个文件</p>
        </div>
      </el-tab-pane>
      <el-tab-pane label="证据整理" name="evidence">
        <div class="tab-content">
          <h3>证据整理</h3>
          <p class="hint">扫描文件夹，自动分组合并 PDF</p>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { tauriCall } from '../../core/tauriBridge.js'

const activeTab = ref('unlock')

async function handleUnlock() {
  // TODO: implement file selection and unlock
}
</script>

<style scoped>
.pdf-tools-view {
  height: 100%;
}

.pdf-tabs {
  height: 100%;
}

.tab-content {
  padding: 20px;
}

.tab-content h3 {
  margin-bottom: 8px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin-bottom: 16px;
}
</style>
