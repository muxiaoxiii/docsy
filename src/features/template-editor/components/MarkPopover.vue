<template>
  <el-popover
    trigger="manual"
    :visible="visible"
    :virtual-ref="anchor"
    virtual-triggering
    :width="320"
    placement="top"
  >
    <div
      class="mark-form"
      v-if="pending"
      @mousedown.stop
      @mouseup.stop
      @click.stop
    >
      <div class="mark-form-row">
        <span class="mark-form-label">选中文本</span>
        <span class="mark-form-text">{{ pending.text }}</span>
      </div>
      <el-alert
        v-if="pending.duplicateChoices?.length"
        class="duplicate-alert"
        type="warning"
        :closable="false"
        show-icon
        title="已有相同或相近文本，可复用已有字段，也可以新建字段。"
      />
      <el-form label-width="80px" size="small">
        <el-form-item
          v-if="pending.duplicateChoices?.length"
          label="复用字段"
        >
          <el-select
            :model-value="pending.reuseKey"
            clearable
            placeholder="不复用，作为新字段"
            style="width: 100%"
            @update:model-value="$emit('reuse-field', $event)"
          >
            <el-option
              v-for="c in pending.duplicateChoices"
              :key="c.key"
              :label="`${c.label || c.key}（${c.text}）`"
              :value="c.key"
            />
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="pending.locationChoices?.length > 1"
          label="原文位置"
        >
          <el-select
            :model-value="pending.locationKey"
            placeholder="选择要标记的位置"
            style="width: 100%"
            @update:model-value="$emit('choose-location', $event)"
          >
            <el-option
              v-for="c in pending.locationChoices"
              :key="c.key"
              :label="c.label"
              :value="c.key"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="标签">
          <el-input
            :model-value="pending.label"
            @update:model-value="updateField('label', $event)"
            :placeholder="suggestLabel(pending.text)"
            :disabled="!!pending.reuseKey"
          />
        </el-form-item>
        <el-form-item label="类型">
          <el-select
            :model-value="pending.type"
            style="width: 100%"
            :disabled="!!pending.reuseKey"
            @update:model-value="updateField('type', $event)"
          >
            <el-option label="文本（推荐）" value="text" />
            <el-option label="日期" value="date" />
            <el-option label="单选（带候选）" value="select" />
            <el-option label="当事人列表（多人）" value="party" />
          </el-select>
        </el-form-item>
        <el-form-item label="可见性">
          <el-radio-group
            :model-value="pending.visibility"
            @update:model-value="updateField('visibility', $event)"
          >
            <el-radio value="full">完整 👀</el-radio>
            <el-radio value="value_only">只值 👁</el-radio>
            <el-radio value="auto">自动 🙈</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="必填">
          <el-checkbox
            :model-value="pending.required"
            @update:model-value="updateField('required', $event)"
          />
        </el-form-item>
        <el-form-item label="行重复">
          <el-checkbox
            :model-value="pending.row_repeat"
            @update:model-value="updateField('row_repeat', $event)"
          >
            作为表格列表项（每个值占一行）
          </el-checkbox>
        </el-form-item>
        <el-form-item label="自动编号" v-if="pending.row_repeat">
          <el-checkbox
            :model-value="pending.auto_number"
            @update:model-value="updateField('auto_number', $event)"
          >
            当前选中的位置渲染为行号 1/2/3…
          </el-checkbox>
        </el-form-item>
      </el-form>
      <div class="mark-form-actions">
        <el-button size="small" @click="$emit('cancel-mark')">取消</el-button>
        <el-button size="small" type="primary" @click="$emit('confirm-mark', pending)">
          {{ isEditing ? '保存修改' : '标记为字段' }}
        </el-button>
      </div>
    </div>
  </el-popover>
</template>

<script setup>
const props = defineProps({
  visible: { type: Boolean, default: false },
  anchor: { type: Object, default: null },
  pending: { type: Object, default: null },
  isEditing: { type: Boolean, default: false },
});

const emit = defineEmits([
  "confirm-mark",
  "cancel-mark",
  "reuse-field",
  "choose-location",
  "update:pending",
]);

function updateField(field, value) {
  if (!props.pending) return;
  emit("update:pending", { ...props.pending, [field]: value });
}

function suggestLabel(text) {
  if (!text) return "如 法院";
  const t = text.trim();
  if (t.length <= 6) return `如 ${t}`;
  return `如 ${t.slice(0, 6)}…`;
}
</script>

<style scoped>
.mark-form-row {
  margin-bottom: 10px;
}
.mark-form-label {
  font-size: 12px;
  color: #6b7280;
  margin-right: 8px;
}
.mark-form-text {
  font-size: 13px;
  color: #1f2937;
}
.mark-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
.duplicate-alert {
  margin-bottom: 10px;
}
</style>
