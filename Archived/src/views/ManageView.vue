<template>
  <el-container class="manage-view">
    <el-aside width="220px" class="manage-aside">
      <div class="aside-title">已启用模板</div>
      <el-menu :default-active="active" @select="onSelect">
        <el-menu-item
          v-for="t in enabledTemplates"
          :key="t.id"
          :index="t.id"
        >
          <span class="menu-row">
            <span>
              {{ t.name }}
              <el-tag v-if="t.builtin" size="small" type="info" effect="plain" class="builtin-tag">
                内置
              </el-tag>
            </span>
            <el-button
              size="small"
              link
              type="danger"
              class="row-action"
              @click.stop="disable(t.id)"
            >
              删除
            </el-button>
          </span>
        </el-menu-item>
      </el-menu>

      <template v-if="disabledTemplates.length">
        <div class="aside-title aside-title-deleted">已删除</div>
        <el-menu>
          <el-menu-item
            v-for="t in disabledTemplates"
            :key="t.id"
            :index="`__d_${t.id}`"
          >
              <span class="menu-row">
                <span class="deleted-name">{{ t.name }}</span>
                <el-button
                  size="small"
                  link
                  type="primary"
                  class="row-action"
                  @click.stop="editTemplate(t.id)"
                >
                  编辑
                </el-button>
                <el-button
                  size="small"
                  link
                type="primary"
                class="row-action"
                @click.stop="enable(t.id)"
              >
                恢复
              </el-button>
            </span>
          </el-menu-item>
        </el-menu>
      </template>
    </el-aside>

    <el-container class="manage-main">
      <el-header class="manage-head">
        <el-tabs v-model="tab" class="manage-tabs">
          <el-tab-pane label="字段" name="fields" />
<<<<<<< ours
=======
          <el-tab-pane label="推荐逻辑" name="logic" />
