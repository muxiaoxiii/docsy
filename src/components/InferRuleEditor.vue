<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="$emit('update:visible', $event)"
    title="编辑推断规则"
    width="500px"
  >
    <el-form label-width="100px" size="small">
      <el-form-item label="目标字段">
        <el-tag>{{ fieldLabel }}</el-tag>
      </el-form-item>
      <el-form-item label="源字段">
        <el-select
          v-model="form.source_field"
          placeholder="选择触发推断的字段"
          style="width: 100%"
        >
          <el-option
            v-for="f in availableSourceFields"
            :key="f.key"
            :label="f.label || f.key"
            :value="f.key"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="映射关系">
        <div class="mapping-list">
          <div
            v-for="(entry, i) in form.mappingEntries"
            :key="i"
            class="mapping-entry"
          >
            <el-select
              v-model="entry.source_value"
              placeholder="当源字段值为"
              style="width: 180px"
            >
              <el-option
                v-for="opt in sourceValueOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
            <span class="mapping-arrow">→</span>
            <el-input
              v-model="entry.target_value"
              placeholder="目标字段值为"
              style="width: 180px"
            />
            <el-button
              size="small"
              link
              type="danger"
              @click="removeMapping(i)"
            >
              删除
            </el-button>
          </div>
          <el-button size="small" @click="addMapping">
            添加映射
          </el-button>
        </div>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="$emit('update:visible', false)">取消</el-button>
      <el-button type="primary" @click="confirm">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, watch, reactive } from "vue";

const props = defineProps({
  visible: { type: Boolean, default: false },
  field: { type: Object, default: null },
  fields: { type: Array, default: () => [] },
});

const emit = defineEmits(["update:visible", "confirm"]);

const fieldLabel = computed(() => props.field?.label || props.field?.key || "");

const availableSourceFields = computed(() =>
  props.fields.filter((f) => f.key !== props.field?.key)
);

const sourceValueOptions = computed(() => {
  // 从源字段的引用字段中获取可能的值
  const sourceField = props.fields.find((f) => f.key === form.source_field);
  if (!sourceField) return [];
  // 如果源字段有 references，返回这些引用字段的 key
  if (sourceField.references) {
    return sourceField.references.map((refKey) => {
      const refField = props.fields.find((f) => f.key === refKey);
      return {
        label: refField?.label || refKey,
        value: refKey,
      };
    });
  }
  return [];
});

const form = reactive({
  source_field: "",
  mappingEntries: [],
});

// 初始化表单
watch(
  () => props.visible,
  (visible) => {
    if (visible && props.field) {
      const inferFrom = props.field.infer_from || {};
      form.source_field = inferFrom.source_field || "";
      form.mappingEntries = [];
      if (inferFrom.mapping) {
        for (const [key, value] of Object.entries(inferFrom.mapping)) {
          form.mappingEntries.push({
            source_value: key,
            target_value: value,
          });
        }
      }
      if (!form.mappingEntries.length) {
        form.mappingEntries.push({ source_value: "", target_value: "" });
      }
    }
  }
);

function addMapping() {
  form.mappingEntries.push({ source_value: "", target_value: "" });
}

function removeMapping(index) {
  form.mappingEntries.splice(index, 1);
}

function confirm() {
  const mapping = {};
  for (const entry of form.mappingEntries) {
    if (entry.source_value && entry.target_value) {
      mapping[entry.source_value] = entry.target_value;
    }
  }
  emit("confirm", {
    source_field: form.source_field,
    mapping,
  });
  emit("update:visible", false);
}
</script>

<style scoped>
.mapping-list {
  width: 100%;
}
.mapping-entry {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.mapping-arrow {
  color: #6b7280;
  font-weight: 600;
}
</style>
