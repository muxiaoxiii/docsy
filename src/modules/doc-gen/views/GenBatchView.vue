<template>
  <div class="batch-view">
    <h3>批量生成</h3>
    <p class="hint">从 Excel/CSV 读取数据，批量生成多份文档</p>

    <el-steps :active="step" finish-status="success" simple style="margin-bottom: 20px">
      <el-step title="选择模板" />
      <el-step title="导入数据" />
      <el-step title="字段映射" />
      <el-step title="生成" />
    </el-steps>

    <!-- Step 1: Select Template -->
    <div v-if="step === 0" class="step-content">
      <el-select v-model="selectedTemplateId" placeholder="选择模板" style="width: 100%">
        <el-option
          v-for="tpl in templates"
          :key="tpl.id"
          :label="tpl.name"
          :value="tpl.id"
        />
      </el-select>
      <el-button type="primary" @click="step = 1" :disabled="!selectedTemplateId" style="margin-top: 12px">
        下一步
      </el-button>
    </div>

    <!-- Step 2: Import Data -->
    <div v-if="step === 1" class="step-content">
      <el-button @click="importExcel">导入 Excel / CSV</el-button>
      <div v-if="importedData.length" class="import-preview">
        <p>已导入 {{ importedData.length }} 行数据</p>
        <el-table :data="importedData.slice(0, 5)" size="small" stripe>
          <el-table-column
            v-for="col in importColumns"
            :key="col"
            :prop="col"
            :label="col"
            min-width="100"
          />
        </el-table>
        <p v-if="importedData.length > 5" class="more-hint">...及其他 {{ importedData.length - 5 }} 行</p>
      </div>
      <div class="step-actions">
        <el-button @click="step = 0">上一步</el-button>
        <el-button type="primary" @click="step = 2" :disabled="!importedData.length">下一步</el-button>
      </div>
    </div>

    <!-- Step 3: Field Mapping -->
    <div v-if="step === 2" class="step-content">
      <p>将 Excel 列映射到模板字段：</p>
      <el-form label-width="120px" size="small">
        <el-form-item v-for="field in templateFields" :key="field.key" :label="field.label">
          <el-select v-model="fieldMapping[field.key]" placeholder="选择 Excel 列" clearable style="width: 100%">
            <el-option
              v-for="col in importColumns"
              :key="col"
              :label="col"
              :value="col"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <div class="step-actions">
        <el-button @click="step = 1">上一步</el-button>
        <el-button type="primary" @click="step = 3">下一步</el-button>
      </div>
    </div>

    <!-- Step 4: Generate -->
    <div v-if="step === 3" class="step-content">
      <el-button type="primary" @click="batchGenerate" :loading="generating">
        批量生成 {{ importedData.length }} 份文档
      </el-button>

      <div v-if="results.length" class="results">
        <el-progress :percentage="progress" style="margin-bottom: 12px" />
        <el-table :data="results" size="small" stripe>
          <el-table-column prop="index" label="#" width="50" />
          <el-table-column prop="status" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.success ? 'success' : 'danger'" size="small">
                {{ row.success ? '成功' : '失败' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="output" label="输出文件" />
          <el-table-column prop="error" label="错误" />
        </el-table>
      </div>

      <div class="step-actions">
        <el-button @click="step = 2">上一步</el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { tauriCallSafe } from '../../../core/tauriBridge.js'
import { open } from '@tauri-apps/plugin-dialog'

const step = ref(0)
const templates = ref([])
const selectedTemplateId = ref('')
const templateFields = ref([])
const importedData = ref([])
const importColumns = ref([])
const fieldMapping = ref({})
const generating = ref(false)
const results = ref([])
const progress = ref(0)

async function loadTemplates() {
  const res = await tauriCallSafe('list_templates')
  if (res.ok) templates.value = res.data
}

async function loadTemplateFields() {
  if (!selectedTemplateId.value) return
  const res = await tauriCallSafe('get_template_meta', { templateId: selectedTemplateId.value })
  if (res.ok) {
    templateFields.value = res.data.fields?.fields || []
  }
}

async function importExcel() {
  const selected = await open({
    filters: [{ name: 'Excel/CSV', extensions: ['xlsx', 'xls', 'csv'] }],
  })
  if (!selected) return

  // For now, read as CSV-like. Full Excel parsing would need calamine on backend.
  // This is a simplified version - real implementation would call a backend command
  importedData.value = []
  importColumns.value = []
  // TODO: implement Excel/CSV parsing via backend command
}

async function batchGenerate() {
  generating.value = true
  results.value = []
  const total = importedData.value.length

  for (let i = 0; i < total; i++) {
    const row = importedData.value[i]
    const values = {}
    for (const [fieldKey, colName] of Object.entries(fieldMapping.value)) {
      if (colName && row[colName] !== undefined) {
        values[fieldKey] = row[colName]
      }
    }

    const res = await tauriCallSafe('generate_document', {
      args: {
        template_id: selectedTemplateId.value,
        values,
        export_pdf: false,
      },
    })

    results.value.push({
      index: i + 1,
      success: res.ok,
      output: res.ok ? res.data.docx_path : null,
      error: res.ok ? null : res.error,
    })
    progress.value = Math.round(((i + 1) / total) * 100)
  }

  generating.value = false
}

onMounted(loadTemplates)
</script>

<style scoped>
.batch-view {
  max-width: 700px;
  margin: 0 auto;
  padding: 20px;
}

.batch-view h3 {
  margin: 0 0 4px;
}

.hint {
  color: #909399;
  font-size: 13px;
  margin: 0 0 16px;
}

.step-content {
  min-height: 200px;
}

.step-actions {
  margin-top: 16px;
  display: flex;
  gap: 8px;
}

.import-preview {
  margin-top: 12px;
}

.more-hint {
  font-size: 12px;
  color: #909399;
}

.results {
  margin-top: 16px;
}
</style>
