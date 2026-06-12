<template>
  <div class="pdf-tools-view">
    <el-tabs v-model="activeTab" tab-position="left" class="pdf-tabs">
      <!-- Unlock Tab -->
      <el-tab-pane label="解锁" name="unlock">
        <div class="tab-content">
          <h3>PDF 解锁</h3>
          <p class="hint">移除 PDF 文件的密码保护</p>

          <el-button type="primary" @click="selectUnlockFiles">
            选择 PDF 文件
          </el-button>

          <div class="file-list" v-if="unlockFiles.length">
            <div v-for="(file, idx) in unlockFiles" :key="idx" class="file-item">
              <span class="file-name">{{ file.name }}</span>
              <el-tag :type="file.statusType" size="small">{{ file.statusText }}</el-tag>
            </div>
            <el-button type="success" @click="batchUnlock" :loading="unlocking" :disabled="!unlockFiles.length">
              批量解锁
            </el-button>
          </div>
        </div>
      </el-tab-pane>

      <!-- Merge Tab -->
      <el-tab-pane label="合并" name="merge">
        <div class="tab-content">
          <h3>PDF 合并</h3>
          <p class="hint">将多个 PDF 合并为一个文件</p>

          <el-button @click="selectMergeFiles">添加 PDF 文件</el-button>

          <div class="merge-list" v-if="mergeFiles.length">
            <div v-for="(file, idx) in mergeFiles" :key="idx" class="file-item">
              <el-icon class="drag-handle"><Rank /></el-icon>
              <span class="file-name">{{ file }}</span>
              <el-button text type="danger" size="small" @click="mergeFiles.splice(idx, 1)">删除</el-button>
            </div>
            <el-button type="success" @click="doMerge" :loading="merging" :disabled="mergeFiles.length < 2">
              合并为一个 PDF
            </el-button>
          </div>
        </div>
      </el-tab-pane>

      <!-- Evidence Tab -->
      <el-tab-pane label="证据整理" name="evidence">
        <div class="tab-content">
          <h3>证据整理</h3>
          <p class="hint">扫描文件夹，按子文件夹自动分组合并 PDF</p>

          <el-button type="primary" @click="selectEvidenceFolder">
            选择证据文件夹
          </el-button>

          <div v-if="evidenceFolder" class="evidence-info">
            <p>文件夹: {{ evidenceFolder }}</p>
            <el-button @click="scanEvidence" :loading="scanning">扫描</el-button>
          </div>

          <div v-if="evidenceGroups.length" class="evidence-groups">
            <div v-for="group in evidenceGroups" :key="group.name" class="group-item">
              <h4>{{ group.name }} ({{ group.files.length }} 个文件)</h4>
              <div class="group-files">
                <span v-for="f in group.files" :key="f.path" class="group-file">{{ f.name }}</span>
              </div>
            </div>
            <el-button type="success" @click="buildEvidence" :loading="building">
              生成合并 PDF
            </el-button>
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'

const activeTab = ref('unlock')

// Unlock
const unlockFiles = ref([])
const unlocking = ref(false)

async function selectUnlockFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    unlockFiles.value = paths.map(p => ({
      path: p,
      name: p.split('/').pop(),
      statusText: '等待',
      statusType: 'info',
    }))
  }
}

async function batchUnlock() {
  unlocking.value = true
  for (const file of unlockFiles.value) {
    file.statusText = '处理中'
    file.statusType = 'warning'
    const result = await tauriCallSafe('unlock_pdf', { input: file.path })
    if (result.ok) {
      file.statusText = '成功'
      file.statusType = 'success'
    } else {
      file.statusText = result.error || '失败'
      file.statusType = 'danger'
    }
  }
  unlocking.value = false
}

// Merge
const mergeFiles = ref([])
const merging = ref(false)

async function selectMergeFiles() {
  const selected = await open({
    multiple: true,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected]
    mergeFiles.value.push(...paths)
  }
}

async function doMerge() {
  merging.value = true
  const output = await open({ directory: true })
  if (output) {
    const outputPath = `${output}/merged.pdf`
    await tauriCallSafe('merge_pdfs', { inputs: mergeFiles.value, output: outputPath })
  }
  merging.value = false
}

// Evidence
const evidenceFolder = ref('')
const evidenceGroups = ref([])
const scanning = ref(false)
const building = ref(false)

async function selectEvidenceFolder() {
  const selected = await open({ directory: true })
  if (selected) {
    evidenceFolder.value = selected
  }
}

async function scanEvidence() {
  if (!evidenceFolder.value) return
  scanning.value = true
  const result = await tauriCallSafe('scan_evidence_folder', { root: evidenceFolder.value })
  if (result.ok) {
    evidenceGroups.value = result.data.groups || []
  }
  scanning.value = false
}

async function buildEvidence() {
  building.value = true
  const result = await tauriCallSafe('build_evidence_group_pdfs', {
    root: evidenceFolder.value,
    groups: evidenceGroups.value,
  })
  building.value = false
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
  padding: 16px;
  max-width: 600px;
}

.tab-content h3 {
  margin: 0 0 6px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin: 0 0 16px;
}

.file-list, .merge-list {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.file-name {
  flex: 1;
  font-size: 13px;
}

.drag-handle {
  cursor: move;
  color: #c0c4cc;
}

.evidence-info {
  margin-top: 12px;
  font-size: 13px;
  color: #606266;
}

.evidence-groups {
  margin-top: 16px;
}

.group-item {
  margin-bottom: 12px;
  padding: 10px;
  background: #f9f9f9;
  border-radius: 4px;
}

.group-item h4 {
  margin: 0 0 6px;
  font-size: 13px;
}

.group-files {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.group-file {
  font-size: 12px;
  color: #909399;
}
</style>
