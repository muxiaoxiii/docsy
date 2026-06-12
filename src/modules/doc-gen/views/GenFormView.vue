<template>
  <div class="gen-form-view">
    <el-empty v-if="!templateId" description="请从侧边栏选择一个模板" />
    <template v-else>
      <div class="gen-layout">
        <div class="gen-form">
          <el-form label-width="100px" label-position="right">
            <template v-for="field in fields" :key="field.key">
              <el-form-item
                :label="field.label"
                :required="field.required"
                v-show="!field.hidden"
              >
                <!-- Text -->
                <el-input
                  v-if="field.type === 'text'"
                  v-model="formValues[field.key]"
                  :placeholder="field.label"
                  filterable
                />

                <!-- Textarea -->
                <el-input
                  v-else-if="field.type === 'textarea'"
                  v-model="formValues[field.key]"
                  type="textarea"
                  :rows="3"
                  :placeholder="field.label"
                />

                <!-- Date -->
                <el-date-picker
                  v-else-if="field.type === 'date'"
                  v-model="formValues[field.key]"
                  type="date"
                  value-format="YYYY 年 M 月 D 日"
                  placeholder="选择日期"
                />

                <!-- Select -->
                <el-select
                  v-else-if="field.type === 'select'"
                  v-model="formValues[field.key]"
                  filterable
                  allow-create
                  placeholder="请选择或输入"
                  style="width: 100%"
                >
                  <el-option
                    v-for="opt in getFieldOptions(field)"
                    :key="opt.value"
                    :label="opt.label"
                    :value="opt.value"
                  />
                </el-select>

                <!-- Number -->
                <el-input-number
                  v-else-if="field.type === 'number'"
                  v-model="formValues[field.key]"
                  :min="0"
                />

                <!-- Party (multiple names) -->
                <div v-else-if="field.type === 'party'" class="party-field">
                  <div
                    v-for="(item, idx) in formValues[field.key]"
                    :key="idx"
                    class="party-item"
                  >
                    <el-input
                      v-model="formValues[field.key][idx]"
                      :placeholder="`${field.label} ${idx + 1}`"
                      style="flex: 1"
                    />
                    <el-button
                      type="danger"
                      text
                      @click="removePartyItem(field.key, idx)"
                    >
                      删除
                    </el-button>
                  </div>
                  <el-button
                    type="primary"
                    text
                    @click="addPartyItem(field.key)"
                  >
                    + 添加{{ field.label }}
                  </el-button>
                </div>

                <!-- Reference (pick from other fields) -->
                <el-select
                  v-else-if="field.type === 'reference'"
                  v-model="formValues[field.key]"
                  filterable
                  allow-create
                  placeholder="请选择或输入"
                  style="width: 100%"
                >
                  <el-option
                    v-for="opt in getReferenceOptions(field)"
                    :key="opt"
                    :label="opt"
                    :value="opt"
                  />
                </el-select>

                <!-- Fallback: text input -->
                <el-input
                  v-else
                  v-model="formValues[field.key]"
                  :placeholder="field.label"
                />
              </el-form-item>
            </template>
          </el-form>

          <div class="gen-actions">
            <el-button type="primary" @click="handleGenerate" :loading="generating">
              生成文档
            </el-button>
            <el-button @click="handleExportPdf" :loading="generating">
              导出 PDF
            </el-button>
            <el-button @click="loadLastInput" text>
              复用上次输入
            </el-button>
          </div>
        </div>

        <div class="gen-preview">
          <div class="preview-header">预览</div>
          <div v-if="previewHtml" v-html="previewHtml" class="preview-content"></div>
          <el-empty v-else description="填写表单后预览" :image-size="60" />
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { tauriCall } from '../../../core/tauriBridge.js'
import { save as saveTemplate } from '../../template-editor/composables/useTemplateSave.js'

const route = useRoute()
const templateId = computed(() => route.params.templateId)

const fields = ref([])
const formValues = ref({})
const previewHtml = ref('')
const generating = ref(false)
const dictionaries = ref({})

