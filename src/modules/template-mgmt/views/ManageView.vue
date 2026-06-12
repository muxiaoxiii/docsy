<template>
  <div class="manage-view">
    <div class="manage-layout">
      <!-- Template List -->
      <div class="template-list">
        <div class="list-header">
          <h3>模板列表</h3>
          <el-button type="primary" text @click="loadTemplates">
            刷新
          </el-button>
        </div>
        <div class="list-items">
          <div
            v-for="tpl in templates"
            :key="tpl.id"
            class="template-item"
            :class="{ active: selectedId === tpl.id }"
            @click="selectTemplate(tpl)"
          >
            <div class="tpl-name">
              <el-icon v-if="tpl.builtin"><Star /></el-icon>
              {{ tpl.name }}
            </div>
            <div class="tpl-meta">
              <el-tag v-if="tpl.builtin" size="small" type="info">内置</el-tag>
              <el-tag v-if="tpl.pinned_to_tab" size="small" type="success">已固定</el-tag>
            </div>
          </div>
          <el-empty v-if="!templates.length" description="暂无模板" :image-size="40" />
        </div>
      </div>

      <!-- Template Detail -->
      <div class="template-detail">
        <template v-if="selected">
          <div class="detail-header">
            <h2>{{ selected.name }}</h2>
            <div class="detail-actions">
              <el-button size="small" @click="handleEdit">编辑</el-button>
              <el-button size="small" @click="handlePin">
                {{ selected.pinned_to_tab ? '取消固定' : '固定到侧边栏' }}
              </el-button>
              <el-button size="small" type="danger" @click="handleDelete" v-if="!selected.builtin">
                删除
              </el-button>
            </div>
          </div>

          <el-tabs v-model="activeTab">
            <!-- Fields Tab -->
            <el-tab-pane label="字段" name="fields">
              <el-table :data="fields" stripe size="small">
                <el-table-column prop="key" label="Key" width="120" />
                <el-table-column prop="label" label="标签" width="120" />
                <el-table-column prop="type" label="类型" width="100">
                  <template #default="{ row }">
                    <el-tag size="small">{{ row.type }}</el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="required" label="必填" width="60">
                  <template #default="{ row }">
                    <el-icon v-if="row.required" color="#67c23a"><Check /></el-icon>
                  </template>
                </el-table-column>
                <el-table-column prop="dict_source" label="字典来源" />
              </el-table>
            </el-tab-pane>

            <!-- Dictionary Tab -->
            <el-tab-pane label="字典" name="dict">
              <div class="dict-section">
                <div v-for="(items, name) in dictionaries" :key="name" class="dict-group">
                  <h4>{{ name }} ({{ items.length }})</h4>
                  <div class="dict-items">
                    <el-tag
                      v-for="(item, idx) in items.slice(0, 20)"
                      :key="idx"
                      size="small"
                      class="dict-tag"
                    >
                      {{ typeof item === 'string' ? item : item.name || item.label }}
                    </el-tag>
                    <span v-if="items.length > 20" class="more">+{{ items.length - 20 }} 更多</span>
                  </div>
                </div>
                <el-empty v-if="!Object.keys(dictionaries).length" description="暂无字典数据" :image-size="40" />
              </div>
            </el-tab-pane>

            <!-- History Tab -->
            <el-tab-pane label="生成记录" name="history">
              <el-table :data="records" stripe size="small">
                <el-table-column prop="label" label="标签" />
                <el-table-column prop="timestamp" label="时间" width="180" />
                <el-table-column label="操作" width="120">
                  <template #default="{ row }">
                    <el-button size="small" text @click="loadRecord(row)">复用</el-button>
                    <el-button size="small" text type="danger" @click="deleteRecord(row)">删除</el-button>
                  </template>
                </el-table-column>
              </el-table>
              <el-empty v-if="!records.length" description="暂无记录" :image-size="40" />
            </el-tab-pane>
          </el-tabs>
        </template>
        <el-empty v-else description="选择一个模板查看详情" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { tauriCall, tauriCallSafe } from '../../../core/tauriBridge.js'

const router = useRouter()

const templates = ref([])
const selected = ref(null)
const selectedId = ref(null)
const activeTab = ref('fields')
const fields = ref([])
const dictionaries = ref({})
const records = ref([])

async function loadTemplates() {
  const result = await tauriCallSafe('list_templates')
  if (result.ok) {
    templates.value = result.data
  }
}

async function selectTemplate(tpl) {
  selected.value = tpl
  selectedId.value = tpl.id
  activeTab.value = 'fields'

  // Load meta
  const meta = await tauriCallSafe('get_template_meta', { templateId: tpl.id })
  if (meta.ok) {
    fields.value = meta.data.fields?.fields || []
    dictionaries.value = meta.data.dictionaries || {}
  }

  // Load records
  const recs = await tauriCallSafe('list_generation_records', { templateId: tpl.id })
  records.value = recs.ok ? recs.data : []
}

function handleEdit() {
  if (selectedId.value) {
    router.push({ name: 'template-editor', params: { templateId: selectedId.value } })
  }
}

async function handlePin() {
  if (!selected.value) return
  const cmd = selected.value.pinned_to_tab ? 'unpin_from_tab' : 'pin_to_tab'
  await tauriCallSafe(cmd, { templateId: selected.value.id })
  await loadTemplates()
  if (selectedId.value) {
    const tpl = templates.value.find(t => t.id === selectedId.value)
    if (tpl) selected.value = tpl
  }
}

async function handleDelete() {
  if (!selected.value || selected.value.builtin) return
  await tauriCallSafe('delete_template', { templateId: selected.value.id })
  selected.value = null
  selectedId.value = null
  await loadTemplates()
}

function loadRecord(record) {
  router.push({ name: 'doc-gen-form', params: { templateId: selectedId.value } })
}

async function deleteRecord(record) {
  await tauriCallSafe('delete_generation_record', {
    templateId: selectedId.value,
    recordId: record.id,
  })
  const recs = await tauriCallSafe('list_generation_records', { templateId: selectedId.value })
  records.value = recs.ok ? recs.data : []
}

onMounted(loadTemplates)
</script>

<style scoped>
.manage-view {
  height: 100%;
}

.manage-layout {
  display: flex;
  gap: 20px;
  height: 100%;
}

.template-list {
  width: 240px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  border-bottom: 1px solid #e4e7ed;
  background: #f5f7fa;
}

.list-header h3 {
  margin: 0;
  font-size: 14px;
}

.list-items {
  flex: 1;
  overflow-y: auto;
}

.template-item {
  padding: 10px 12px;
  cursor: pointer;
  border-bottom: 1px solid #f0f0f0;
  transition: background 0.15s;
}

.template-item:hover {
  background: #f5f7fa;
}

.template-item.active {
  background: #ecf5ff;
  border-left: 3px solid #409eff;
}

.tpl-name {
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 4px;
}

.tpl-meta {
  margin-top: 4px;
  display: flex;
  gap: 4px;
}

.template-detail {
  flex: 1;
  overflow-y: auto;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.detail-header h2 {
  margin: 0;
  font-size: 18px;
}

.detail-actions {
  display: flex;
  gap: 8px;
}

.dict-section {
  padding: 8px 0;
}

.dict-group {
  margin-bottom: 16px;
}

.dict-group h4 {
  margin: 0 0 8px;
  font-size: 13px;
  color: #606266;
}

.dict-items {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.dict-tag {
  margin: 0;
}

.more {
  font-size: 12px;
  color: #909399;
  line-height: 24px;
}
</style>