>>>>>>> theirs
          <el-tab-pane label="字典" name="dicts" />
          <el-tab-pane label="历史" name="history" />
          <el-tab-pane label="生成记录" name="records" />
        </el-tabs>
        <div class="manage-actions">
          <el-button
            v-if="active"
            size="small"
            @click="editTemplate()"
          >
            编辑模板
          </el-button>
          <el-input-number
            v-model="maxArchives"
            :min="1"
            :max="50"
            size="small"
            controls-position="right"
            class="max-archives"
          />
          <span class="hint">最多归档</span>
          <el-button :disabled="!dirty" @click="reload">放弃改动</el-button>
          <el-button type="primary" :loading="saving" @click="save">
            保存
          </el-button>
        </div>
      </el-header>

      <el-main class="manage-body">
        <el-skeleton v-if="!current" :rows="6" animated />

        <template v-else>
          <div v-if="tab === 'fields'" class="fields-pane">
            <div class="pane-toolbar">
              <el-checkbox v-model="showInternal" size="small">
                显示 key 和类型（编辑模板时需要）
              </el-checkbox>
            </div>
            <el-table :data="current.fields" size="small" border row-key="key">
              <el-table-column v-if="showInternal" prop="key" label="key" width="120" />
              <el-table-column label="生成页" width="82">
                <template #default="{ row }">
                  <el-tag
                    size="small"
                    :type="isFieldVisibleInForm(row) ? 'success' : 'info'"
                    effect="plain"
                  >
                    {{ isFieldVisibleInForm(row) ? "显示" : "隐藏" }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column label="标签" width="140">
                <template #default="{ row }">
                  <el-input v-model="row.label" size="small" @input="markDirty" />
                </template>
              </el-table-column>
              <el-table-column v-if="showInternal" prop="type" label="类型" width="100" />
              <el-table-column label="默认必填" width="90">
                <template #default="{ row }">
                  <el-checkbox v-model="row.required" @change="markDirty" />
                </template>
              </el-table-column>
              <el-table-column label="默认可见性" width="160">
                <template #default="{ row }">
                  <el-select v-model="row.visibility" size="small" @change="markDirty">
                    <el-option label="完整显示 👀" value="full" />
                    <el-option label="只显示值 👁" value="value_only" />
                    <el-option label="自动隐藏 🙈" value="auto" />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column label="value 后缀" width="120">
                <template #default="{ row }">
                  <el-input
                    v-model="row.value_suffix"
                    size="small"
                    placeholder="例如：律师"
                    @input="markDirty"
                  />
                </template>
              </el-table-column>
              <el-table-column label="字典源" width="120">
                <template #default="{ row }">
                  <el-select
                    v-model="row.dict_source"
                    size="small"
                    clearable
                    @change="markDirty"
                  >
                    <el-option
                      v-for="k in dictKeys"
                      :key="k"
                      :label="dictLabel(k)"
                      :value="k"
                    />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column label="默认值">
                <template #default="{ row }">
                  <el-input
                    v-model="row.default"
                    size="small"
                    placeholder="可留空"
                    @input="markDirty"
                  />
                </template>
              </el-table-column>
              <el-table-column label="操作" width="120">
                <template #default="{ row }">
                  <el-button
                    size="small"
                    link
                    :type="isFieldVisibleInForm(row) ? 'warning' : 'primary'"
                    @click="toggleFieldInForm(row)"
                  >
                    {{ isFieldVisibleInForm(row) ? "隐藏" : "恢复" }}
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>

<<<<<<< ours
=======
          <div v-if="tab === 'logic'" class="logic-pane">
            <div class="pane-toolbar">
              <span class="hint">配置字段之间的引用、推断和互斥关系</span>
            </div>
            <el-table :data="logicFields" size="small" border row-key="key">
              <el-table-column prop="key" label="字段" width="150" />
              <el-table-column prop="label" label="标签" width="120" />
              <el-table-column label="引用来源" min-width="200">
                <template #default="{ row }">
                  <el-select
                    v-model="row.references"
                    multiple
                    size="small"
                    clearable
                    placeholder="选择引用字段"
                    @change="markDirty"
                  >
                    <el-option
                      v-for="f in current.fields"
                      :key="f.key"
                      :label="f.label || f.key"
                      :value="f.key"
                    />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column label="互斥字段" min-width="200">
                <template #default="{ row }">
                  <el-select
                    v-model="row.exclude"
                    multiple
                    size="small"
                    clearable
                    placeholder="选择互斥字段"
                    @change="markDirty"
                  >
                    <el-option
                      v-for="f in current.fields"
                      :key="f.key"
                      :label="f.label || f.key"
                      :value="f.key"
                    />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column label="推断规则" min-width="250">
                <template #default="{ row }">
                  <div v-if="row.infer_from" class="infer-rule">
                    <el-tag size="small" type="success">自动推断</el-tag>
                    <span class="infer-desc">
                      当「{{ fieldLabel(row.infer_from.source_field) }}」变化时，
                      根据匹配自动填充
                    </span>
                    <el-button size="small" link @click="editInferRule(row)">
                      编辑
                    </el-button>
                  </div>
                  <el-button
                    v-else
                    size="small"
                    link
                    @click="addInferRule(row)"
                  >
                    添加推断规则
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>

>>>>>>> theirs
          <div v-if="tab === 'dicts'" class="dicts-pane">
            <div class="pane-toolbar">
              <el-button size="small" @click="exportDicts">
                导出 Excel 模板
              </el-button>
              <el-button size="small" @click="importDicts">
                从 Excel 导入
              </el-button>
              <span class="hint">导出全部字典；导入时合并到现有字典</span>
            </div>
            <el-tabs v-model="activeDict" tab-position="left" class="dict-tabs">
              <el-tab-pane
                v-for="k in dictKeys"
                :key="k"
                :label="dictLabel(k)"
                :name="k"
              >
                <DictEditor
                  :name="dictLabel(k)"
                  :items="dictionaries[k] || []"
                  @change="(items) => updateDict(k, items)"
                />
              </el-tab-pane>
            </el-tabs>
          </div>

          <div v-if="tab === 'history'" class="history-pane">
            <el-empty
              v-if="!archives.length"
              description="还没有保存过历史"
              :image-size="80"
            />
            <el-table v-else :data="archives" size="small" border>
              <el-table-column label="类型" width="90">
                <template #default="{ row }">
                  <el-tag
                    size="small"
                    :type="row.kind === 'fields' ? 'success' : 'warning'"
                  >
                    {{ row.kind === "fields" ? "字段" : "字典" }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="saved_at" label="时间戳" />
              <el-table-column label="大小" width="100">
                <template #default="{ row }">
                  {{ (row.size / 1024).toFixed(2) }} KB
                </template>
              </el-table-column>
              <el-table-column label="操作" width="240">
                <template #default="{ row }">
                  <el-button size="small" link @click="preview(row)">查看</el-button>
                  <el-button
                    size="small"
                    link
                    type="primary"
                    @click="restore(row)"
                  >
                    恢复
                  </el-button>
                  <el-button
                    size="small"
                    link
                    type="danger"
                    @click="deleteArchive(row)"
                  >
                    删除
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>
          <div v-if="tab === 'records'" class="records-pane">
            <div class="pane-toolbar">
              <span class="hint">最多保留</span>
              <el-input-number
                v-model="historyMax"
                :min="1"
                :max="500"
                size="small"
                controls-position="right"
                style="width: 100px"
                @change="saveHistoryMax"
              />
              <span class="hint">条记录（按模板独立计数，超出自动删除最旧）</span>
            </div>
            <el-empty
              v-if="!records.length"
              description="还没有生成记录"
              :image-size="80"
            />
            <el-table v-else :data="records" size="small" border>
              <el-table-column label="时间" width="160">
                <template #default="{ row }">
                  {{ formatRecordTime(row.timestamp) }}
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
                  <el-button size="small" link @click="previewRecord(row)">查看</el-button>
                  <el-button size="small" link type="primary" @click="openRecordOutput(row)">
                    打开
                  </el-button>
                  <el-button size="small" link type="danger" @click="deleteRecord(row)">
                    删除
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </template>
      </el-main>
    </el-container>
  </el-container>
<<<<<<< ours
=======

  <InferRuleEditor
    v-model:visible="inferRuleVisible"
    :field="inferRuleField"
    :fields="current?.fields || []"
    @confirm="confirmInferRule"
  />
>>>>>>> theirs
</template>

<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage, ElMessageBox } from "element-plus";
import DictEditor from "../components/DictEditor.vue";
<<<<<<< ours
=======
import InferRuleEditor from "../components/InferRuleEditor.vue";
>>>>>>> theirs

const templates = ref([{ id: "letter", name: "律师事务所函", builtin: true }]);
const enabledMap = ref({});
const active = ref("letter");
const tab = ref("fields");
const activeDict = ref("");

const enabledTemplates = computed(() =>
  templates.value.filter((t) => enabledMap.value[t.id] !== false)
);
const disabledTemplates = computed(() =>
  templates.value.filter((t) => enabledMap.value[t.id] === false)
);

const activeTemplate = computed(() =>
  templates.value.find((t) => t.id === active.value)
);

const emit = defineEmits(["templates-changed", "navigate"]);

function editTemplate(id = active.value) {
  if (id) {
    emit("navigate", `edit:${id}`);
  }
}

const current = ref(null);
const dictionaries = ref({});
const archives = ref([]);
const dirty = ref(false);
const saving = ref(false);
const maxArchives = ref(10);
const showInternal = ref(false);
const records = ref([]);
const historyMax = ref(50);
<<<<<<< ours
=======
const inferRuleVisible = ref(false);
const inferRuleField = ref(null);
>>>>>>> theirs

const dictKeys = computed(() => Object.keys(dictionaries.value));

const DICT_LABELS = {
  courts: "法院",
  firms: "律所",
  lawyers: "律师",
  parties: "当事人",
  causes: "案由",
  stages: "阶段",
};

function dictLabel(k) {
  return DICT_LABELS[k] || k;
}

<<<<<<< ours
=======
// 推荐逻辑标签页
const logicFields = computed(() => {
  if (!current.value?.fields) return [];
  return current.value.fields;
});

function fieldLabel(key) {
  if (!current.value?.fields) return key;
  const f = current.value.fields.find((f) => f.key === key);
  return f?.label || key;
}

function editInferRule(row) {
  const field = current.value.fields.find((f) => f.key === row.key);
  if (field) {
    inferRuleField.value = field;
    inferRuleVisible.value = true;
  }
}

function addInferRule(row) {
  const field = current.value.fields.find((f) => f.key === row.key);
  if (field) {
    field.infer_from = null;
    inferRuleField.value = field;
    inferRuleVisible.value = true;
  }
}

function confirmInferRule(rule) {
  if (inferRuleField.value) {
    inferRuleField.value.infer_from = rule;
    markDirty();
  }
}

>>>>>>> theirs
onMounted(async () => {
  await refreshTemplates();
  await load();
});

async function refreshTemplates() {
  try {
    const userList = await invoke("list_user_templates");
    templates.value = [
      { id: "letter", name: "律师事务所函", builtin: true },
      ...userList.map((t) => ({
        id: t.id,
        name: t.name,
        builtin: false,
      })),
    ];
  } catch {
    // 用户模板列表加载失败不影响内置
  }
  await refreshEnabledMap();
}

async function refreshEnabledMap() {
  const map = {};
  for (const t of templates.value) {
    try {
      map[t.id] = await invoke("is_template_enabled", { templateId: t.id });
    } catch {
      map[t.id] = true;
    }
  }
  enabledMap.value = map;
}

async function disable(id) {
  try {
    const t = templates.value.find((x) => x.id === id);
    const isBuiltin = t?.builtin !== false;
    await ElMessageBox.confirm(
      `确定要${isBuiltin ? "删除" : "永久删除"}「${t?.name}」吗？${
        isBuiltin
          ? "删除后可在「已删除」区域恢复。"
          : "用户模板将从磁盘永久删除，无法恢复。"
      }`,
      "删除模板",
      { type: "warning", confirmButtonText: "删除", cancelButtonText: "取消" }
    );
    if (isBuiltin) {
      await invoke("set_template_enabled", { templateId: id, enabled: false });
      await refreshEnabledMap();
    } else {
      await invoke("delete_user_template", { id });
      await refreshTemplates();
    }
    if (active.value === id) {
      const remain = enabledTemplates.value[0];
      if (remain) active.value = remain.id;
    }
    emit("templates-changed");
    ElMessage.success("已删除");
  } catch {
    // 取消
  }
}

async function enable(id) {
  await invoke("set_template_enabled", { templateId: id, enabled: true });
  await refreshEnabledMap();
  emit("templates-changed");
  ElMessage.success("已恢复");
}

async function exportDicts() {
  const path = await saveDialog({
    defaultPath: `Docsy 字典-${formatTimestamp(new Date())}.xlsx`,
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  try {
    await invoke("export_dictionaries_xlsx", { path });
    ElMessage.success("已导出");
    try {
      await invoke("open_path", { path });
    } catch {}
  } catch (err) {
    ElMessage.error(`导出失败：${err}`);
  }
}

async function importDicts() {
  const path = await openDialog({
    multiple: false,
    filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
  });
  if (!path) return;
  let mode = "merge";
  try {
    await ElMessageBox.confirm(
      "导入方式：合并（同名字典整体替换为 Excel 内容，其他字典保留）。如选择「覆盖」，未出现在 Excel 中的字典也会被清空。",
      "导入字典",
      {
        confirmButtonText: "合并",
        cancelButtonText: "覆盖",
        distinguishCancelAndClose: true,
        type: "info",
      }
    ).catch((action) => {
      if (action === "cancel") {
        mode = "replace";
        return;
      }
      throw "abort";
    });
  } catch {
    return;
  }
  try {
    const next = await invoke("import_dictionaries_xlsx", {
      args: { path, mode },
    });
    dictionaries.value = next;
    ElMessage.success("已导入");
    emit("templates-changed");
    await checkOrphanDicts();
  } catch (err) {
    ElMessage.error(`导入失败：${err}`);
  }
}

async function checkOrphanDicts() {
  if (!current.value) return;
  const referenced = new Set(
    current.value.fields
      .map((f) => f.dict_source)
      .filter(Boolean)
  );
  const orphans = Object.keys(dictionaries.value).filter(
    (k) => !referenced.has(k)
  );
  if (!orphans.length) return;

  try {
    await ElMessageBox.confirm(
      `检测到 ${orphans.length} 个字典未被任何字段引用：${orphans.join("、")}。
是否为它们各自创建一个新字段？字段 key 会自动编号生成，标签默认与字典名相同，可在字段表里继续编辑。`,
      "发现新字典",
      {
        confirmButtonText: "创建字段",
        cancelButtonText: "暂不创建",
        type: "info",
      }
    );
    let counter = current.value.fields.length + 1;
    for (const k of orphans) {
      while (
        current.value.fields.some((f) => f.key === `field_${counter}`)
      ) {
        counter += 1;
      }
      current.value.fields.push({
        key: `field_${counter}`,
        label: dictLabel(k),
        type: "text",
        required: false,
        visibility: "value_only",
        value_suffix: "",
        dict_source: k,
        default: "",
      });
      counter += 1;
    }
    markDirty();
    tab.value = "fields";
    ElMessage.info("已添加占位字段，请在字段 tab 检查并完善");
  } catch {
    // 取消
  }
}

function isFieldVisibleInForm(field) {
  return field?.enabled !== false && field?.hidden_in_form !== true;
}

async function toggleFieldInForm(f) {
  if (!f) return;
  if (!isFieldVisibleInForm(f)) {
    f.enabled = true;
    f.hidden_in_form = false;
    markDirty();
    return;
  }
  try {
    await ElMessageBox.confirm(
      `确定在生成页隐藏字段「${f.label || f.key}」？
字段仍保留在模板中；生成时会按空值处理，并使用自动隐藏规则，真正取消字段标记请进入「编辑模板」。`,
      "隐藏字段",
      {
        type: "warning",
        confirmButtonText: "隐藏",
        cancelButtonText: "取消",
      }
    );
    f.enabled = false;
    f.hidden_in_form = true;
    markDirty();
  } catch {
    // 取消
  }
}

watch(active, load);
watch(tab, async (v) => {
  if (v === "history") loadArchives();
  if (v === "records") loadRecords();
});

async function loadRecords() {
  try {
    records.value = await invoke("list_generation_records", {
      templateId: active.value,
    });
  } catch {
    records.value = [];
  }
  try {
    const settings = await invoke("get_app_settings");
    historyMax.value = settings.history_max || 50;
  } catch {}
}

async function saveHistoryMax(v) {
  try {
    await invoke("set_app_settings", { settings: { history_max: v } });
  } catch (err) {
    ElMessage.error(`保存失败：${err}`);
  }
}

function formatRecordTime(ts) {
  if (!ts) return "";
  const m = String(ts).match(/^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$/);
  if (!m) return ts;
  return `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}:${m[6]}`;
}

async function previewRecord(row) {
  try {
    const data = await invoke("read_generation_record", {
      templateId: active.value,
      recordId: row.id,
    });
    ElMessageBox.alert(
      buildRecordSummaryHtml(data),
      `${formatRecordTime(row.timestamp)} 的字段值`,
      {
        dangerouslyUseHTMLString: true,
        customClass: "record-summary-dialog",
      }
    );
  } catch (err) {
    ElMessage.error(String(err));
  }
}

function buildRecordSummaryHtml(rec) {
  const values = rec.values || {};
  const fieldList = current.value?.fields || [];
  const rows = fieldList.map((f) => {
    const value = formatRecordFieldValue(values[f.key], f);
    return `<tr><th>${escapeHtml(f.label || f.key)}</th><td>${escapeHtml(value || "（空）")}</td></tr>`;
  });
  const output = rec.output_path
    ? `<div class="record-output">输出文件：${escapeHtml(rec.output_path)}</div>`
    : "";
  const raw = `<details><summary>原始 JSON</summary><pre>${escapeHtml(JSON.stringify(rec, null, 2))}</pre></details>`;
  return `
    <div class="record-summary">
      <table><tbody>${rows.join("")}</tbody></table>
      ${output}
      ${raw}
    </div>
  `;
}

function formatRecordFieldValue(value, field) {
  if (Array.isArray(value)) return value.filter(Boolean).join(field.separator || "、");
  if (value && typeof value === "object") return JSON.stringify(value);
  return String(value ?? "");
}

async function openRecordOutput(row) {
  try {
    await invoke("open_path", { path: row.output_path });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}

async function deleteRecord(row) {
  try {
    await ElMessageBox.confirm(
      `删除 ${formatRecordTime(row.timestamp)} 的记录？`,
      "确认",
      { type: "warning" }
    );
    await invoke("delete_generation_record", {
      templateId: active.value,
      recordId: row.id,
    });
    await loadRecords();
    ElMessage.success("已删除");
  } catch {
    // 取消
  }
}

async function load() {
  try {
    const [meta, dicts] = await Promise.all([
      invoke("get_template_meta", { templateId: active.value }),
      invoke("get_dictionaries", { templateId: active.value }),
    ]);
    const fieldsObj = meta.fields || {};
    fieldsObj.fields = (fieldsObj.fields || []).map((f) => ({
      visibility: "value_only",
      value_suffix: "",
<<<<<<< ours
=======
      references: [],
      exclude: [],
>>>>>>> theirs
      ...f,
    }));
    current.value = fieldsObj;
    const t = templates.value.find((x) => x.id === active.value);
    if (t) t.name = meta.name || t.name;
    dictionaries.value = dicts || {};
<<<<<<< ours
    if (!activeDict.value && dictKeys.value.length) {
=======
    if (!dictKeys.value.includes(activeDict.value)) {
>>>>>>> theirs
      activeDict.value = dictKeys.value[0];
    }
    dirty.value = false;
  } catch (err) {
    ElMessage.error(`加载失败：${err}`);
  }
  loadArchives();
}

async function loadArchives() {
  try {
    const dictKey =
      active.value === "letter" ? "dictionaries" : `dict_${active.value}`;
    const [fieldArch, dictArch] = await Promise.all([
      invoke("list_template_archives", { templateId: active.value }),
      invoke("list_template_archives", { templateId: dictKey }),
    ]);
    const fielded = fieldArch.map((a) => ({ ...a, kind: "fields", source: active.value }));
    const dicted = dictArch.map((a) => ({ ...a, kind: "dicts", source: dictKey }));
    archives.value = [...fielded, ...dicted].sort((a, b) =>
      b.id.localeCompare(a.id)
    );
  } catch (err) {
    archives.value = [];
  }
}

function onSelect(v) {
  if (dirty.value) {
    ElMessageBox.confirm("有未保存的改动，确定切换吗？", "提示", {
      type: "warning",
    })
      .then(() => {
        active.value = v;
      })
      .catch(() => {});
  } else {
    active.value = v;
  }
}

function markDirty() {
  dirty.value = true;
}

function updateDict(name, items) {
  dictionaries.value[name] = items;
  markDirty();
}

async function save() {
  if (!current.value) return;
  saving.value = true;
  try {
    const ts = formatTimestamp(new Date());
<<<<<<< ours
    if (activeTemplate.value?.builtin === false) {
      await invoke("update_user_template_config", {
        args: {
          templateId: active.value,
          fields: { fields: current.value.fields },
          dictionaries: dictionaries.value,
          timestamp: ts,
          maxArchives: maxArchives.value,
        },
      });
    } else {
      // 内置模板没有 docsytpl 当前包，继续保存用户覆盖配置。
      await invoke("save_template_config", {
        args: {
          template_id: active.value,
          config: { fields: current.value.fields },
          timestamp: ts,
          max_archives: maxArchives.value,
        },
      });
      const dictKey = active.value === "letter" ? "dictionaries" : `dict_${active.value}`;
      await invoke("save_template_config", {
        args: {
          template_id: dictKey,
          config: dictionaries.value,
          timestamp: ts,
          max_archives: maxArchives.value,
        },
      });
    }
=======
    await invoke("save_template_management_config", {
      args: {
        templateId: active.value,
        fields: { fields: current.value.fields },
        dictionaries: dictionaries.value,
        timestamp: ts,
        maxArchives: maxArchives.value,
      },
    });
>>>>>>> theirs
    ElMessage.success("已保存");
    dirty.value = false;
    emit("templates-changed");
    await loadArchives();
  } catch (err) {
    ElMessage.error(`保存失败：${err}`);
  } finally {
    saving.value = false;
  }
}

function formatTimestamp(d) {
  const p = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(
    d.getHours()
  )}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

async function reload() {
  await load();
  ElMessage.info("已重新加载");
}

async function preview(row) {
  try {
    const data = await invoke("read_template_archive", {
      templateId: row.source,
      archiveId: row.id,
    });
    ElMessageBox.alert(JSON.stringify(data, null, 2), `${row.id}（${row.kind === "fields" ? "字段" : "字典"}）`, {
      customClass: "archive-preview",
    });
  } catch (err) {
    ElMessage.error(String(err));
  }
}

async function restore(row) {
  try {
    await ElMessageBox.confirm(
      `将 ${row.id}（${row.kind === "fields" ? "字段" : "字典"}）设为当前生效配置？`,
      "恢复历史",
      {
        type: "warning",
        confirmButtonText: "恢复",
        cancelButtonText: "取消",
      }
    );
    await invoke("restore_template_archive", {
      templateId: row.source,
      archiveId: row.id,
    });
    ElMessage.success("已恢复");
    await load();
  } catch {
    // 取消
  }
}

async function deleteArchive(row) {
  try {
    let preview = "";
    try {
      const data = await invoke("read_template_archive", {
        templateId: row.source,
        archiveId: row.id,
      });
      preview = JSON.stringify(data, null, 2).slice(0, 1500);
    } catch {}
    await ElMessageBox.confirm(
      `<div style="font-size:12px;color:#6b7280;margin-bottom:6px">${row.kind === "fields" ? "字段归档" : "字典归档"} · ${row.id}</div><pre style="max-height:300px;overflow:auto;background:#f3f4f6;padding:8px;border-radius:4px;font-size:11px">${escapeHtml(preview)}</pre>`,
      "删除归档？",
      {
        dangerouslyUseHTMLString: true,
        type: "warning",
        confirmButtonText: "删除",
        cancelButtonText: "取消",
      }
    );
    await invoke("delete_template_archive", {
      templateId: row.source,
      archiveId: row.id,
    });
    ElMessage.success("已删除");
    await loadArchives();
  } catch {
    // 取消
  }
}

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
</script>

<style scoped>
.manage-view {
  height: 100%;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
}
.manage-aside {
  border-right: 1px solid #e5e7eb;
}
.aside-title {
  padding: 12px 16px;
  font-size: 13px;
  font-weight: 600;
  color: #6b7280;
}
.aside-title-deleted {
  margin-top: 8px;
  border-top: 1px solid #e5e7eb;
}
.menu-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
.row-action {
  opacity: 0;
  transition: opacity 0.15s;
}
.menu-row:hover .row-action {
  opacity: 1;
}
.deleted-name {
  color: #9ca3af;
  text-decoration: line-through;
}
.builtin-tag {
  margin-left: 6px;
}
.manage-main {
  display: flex;
  flex-direction: column;
}
.manage-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #e5e7eb;
  padding: 0 16px;
}
.manage-tabs {
  margin-bottom: -1px;
}
.manage-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.max-archives {
  width: 80px;
}
.hint {
  color: #6b7280;
  font-size: 12px;
}
.manage-body {
  padding: 16px;
}
.pane-toolbar {
  margin-bottom: 12px;
}
.muted {
  color: #9ca3af;
  font-size: 12px;
}
.dict-tabs {
  height: calc(100vh - 240px);
}
<<<<<<< ours
=======
.logic-pane {
  /* 推荐逻辑标签页 */
}
.infer-rule {
  display: flex;
  align-items: center;
  gap: 8px;
}
.infer-desc {
  font-size: 12px;
  color: #6b7280;
}
>>>>>>> theirs
:global(.record-summary table) {
  width: 100%;
  border-collapse: collapse;
}
:global(.record-summary th),
:global(.record-summary td) {
  border-bottom: 1px solid #e5e7eb;
  padding: 8px 6px;
  text-align: left;
  vertical-align: top;
  font-size: 13px;
}
:global(.record-summary th) {
  width: 120px;
  color: #374151;
  background: #f9fafb;
  font-weight: 600;
}
:global(.record-output) {
  margin-top: 12px;
  color: #6b7280;
  font-size: 12px;
  word-break: break-all;
}
:global(.record-summary details) {
  margin-top: 12px;
}
:global(.record-summary pre) {
  max-height: 220px;
  overflow: auto;
  background: #f3f4f6;
  padding: 8px;
  border-radius: 4px;
  font-size: 11px;
}
</style>