async function loadTemplate() {
  if (!templateId.value) return
  try {
    const meta = await tauriCall('get_template_meta', { templateId: templateId.value })
    if (meta.fields?.fields) {
      fields.value = meta.fields.fields
    }
    if (meta.dictionaries) {
      dictionaries.value = meta.dictionaries
    }
    // Initialize form values
    const vals = {}
    for (const field of fields.value) {
      if (field.type === 'party') {
        vals[field.key] = field.multiple ? [''] : ''
      } else if (field.type === 'date' && field.default_today) {
        vals[field.key] = new Date().toISOString().slice(0, 10)
      } else {
        vals[field.key] = field.default || ''
      }
    }
    formValues.value = vals
  } catch (err) {
    console.error('Failed to load template:', err)
  }
}

function getFieldOptions(field) {
  const opts = []
  // From field options
  if (field.options) {
    for (const o of field.options) {
      if (typeof o === 'string') {
        opts.push({ label: o, value: o })
      } else {
        opts.push({ label: o.name || o.label || String(o), value: o.name || o.label || String(o) })
      }
    }
  }
  // From dictionary
  if (field.dict_source && dictionaries.value[field.dict_source]) {
    for (const item of dictionaries.value[field.dict_source]) {
      const label = typeof item === 'string' ? item : item.name || item.label || String(item)
      if (!opts.find(o => o.value === label)) {
        opts.push({ label, value: label })
      }
    }
  }
  return opts
}

function getReferenceOptions(field) {
  const opts = []
  if (field.references) {
    for (const refKey of field.references) {
      const val = formValues.value[refKey]
      if (Array.isArray(val)) {
        opts.push(...val.filter(Boolean))
      } else if (val) {
        opts.push(val)
      }
    }
  }
  return [...new Set(opts)]
}

function addPartyItem(key) {
  if (Array.isArray(formValues.value[key])) {
    formValues.value[key].push('')
  }
}

function removePartyItem(key, idx) {
  if (Array.isArray(formValues.value[key])) {
    formValues.value[key].splice(idx, 1)
  }
}

async function handleGenerate() {
  if (!templateId.value) return
  generating.value = true
  try {
    const result = await tauriCall('generate_document', {
      args: {
        template_id: templateId.value,
        values: cleanValues(formValues.value),
        export_pdf: false,
      },
    })
    console.log('Generated:', result)
  } catch (err) {
    console.error('Generate failed:', err)
  } finally {
    generating.value = false
  }
}

async function handleExportPdf() {
  if (!templateId.value) return
  generating.value = true
  try {
    const result = await tauriCall('generate_document', {
      args: {
        template_id: templateId.value,
        values: cleanValues(formValues.value),
        export_pdf: true,
      },
    })
    console.log('Generated with PDF:', result)
  } catch (err) {
    console.error('Generate failed:', err)
  } finally {
    generating.value = false
  }
}

function cleanValues(vals) {
  const cleaned = {}
  for (const [k, v] of Object.entries(vals)) {
    if (Array.isArray(v)) {
      cleaned[k] = v.filter(Boolean)
    } else if (v) {
      cleaned[k] = v
    }
  }
  return cleaned
}

function loadLastInput() {
  // TODO: load from history
}

watch(templateId, loadTemplate, { immediate: true })
</script>

<style scoped>
.gen-form-view {
  height: 100%;
}

.gen-layout {
  display: flex;
  gap: 20px;
  height: 100%;
}

.gen-form {
  flex: 1;
  overflow-y: auto;
  padding-right: 20px;
}

.gen-preview {
  width: 400px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.preview-header {
  padding: 8px 16px;
  background: #f5f7fa;
  border-bottom: 1px solid #e4e7ed;
  font-size: 13px;
  font-weight: 600;
  color: #606266;
}

.preview-content {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
  font-size: 14px;
  line-height: 1.8;
}

.gen-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
}

.party-field {
  width: 100%;
}

.party-item {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
}
</style>
