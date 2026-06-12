<template>
  <div class="dict-editor">
    <div class="dict-head">
      <span class="dict-name">{{ name }}（共 {{ items.length }} 项）</span>
      <el-button size="small" @click="add">+ 新增</el-button>
    </div>

    <el-table :data="items" size="small" border max-height="600">
      <el-table-column type="index" width="50" />
      <template v-if="isParty">
        <el-table-column label="名称">
          <template #default="{ row, $index }">
            <el-input
              v-model="row.name"
              size="small"
              @input="emitChange"
            />
          </template>
        </el-table-column>
        <el-table-column label="主体类型" width="160">
          <template #default="{ row }">
            <el-select v-model="row.subject_type" size="small" @change="emitChange">
              <el-option label="自然人" value="自然人" />
              <el-option label="法人" value="法人" />
              <el-option label="其他组织" value="其他组织" />
            </el-select>
          </template>
        </el-table-column>
      </template>
      <template v-else>
        <el-table-column label="值">
          <template #default="{ row, $index }">
            <el-input
              :model-value="row"
              size="small"
              @input="(v) => updateScalar($index, v)"
            />
          </template>
        </el-table-column>
      </template>
      <el-table-column label="操作" width="80">
        <template #default="{ $index }">
          <el-button
            size="small"
            link
            type="danger"
            @click="remove($index)"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup>
import { computed } from "vue";

const props = defineProps({
  name: { type: String, required: true },
  items: { type: Array, default: () => [] },
});
const emit = defineEmits(["change"]);

const isParty = computed(
  () => props.items.length > 0 && typeof props.items[0] === "object"
);

function emitChange() {
  emit("change", [...props.items]);
}

function updateScalar(i, v) {
  const next = [...props.items];
  next[i] = v;
  emit("change", next);
}

function add() {
  if (isParty.value || props.name === "parties") {
    emit("change", [...props.items, { name: "", subject_type: "法人" }]);
  } else {
    emit("change", [...props.items, ""]);
  }
}

function remove(i) {
  emit(
    "change",
    props.items.filter((_, idx) => idx !== i)
  );
}
</script>

<style scoped>
.dict-editor {
  padding: 0 12px;
}
.dict-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.dict-name {
  font-size: 13px;
  color: #374151;
}
</style>
