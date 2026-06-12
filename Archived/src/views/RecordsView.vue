<template>
  <div class="records-view">
    <div class="records-toolbar">
      <el-select
        v-model="filterTemplate"
        placeholder="全部模板"
        clearable
        size="small"
        style="width: 180px"
      >
        <el-option
          v-for="t in templateOptions"
          :key="t.id"
          :label="t.name"
          :value="t.id"
        />
      </el-select>
      <el-select
        v-model="sortOrder"
        size="small"
        style="width: 140px"
      >
        <el-option label="最新优先" value="desc" />
        <el-option label="最早优先" value="asc" />
      </el-select>
      <span class="records-count">共 {{ filteredRecords.length }} 条记录</span>
    </div>

    <el-empty
      v-if="!filteredRecords.length"
      description="还没有生成记录"
      :image-size="80"
    />

    <el-table
      v-else
      :data="filteredRecords"
      size="small"
      border
      class="records-table"
      @row-dblclick="loadRecord"
    >
      <el-table-column label="时间" width="160">
        <template #default="{ row }">
          {{ formatTime(row.timestamp) }}
        </template>
      </el-table-column>
      <el-table-column label="模板" width="140" show-overflow-tooltip>
        <template #default="{ row }">
          {{ row.templateName }}
        </template>
      </el-table-column>
      <el-table-column prop="label" label="说明" min-width="220" show-overflow-tooltip />
      <el-table-column label="输出文件" min-width="180" show-overflow-tooltip>
        <template #default="{ row }">
          <span class="muted">{{ row.output_path }}</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200">
        <template #default="{ row }">
          <el-button size="small" link type="primary" @click="loadRecord(row)">
            载入
          </el-button>
          <el-button size="small" link @click="openOutput(row)">
            打开
          </el-button>
          <el-button size="small" link type="danger" @click="deleteRecord(row)">
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="records-tip">双击行可直接载入到对应模板生成页</div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage, ElMessageBox } from "element-plus";

const emit = defineEmits(["navigate"]);

const allRecords = ref([]);
const templateOptions = ref([]);
const filterTemplate = ref("");
const sortOrder = ref("desc");

const filteredRecords = computed(() => {
  let list = allRecords.value;
  if (filterTemplate.value) {
    list = list.filter((r) => r.template_id === filterTemplate.value);
  }
  const sorted = [...list].sort((a, b) =>
    sortOrder.value === "desc"
      ? (b.timestamp || "").localeCompare(a.timestamp || "")
      : (a.timestamp || "").localeCompare(b.timestamp || "")
  );
  return sorted;
});

onMounted(async () => {
  await loadAll();
});

async function loadAll() {
  const templates = [{ id: "letter", name: "律师事务所函" }];
  try {
    const users = await invoke("list_user_templates");
    for (const t of users || []) {
      templates.push({ id: t.id, name: t.name });
    }
  } catch {}

  templateOptions.value = templates;

  const nameMap = {};
  for (const t of templates) {
    nameMap[t.id] = t.name;
  }

  const records = [];
  for (const t of templates) {
    try {
      const recs = await invoke("list_generation_records", { templateId: t.id });
      for (const r of recs || []) {
        records.push({ ...r, templateName: nameMap[t.id] || t.id });
      }
    } catch {}
  }

  records.sort((a, b) => (b.timestamp || "").localeCompare(a.timestamp || ""));
  allRecords.value = records;
}

function formatTime(ts) {
  if (!ts) return "";
  const m = String(ts).match(/^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$/);
  if (!m) return ts;
  return `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}:${m[6]}`;
}

async function loadRecord(row) {
  if (row.template_id === "letter") {
    emit("navigate", "letter");
  } else {
    emit("navigate", `tpl:${row.template_id}`);
  }
}

async function openOutput(row) {
  if (!row.output_path) return;
  try {
    await invoke("open_path", { path: row.output_path });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}

async function deleteRecord(row) {
  try {
    await ElMessageBox.confirm(
      `删除 ${formatTime(row.timestamp)} 的记录？`,
      "确认删除",
      { type: "warning" }
    );
    await invoke("delete_generation_record", {
      templateId: row.template_id,
      recordId: row.id,
    });
    allRecords.value = allRecords.value.filter((r) => r.id !== row.id || r.template_id !== row.template_id);
    ElMessage.success("已删除");
  } catch {
    // 取消
  }
}
</script>

<style scoped>
.records-view {
  max-width: 960px;
  margin: 0 auto;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  padding: 20px;
}
.records-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.records-count {
  margin-left: auto;
  font-size: 13px;
  color: #6b7280;
}
.records-table {
  width: 100%;
}
.records-tip {
  margin-top: 12px;
  text-align: center;
  font-size: 12px;
  color: #9ca3af;
}
.muted {
  color: #9ca3af;
  font-size: 12px;
}
</style>
