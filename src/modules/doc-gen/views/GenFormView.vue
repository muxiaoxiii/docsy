<template>
  <div class="gen-form-view">
    <el-empty v-if="!templateId" description="请从侧边栏选择一个模板" />
    <template v-else>
      <div class="gen-layout">
        <div class="gen-form">
          <el-form label-width="100px">
            <el-form-item
              v-for="field in fields"
              :key="field.key"
              :label="field.label"
              :required="field.required"
            >
              <el-input
                v-if="field.type === 'text'"
                v-model="formValues[field.key]"
                :placeholder="field.label"
              />
              <el-input
                v-else-if="field.type === 'textarea'"
                v-model="formValues[field.key]"
                type="textarea"
                :rows="3"
              />
              <el-date-picker
                v-else-if="field.type === 'date'"
                v-model="formValues[field.key]"
                type="date"
                value-format="YYYY-MM-DD"
              />
              <el-select
                v-else-if="field.type === 'select'"
                v-model="formValues[field.key]"
                filterable
                allow-create
                placeholder="请选择"
              >
                <el-option
                  v-for="opt in getFieldOptions(field)"
                  :key="opt"
                  :label="opt"
                  :value="opt"
                />
              </el-select>
              <el-input-number
                v-else-if="field.type === 'number'"
                v-model="formValues[field.key]"
              />
              <el-input
                v-else
                v-model="formValues[field.key]"
                :placeholder="field.label"
              />
            </el-form-item>
          </el-form>
          <div class="gen-actions">
            <el-button type="primary" @click="handleGenerate">生成文档</el-button>
            <el-button @click="loadLastInput">复用上次输入</el-button>
          </div>
        </div>
        <div class="gen-preview" v-if="previewHtml">
          <div v-html="previewHtml" class="preview-content"></div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { tauriCall } from '../../../core/tauriBridge.js'

const route = useRoute()
const templateId = computed(() => route.params.templateId)

const fields = ref([])
const formValues = ref({})
const previewHtml = ref('')

async function loadTemplate() {
  if (!templateId.value) return
  try {
    const meta = await tauriCall('get_template_meta', { templateId: templateId.value })
    if (meta.fields?.fields) {
      fields.value = meta.fields.fields
    }
    // Initialize form values
    for (const field of fields.value) {
      if (!(field.key in formValues.value)) {
        formValues.value[field.key] = field.default || ''
      }
    }
  } catch (err) {
    console.error('Failed to load template:', err)
  }
}

function getFieldOptions(field) {
  if (field.options) {
    return field.options.map(o => typeof o === 'string' ? o : o.name || o.label || String(o))
  }
  return []
}

async function handleGenerate() {
  if (!templateId.value) return
  try {
    const result = await tauriCall('generate_document', {
      args: {
        template_id: templateId.value,
        values: formValues.value,
        export_pdf: false,
      },
    })
    console.log('Generated:', result)
  } catch (err) {
    console.error('Generate failed:', err)
  }
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
  flex: 1;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  padding: 16px;
  overflow-y: auto;
  background: #fafafa;
}

.preview-content {
  font-size: 14px;
  line-height: 1.8;
}

.gen-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
}
</style>
