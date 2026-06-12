<template>
  <div
    v-if="visible"
    class="mark-popover-overlay"
    @mousedown.self="cancel"
  >
    <div class="mark-popover" :style="popoverStyle">
      <div class="popover-header">
        <span>{{ isEdit ? '编辑字段' : '标记字段' }}</span>
        <el-button text @click="cancel" class="close-btn">&times;</el-button>
      </div>

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="70px"
        label-position="left"
        size="small"
      >
        <el-form-item label="标签" prop="label">
          <el-input v-model="form.label" placeholder="如：甲方姓名" />
        </el-form-item>

        <el-form-item label="键名" prop="key">
          <el-input v-model="form.key" placeholder="自动生成" :disabled="isEdit" />
        </el-form-item>

        <el-form-item label="类型" prop="type">
          <el-select v-model="form.type" style="width: 100%">
            <el-option label="文本" value="text" />
            <el-option label="多行文本" value="textarea" />
            <el-option label="日期" value="date" />
            <el-option label="数字" value="number" />
            <el-option label="选择" value="select" />
            <el-option label="当事人" value="party" />
          </el-select>
        </el-form-item>

        <el-form-item label="必填">
          <el-switch v-model="form.required" />
        </el-form-item>

        <el-form-item label="默认值">
          <el-input v-model="form.default" placeholder="可选" />
        </el-form-item>

        <el-form-item v-if="form.type === 'select'" label="选项">
          <div class="options-list">
            <div
              v-for="(opt, idx) in form.options"
              :key="idx"
              class="option-row"
            >
              <el-input v-model="form.options[idx]" size="small" />
              <el-button text type="danger" size="small" @click="form.options.splice(idx, 1)">
                &times;
              </el-button>
            </div>
            <el-button text size="small" @click="form.options.push('')">
              + 添加选项
            </el-button>
          </div>
        </el-form-item>
      </el-form>

      <div class="popover-actions">
        <el-button size="small" @click="cancel">取消</el-button>
        <el-button
          v-if="isEdit"
          size="small"
          type="danger"
          @click="handleDelete"
        >
          删除标记
        </el-button>
        <el-button size="small" type="primary" @click="handleConfirm">
          {{ isEdit ? '更新' : '标记' }}
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, watch, computed } from 'vue'

const props = defineProps({
  visible: Boolean,
  position: { type: Object, default: () => ({ x: 0, y: 0 }) },
  initialData: { type: Object, default: null },
  selectedText: { type: String, default: '' },
  existingKeys: { type: Array, default: () => [] },
})

const emit = defineEmits(['confirm', 'cancel', 'delete'])

const formRef = ref(null)
const form = reactive({
  label: '',
  key: '',
  type: 'text',
  required: false,
  default: '',
  options: [],
})

const isEdit = computed(() => !!props.initialData)

const rules = {
  label: [{ required: true, message: '请输入标签', trigger: 'blur' }],
}

const popoverStyle = computed(() => {
  const pos = props.position || { x: 0, y: 0 }
  return {
    left: `${Math.max(10, pos.x)}px`,
    top: `${Math.max(10, pos.y)}px`,
  }
})

watch(
  () => props.visible,
  (val) => {
    if (val) {
      if (props.initialData) {
        Object.assign(form, {
          label: props.initialData.label || '',
          key: props.initialData.key || '',
          type: props.initialData.type || 'text',
          required: props.initialData.required || false,
          default: props.initialData.default || '',
          options: [...(props.initialData.options || [])],
        })
      } else {
        form.label = guessLabel(props.selectedText)
        form.key = ''
        form.type = 'text'
        form.required = false
        form.default = ''
        form.options = []
      }
    }
  },
)

async function handleConfirm() {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }
  const config = {
    label: form.label,
    key: form.key || undefined,
    type: form.type,
    required: form.required,
    default: form.default || undefined,
    options: form.type === 'select' ? form.options.filter(Boolean) : undefined,
  }
  emit('confirm', config)
}

function handleDelete() {
  emit('delete')
}

function cancel() {
  emit('cancel')
}

function guessLabel(text) {
  if (!text) return ''
  const clean = text.trim()
  if (clean.length <= 10) return clean
  return clean.slice(0, 10) + '...'
}
</script>

<style scoped>
.mark-popover-overlay {
  position: fixed;
  inset: 0;
  z-index: 2000;
  background: rgba(0, 0, 0, 0.15);
}

.mark-popover {
  position: absolute;
  width: 340px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.18);
  padding: 16px;
  z-index: 2001;
}

.popover-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}

.close-btn {
  font-size: 20px;
  padding: 0;
  line-height: 1;
}

.popover-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #ebeef5;
}

.options-list {
  width: 100%;
}

.option-row {
  display: flex;
  gap: 4px;
  margin-bottom: 4px;
  align-items: center;
}
</style>
