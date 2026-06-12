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
                <component
                  :is="fieldComponent(field.type)"
                  v-model="formValues[field.key]"
                  :field="field"
                  :options="getFieldOptions(field)"
                  :all-values="formValues"
                  :all-fields="fields"
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
import { ref, computed, watch, markRaw } from 'vue'
import { useRoute } from 'vue-router'
import { tauriCall } from '../../../core/tauriBridge.js'
import FieldText from '../../../components/FieldText.vue'
import FieldDate from '../../../components/FieldDate.vue'
import FieldSelect from '../../../components/FieldSelect.vue'
import FieldParty from '../../../components/FieldParty.vue'
import FieldReference from '../../../components/FieldReference.vue'
import FieldList from '../../../components/FieldList.vue'

const FIELD_COMPONENT_MAP = {
  text: markRaw(FieldText),
  textarea: markRaw(FieldText),
  date: markRaw(FieldDate),
  select: markRaw(FieldSelect),
  party: markRaw(FieldParty),
  reference: markRaw(FieldReference),
  list: markRaw(FieldList),
}

function fieldComponent(type) {
  return FIELD_COMPONENT_MAP[type] || FIELD_COMPONENT_MAP.text
}

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

    const vals = {}
    for (const field of fields.value) {
      if (field.type === 'party') {
        vals[field.key] = field.multiple ? [''] : ''
      } else if (field.type === 'list') {
        vals[field.key] = ['']
      } else if (field.type === 'date' && field.default_today) {
        vals[field.key] = formatToday()
      } else {
        vals[field.key] = field.default || ''
      }
    }
    formValues.value = vals

    await loadDictionaryOptions()
  } catch (err) {
    console.error('Failed to load template:', err)
  }
}

async function loadDictionaryOptions() {
  for (const field of fields.value) {
    if (!field.dict_source) continue
    try {
      const entries = await tauriCall('query_dictionary', {
        query: {
          dict_name: field.dict_source,
          template_id: templateId.value,
          field_key: field.key,
          limit: 50,
        },
      })
      if (entries?.length) {
        const labels = entries.map(e => e.label || e.key)
        if (!dictionaries.value[field.dict_source]) {
          dictionaries.value[field.dict_source] = labels
        } else {
          const existing = new Set(dictionaries.value[field.dict_source])
          for (const l of labels) {
            if (!existing.has(l)) dictionaries.value[field.dict_source].push(l)
          }
        }
      }
    } catch (err) {
      console.warn(`Failed to load dict for ${field.key}:`, err)
    }
  }
}

function getFieldOptions(field) {
  const opts = []

  if (field.options) {
    for (const o of field.options) {
      const label = typeof o === 'string' ? o : o.name || o.label || String(o)
      if (!opts.includes(label)) opts.push(label)
    }
  }

  if (field.dict_source && dictionaries.value[field.dict_source]) {
    for (const item of dictionaries.value[field.dict_source]) {
      const label = typeof item === 'string' ? item : item.name || item.label || String(item)
      if (!opts.includes(label)) opts.push(label)
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

function formatToday() {
  const d = new Date()
  return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月 ${d.getDate()} 日`
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
    await recordFieldValues()
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
    await recordFieldValues()
    console.log('Generated with PDF:', result)
  } catch (err) {
    console.error('Generate failed:', err)
  } finally {
    generating.value = false
  }
}

async function recordFieldValues() {
  for (const field of fields.value) {
    const val = formValues.value[field.key]
    if (!val) continue

    if (Array.isArray(val)) {
      for (const item of val) {
        if (item) {
          await tauriCall('record_field_usage', {
            args: { template_id: templateId.value, field_key: field.key, value: item },
          }).catch(err => console.warn('record_field_usage failed:', err))
        }
      }
    } else {
      await tauriCall('record_field_usage', {
        args: { template_id: templateId.value, field_key: field.key, value: val },
      }).catch(err => console.warn('record_field_usage failed:', err))
    }
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

async function loadLastInput() {
  if (!templateId.value) return
  try {
    const records = await tauriCall('list_generation_records', { template_id: templateId.value })
    if (records?.length) {
      const last = records[0]
      if (last.values) {
        formValues.value = { ...formValues.value, ...last.values }
      }
    }
  } catch (err) {
    console.warn('Failed to load last input:', err)
  }
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
</style>
