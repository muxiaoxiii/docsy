<template>
  <div class="image-paddler-view">
    <div class="paddler-layout">
      <!-- Settings Panel -->
      <div class="settings-panel">
        <h3>图片排版</h3>
        <p class="hint">将图片批量排版为 A4 文档</p>

        <el-form label-width="80px" size="small">
          <el-form-item label="文件夹">
            <el-button @click="selectFolder">选择文件夹</el-button>
            <span class="folder-path" v-if="folder">{{ folder }}</span>
          </el-form-item>

          <el-form-item label="输出格式">
            <el-select v-model="settings.output_format">
              <el-option label="DOCX" value="docx" />
              <el-option label="PDF" value="pdf" />
            </el-select>
          </el-form-item>

          <el-form-item label="每页张数">
            <el-select v-model="settings.layout">
              <el-option label="1 张" value="1" />
              <el-option label="2 张" value="2" />
              <el-option label="4 张" value="4" />
              <el-option label="自定义" value="custom" />
            </el-select>
          </el-form-item>

          <el-form-item label="缩放模式">
            <el-select v-model="settings.scale_mode">
              <el-option label="适应页面" value="fit" />
              <el-option label="填满裁切" value="fill" />
              <el-option label="原始大小" value="original" />
            </el-select>
          </el-form-item>

          <el-form-item label="方向">
            <el-select v-model="settings.orientation">
              <el-option label="自动" value="auto" />
              <el-option label="竖向" value="portrait" />
              <el-option label="横向" value="landscape" />
            </el-select>
          </el-form-item>

          <el-form-item>
            <el-button type="primary" @click="analyze" :loading="analyzing" :disabled="!folder">
              分析
            </el-button>
            <el-button type="success" @click="run" :loading="generating" :disabled="!analysis">
              生成文档
            </el-button>
          </el-form-item>
        </el-form>
      </div>

      <!-- Analysis Result -->
      <div class="result-panel">
        <template v-if="analysis">
          <div class="analysis-summary">
            <el-descriptions :column="2" border size="small">
              <el-descriptions-item label="图片数量">{{ analysis.images.length }}</el-descriptions-item>
              <el-descriptions-item label="分组数">{{ analysis.groups.length }}</el-descriptions-item>
              <el-descriptions-item label="推荐方向">{{ analysis.recommended.orientation }}</el-descriptions-item>
              <el-descriptions-item label="推荐布局">{{ analysis.recommended.layout }}</el-descriptions-item>
            </el-descriptions>
          </div>

          <!-- Groups -->
          <div v-if="analysis.groups.length > 1" class="groups-section">
            <h4>文件分组</h4>
            <div v-for="group in analysis.groups" :key="group.prefix" class="group-item">
              <span>{{ group.prefix }}</span>
              <el-tag size="small">{{ group.count }} 张</el-tag>
            </div>
          </div>

          <!-- Image List -->
          <div class="image-list">
            <h4>图片列表 ({{ analysis.images.length }})</h4>
            <div class="image-grid">
              <div v-for="(img, idx) in analysis.images.slice(0, 50)" :key="idx" class="image-thumb">
                <div class="thumb-placeholder">{{ idx + 1 }}</div>
                <span class="thumb-name">{{ img.path.split('/').pop() }}</span>
                <span class="thumb-size">{{ img.width }}×{{ img.height }}</span>
              </div>
              <div v-if="analysis.images.length > 50" class="more-images">
                +{{ analysis.images.length - 50 }} 更多
              </div>
            </div>
          </div>
        </template>
        <el-empty v-else description="选择文件夹后点击分析" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'

const folder = ref('')
const analyzing = ref(false)
const generating = ref(false)
const analysis = ref(null)

const settings = reactive({
  output_format: 'docx',
  layout: '4',
  scale_mode: 'fit',
  orientation: 'auto',
  dpi: 300,
})

async function selectFolder() {
  const selected = await open({ directory: true })
  if (selected) {
    folder.value = selected
    analysis.value = null
  }
}

async function analyze() {
  if (!folder.value) return
  analyzing.value = true
  const result = await tauriCallSafe('analyze_image_paddler_folder', { folder: folder.value })
  if (result.ok) {
    analysis.value = result.data
  }
  analyzing.value = false
}

async function run() {
  if (!folder.value) return
  generating.value = true
  const result = await tauriCallSafe('run_image_paddler', {
    args: {
      folder: folder.value,
      ...settings,
    },
  })
  generating.value = false
}
</script>

<style scoped>
.image-paddler-view {
  height: 100%;
}

.paddler-layout {
  display: flex;
  gap: 20px;
  height: 100%;
}

.settings-panel {
  width: 280px;
  flex-shrink: 0;
}

.settings-panel h3 {
  margin: 0 0 4px;
  color: #303133;
}

.hint {
  color: #909399;
  font-size: 12px;
  margin: 0 0 16px;
}

.folder-path {
  font-size: 12px;
  color: #909399;
  word-break: break-all;
  display: block;
  margin-top: 4px;
}

.result-panel {
  flex: 1;
  overflow-y: auto;
}

.analysis-summary {
  margin-bottom: 16px;
}

.groups-section {
  margin-bottom: 16px;
}

.groups-section h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.group-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
}

.image-list h4 {
  margin: 0 0 8px;
  font-size: 13px;
}

.image-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 8px;
}

.image-thumb {
  text-align: center;
  padding: 8px;
  background: #f5f7fa;
  border-radius: 4px;
}

.thumb-placeholder {
  width: 60px;
  height: 60px;
  margin: 0 auto 4px;
  background: #e4e7ed;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  color: #909399;
}

.thumb-name {
  display: block;
  font-size: 11px;
  color: #606266;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.thumb-size {
  font-size: 10px;
  color: #c0c4cc;
}

.more-images {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
  font-size: 13px;
}
</style>
