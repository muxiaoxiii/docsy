<template>
  <div class="letter-view">
    <el-skeleton v-if="!fields" :rows="6" animated />

    <template v-else>
      <div class="letter-form">
        <div
          v-for="f in fields"
          :key="f.key"
          class="letter-row"
        >
          <div class="letter-label">
            <el-popover
              trigger="manual"
              placement="top-start"
              :width="200"
              :visible="visPopoverOpen === f.key"
              @click.stop
            >
              <template #reference>
                <span
                  class="field-label"
                  :class="{ 'field-label-required': requiredMap[f.key] }"
                  @click.stop="toggleVisPopover(f.key)"
                >
                  {{ f.label }}
                </span>
              </template>
              <div class="vis-popover">
                <div class="vis-row">
                  <span
                    v-for="opt in VIS_OPTIONS"
                    :key="opt.value"
                    class="vis-emoji"
                    :class="{ 'vis-emoji-active': visMap[f.key] === opt.value }"
                    :title="opt.title"
                    @click="setVis(f.key, opt.value)"
                  >{{ opt.emoji }}</span>
                </div>
                <div class="vis-current-text">{{ visTitle(f.key) }}</div>
              </div>
            </el-popover>
          </div>
          <div class="letter-control">
            <FieldDate
              v-if="f.type === 'date'"
              v-model="formValues[f.key]"
              :field="f"
            />
            <FieldSelect
              v-else-if="f.type === 'select'"
              v-model="formValues[f.key]"
              :field="f"
              :options="dictOptions(f)"
            />
            <FieldParty
              v-else-if="f.type === 'party'"
              v-model="formValues[f.key]"
              :field="f"
              :options="dictOptions(f)"
            />
            <FieldReference
              v-else-if="f.type === 'reference'"
              v-model="formValues[f.key]"
              :field="f"
              :all-values="formValues"
              :all-fields="fields"
            />
            <FieldList
              v-else-if="f.multiple"
              v-model="formValues[f.key]"
              :field="f"
              :options="dictOptions(f)"
            />
            <FieldText
              v-else
              v-model="formValues[f.key]"
              :field="f"
              :options="dictOptions(f)"
            />
          </div>
        </div>
      </div>

      <div class="letter-actions">
        <el-button type="primary" :loading="generating" @click="generate">
          生成{{ currentTemplateName }}
        </el-button>
        <el-button @click="reset">重置</el-button>
        <el-button @click="showHistory = true">
          🕒 历史 ({{ historyCount }})
        </el-button>
        <span v-if="lastOutput" class="last-output">
          已生成：{{ lastOutput }}
          <el-button link type="primary" @click="openLast">打开</el-button>
        </span>
      </div>
    </template>

    <el-drawer
      v-model="showHistory"
      title="生成历史"
      direction="rtl"
      size="40%"
    >
      <div v-if="!history.length" class="history-empty">
        还没有生成记录
      </div>
      <el-table
        v-else
        :data="history"
        size="small"
        @row-dblclick="loadHistory"
      >
        <el-table-column label="时间" width="160">
          <template #default="{ row }">
            {{ formatTimeLabel(row.timestamp) }}
          </template>
        </el-table-column>
        <el-table-column prop="label" label="说明" min-width="180" show-overflow-tooltip />
        <el-table-column label="操作" width="160">
          <template #default="{ row }">
            <el-button size="small" link @click="previewHistory(row)">
              查看
            </el-button>
            <el-button size="small" link type="primary" @click="loadHistory(row)">
              载入
            </el-button>
            <el-button size="small" link type="danger" @click="deleteHistory(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <div class="history-tip">双击行可直接载入</div>
    </el-drawer>
  </div>
</template>

<script setup>
defineOptions({ name: "LetterView" });

import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { ElMessage, ElMessageBox } from "element-plus";
<<<<<<< ours
=======
import { logTrace } from "../services/appLogger.js";
>>>>>>> theirs
import FieldText from "../components/FieldText.vue";
import FieldList from "../components/FieldList.vue";
import FieldDate from "../components/FieldDate.vue";
import FieldSelect from "../components/FieldSelect.vue";
import FieldParty from "../components/FieldParty.vue";
import FieldReference from "../components/FieldReference.vue";

const VIS_OPTIONS = [
  { value: "full", emoji: "👀", title: "完整显示：标签前缀 + 值" },
  { value: "value_only", emoji: "👁", title: "只显示值" },
  { value: "auto", emoji: "🙈", title: "值为空时隐藏整段（含标签）" },
];

const fields = ref(null);
const hiddenTemplateFields = ref([]);
const dictionaries = ref({});
const formValues = reactive({});
const requiredMap = reactive({});
const visMap = reactive({});
const visPopoverOpen = ref(null);
const generating = ref(false);
const lastOutput = ref("");
const showHistory = ref(false);
const history = ref([]);
const currentTemplateId = ref("letter");
const currentTemplateName = ref("律师事务所函");
const userTemplates = ref([]);

const props = defineProps({
  templateId: { type: String, default: "letter" },
});

const historyCount = computed(() => history.value.length);

function isFieldVisibleInForm(field) {
  return field?.enabled !== false && field?.hidden_in_form !== true;
}

async function loadHistoryList() {
  try {
    history.value = await invoke("list_generation_records", {
      templateId: currentTemplateId.value,
    });
  } catch {
    history.value = [];
  }
}

function formatTimeLabel(ts) {
  if (!ts) return "";
  const m = String(ts).match(/^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$/);
  if (!m) return ts;
  return `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}:${m[6]}`;
}

async function loadHistory(row) {
  try {
    const rec = await invoke("read_generation_record", {
      templateId: currentTemplateId.value,
      recordId: row.id,
    });
    applyRecord(rec);
    showHistory.value = false;
    ElMessage.success("已载入历史，按需修改后再生成");
  } catch (err) {
    ElMessage.error(`载入失败：${err}`);
  }
}

async function previewHistory(row) {
  try {
    const rec = await invoke("read_generation_record", {
      templateId: currentTemplateId.value,
      recordId: row.id,
    });
    ElMessageBox.alert(
      buildRecordSummaryHtml(rec),
      `${formatTimeLabel(row.timestamp)} 的字段值`,
      {
        dangerouslyUseHTMLString: true,
        customClass: "record-summary-dialog",
      }
    );
  } catch (err) {
    ElMessage.error(`查看失败：${err}`);
  }
}

function buildRecordSummaryHtml(rec) {
  const values = rec.values || {};
  const rows = (fields.value || []).map((f) => {
    const value = formatFieldValue(values[f.key], f);
    return `<tr><th>${escapeHtml(f.label || f.key)}</th><td>${escapeHtml(value || "（空）")}</td></tr>`;
  });
  const output = rec.output_path
    ? `<div class="record-output">输出文件：${escapeHtml(rec.output_path)}</div>`
    : "";
  return `
    <div class="record-summary">
      <table><tbody>${rows.join("")}</tbody></table>
      ${output}
    </div>
  `;
}

function formatFieldValue(value, field) {
  if (Array.isArray(value)) return value.filter(Boolean).join(field.separator || "、");
  if (value && typeof value === "object") return JSON.stringify(value);
  return String(value ?? "");
}

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function applyRecord(rec) {
  if (!fields.value) return;
  // 还原 values
  const v = rec.values || {};
  for (const f of fields.value) {
    if (!(f.key in v)) continue;
    if (f.type === "party") {
      formValues[f.key] = Array.isArray(v[f.key]) ? v[f.key] : [];
    } else if (f.multiple) {
      const s = String(v[f.key] ?? "");
      formValues[f.key] = s ? s.split(f.separator || "、") : [];
    } else {
      formValues[f.key] = String(v[f.key] ?? "");
    }
  }
  // 还原可见性
  if (rec.field_opts && typeof rec.field_opts === "object") {
    for (const k of Object.keys(rec.field_opts)) {
      const o = rec.field_opts[k];
      if (o?.hideable) visMap[k] = "auto";
    }
  }
  // 还原必填
  if (rec.required_map && typeof rec.required_map === "object") {
    for (const k of Object.keys(rec.required_map)) {
      requiredMap[k] = !!rec.required_map[k];
    }
  }
}

async function deleteHistory(row) {
  try {
    await ElMessageBox.confirm(
      `删除 ${formatTimeLabel(row.timestamp)} 的历史？`,
      "确认",
      { type: "warning" }
    );
    await invoke("delete_generation_record", {
      templateId: currentTemplateId.value,
      recordId: row.id,
    });
    await loadHistoryList();
    ElMessage.success("已删除");
  } catch {
    // 取消
  }
}

onMounted(async () => {
  document.addEventListener("click", onDocClick);
  try {
    const list = await invoke("list_user_templates");
    userTemplates.value = list || [];
    await loadTemplate(props.templateId || "letter");
  } catch (err) {
    ElMessage.error(`加载失败：${err}`);
  }
});

// keep-alive 下切换模板时重新加载
watch(
  () => props.templateId,
  async (newId, oldId) => {
    if (newId && newId !== oldId) {
      await loadTemplate(newId);
    }
  }
);

async function loadTemplate(id) {
  try {
    const [meta, dict] = await Promise.all([
      invoke("get_template_meta", { templateId: id }),
      invoke("get_dictionaries", { templateId: id }),
    ]);
    const loadedFields = meta.fields.fields || [];
    fields.value = loadedFields.filter(isFieldVisibleInForm);
    hiddenTemplateFields.value = loadedFields.filter((f) => !isFieldVisibleInForm(f));
    dictionaries.value = dict || {};
    currentTemplateId.value = meta.id;
    currentTemplateName.value = meta.name;
    initValues(fields.value);
<<<<<<< ours
    await loadHistoryList();
=======
    setupInferenceWatches();
    await loadHistoryList();

    // 探针：加载模板后的字段属性统计
    const props = {};
    for (const f of loadedFields) {
      for (const key of ["dict_source","references","infer_from","multiple","separator","hideable","default_role","value_suffix"]) {
        if (f[key]) {
          props[key] = (props[key] || 0) + 1;
        }
      }
    }
    logTrace("template.generate.load", "loadTemplate", {
      templateId: id,
      totalFields: loadedFields.length,
      visibleFields: fields.value.length,
      hiddenFields: hiddenTemplateFields.value.length,
      fieldProps: props,
      dictKeys: Object.keys(dictionaries.value),
    });
>>>>>>> theirs
  } catch (err) {
    ElMessage.error(`加载模板失败：${err}`);
  }
}

<<<<<<< ours
// 自动推断委托人身份：当 client_name 变化时，从原告/被告/第三人里查
// 只要能从当事人列表里推断出身份，就直接覆盖（用户手动选过的也覆盖，
// 因为名称变了身份多半也变；如要保留手动值，可以改成"infer 出 null 时不覆盖"）
watch(
  () => formValues.client_name,
  (name) => {
    if (!name || !fields.value) return;
    const role = inferRole(name);
    if (role) {
      formValues.client_role = role;
    }
  }
);

function inferRole(name) {
  const map = {
    plaintiffs: "原告",
    defendants: "被告",
    third_parties: "第三人",
  };
  for (const key of ["plaintiffs", "defendants", "third_parties"]) {
    const arr = formValues[key];
    if (Array.isArray(arr) && arr.includes(name)) return map[key];
  }
  return null;
=======
// 通用推断逻辑：当 source_field 变化时，根据 infer_from 配置自动填充目标字段
function setupInferenceWatches() {
  if (!fields.value) return;
  for (const field of fields.value) {
    if (!field.infer_from) continue;
    const { source_field, mapping } = field.infer_from;
    if (!source_field || !mapping) continue;

    watch(
      () => formValues[source_field],
      (value) => {
        if (!value || !fields.value) return;
        const sourceField = fields.value.find((f) => f.key === source_field);
        if (!sourceField) return;

        // 检查 value 在哪个字段中
        const sourceKeys = field.references || Object.keys(mapping);
        for (const key of sourceKeys) {
          const arr = formValues[key];
          if (Array.isArray(arr) && arr.includes(value)) {
            formValues[field.key] = mapping[key] || "";
            return;
          }
        }
      }
    );
  }
>>>>>>> theirs
}

function dictOptions(field) {
  const base = cleanOptionList(field.options || [], field);
  const raw = field.dict_source ? dictionaries.value[field.dict_source] || [] : [];
  const dict = raw.length && typeof raw[0] === "object" && raw[0]?.name
    ? raw.map((p) => p.name)
    : raw;
<<<<<<< ours
  return uniqueOptions([
    ...base,
    ...cleanOptionList(dict, field),
    ...cleanOptionList(historyOptions(field), field),
  ]);
=======

  // 对于 reference 类型，添加引用字段的值作为候选
  const refOptions = [];
  if (field.type === "reference" && field.references) {
    for (const refKey of field.references) {
      const val = formValues[refKey];
      if (Array.isArray(val)) {
        refOptions.push(...val.filter(Boolean));
      } else if (val && String(val).trim()) {
        refOptions.push(String(val));
      }
    }
  }

  // 排除 exclude 中指定的字段的值
  let all = uniqueOptions([
    ...base,
    ...cleanOptionList(refOptions, field),
    ...cleanOptionList(dict, field),
    ...cleanOptionList(historyOptions(field), field),
  ]);

  if (field.exclude && field.exclude.length) {
    const excludeSet = new Set();
    for (const exKey of field.exclude) {
      const val = formValues[exKey];
      if (Array.isArray(val)) {
        val.filter(Boolean).forEach((v) => excludeSet.add(v));
      } else if (val && String(val).trim()) {
        excludeSet.add(String(val));
      }
    }
    all = all.filter((opt) => !excludeSet.has(opt));
  }

  return all;
>>>>>>> theirs
}

function historyOptions(field) {
  const out = [];
  for (const rec of history.value || []) {
    const v = rec.values?.[field.key];
    if (Array.isArray(v)) {
      out.push(...v.filter(Boolean));
    } else if (v !== undefined && v !== null && String(v).trim()) {
      out.push(String(v));
    }
  }
  return out;
}

function uniqueOptions(items) {
  const seen = new Set();
  const out = [];
  for (const item of items || []) {
    const value = typeof item === "object" && item?.name ? item.name : item;
    const s = String(value ?? "").trim();
    if (!s || seen.has(s)) continue;
    seen.add(s);
    out.push(s);
  }
  return out;
}

function cleanOptionList(items, field) {
  return (items || []).filter((item) => {
    const value = typeof item === "object" && item?.name ? item.name : item;
    const s = String(value ?? "").trim();
    if (!s) return false;
    if (/^\{\{[#*]?\w+\}\}$/.test(s)) return false;
    if (/^field_\d+$/.test(s)) return false;
    if (s === field.key) return false;
    return true;
  });
}

function initValues(list) {
  for (const f of list) {
    requiredMap[f.key] = !!f.required;
    visMap[f.key] = f.visibility || "value_only";
    if (f.type === "party") {
      formValues[f.key] = [];
    } else if (f.multiple) {
      formValues[f.key] = [];
    } else if (f.type === "date") {
      formValues[f.key] = f.default_today
        ? formatToday(f.format)
        : f.default || "";
    } else {
      // text / number / textarea / select / reference 等都按字符串处理
      formValues[f.key] = f.default || "";
    }
  }
}

function formatToday(fmt) {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  if (!fmt) return `${y}-${m}-${day}`;
  return fmt
    .replace("YYYY", y)
    .replace("MM", m)
    .replace("DD", day);
}

function setVis(key, value) {
  visMap[key] = value;
  visPopoverOpen.value = null;
}
function toggleVisPopover(key) {
  visPopoverOpen.value = visPopoverOpen.value === key ? null : key;
}
function visTitle(key) {
  return VIS_OPTIONS.find((o) => o.value === visMap[key])?.title || "";
}

// 点击页面其他地方关闭气泡
function onDocClick() {
  visPopoverOpen.value = null;
}

function buildFieldOpts() {
  const opts = {};
  for (const f of fields.value) {
    opts[f.key] = {
      hideable: visMap[f.key] === "auto",
      separator: f.separator || "",
      separator_drop_underline: !!f.separator_drop_underline,
      value_suffix: f.value_suffix || "",
    };
  }
  for (const f of hiddenTemplateFields.value) {
    opts[f.key] = {
      hideable: true,
      separator: f.separator || "",
      separator_drop_underline: !!f.separator_drop_underline,
      value_suffix: f.value_suffix || "",
    };
  }
  return opts;
}

function buildValues() {
  const out = {};
  for (const f of fields.value) {
    const v = formValues[f.key];
    if (f.type === "party") {
      out[f.key] = (v || []).filter(Boolean);
    } else if (f.multiple) {
      out[f.key] = (v || []).filter(Boolean).join(f.separator || "、");
    } else {
      out[f.key] = String(v ?? "");
    }
  }
  for (const f of hiddenTemplateFields.value) {
    out[f.key] = "";
  }
  return out;
}

async function generate() {
  // 检查必填项
  const missing = [];
  for (const f of fields.value) {
    if (!requiredMap[f.key]) continue;
    const v = formValues[f.key];
    const empty =
      f.type === "party"
        ? !v || v.length === 0 || v.every((s) => !s)
        : !v;
    if (empty) missing.push(f.label);
  }

  if (missing.length) {
    try {
      await ElMessageBox.confirm(
        `${missing.join("、")} 未填，是否继续生成${currentTemplateName.value}？`,
        "确认生成",
        {
          confirmButtonText: "继续生成",
          cancelButtonText: "返回修改",
          type: "warning",
        }
      );
    } catch {
      return; // 用户取消
    }
  }

  const path = await saveDialog({
    defaultPath: defaultFilename(),
    filters: [{ name: "Word", extensions: ["docx"] }],
  });
  if (!path) return;

  generating.value = true;
  try {
    const out = await invoke("generate_letter", {
      args: {
        template_id: currentTemplateId.value,
        values: buildValues(),
        field_opts: buildFieldOpts(),
        output_path: path,
      },
    });
    lastOutput.value = out;
    ElMessage.success(`${currentTemplateName.value}生成成功`);

    // 写入生成历史
    try {
      const ts = formatTimestamp(new Date());
      const label = buildHistoryLabel();
      await invoke("save_generation_record", {
        args: {
          template_id: currentTemplateId.value,
          timestamp: ts,
          values: buildValues(),
          field_opts: buildFieldOpts(),
          required_map: { ...requiredMap },
          output_path: out,
          label,
        },
      });
      await loadHistoryList();
    } catch (e) {
      // 历史写入失败不影响主流程
    }
  } catch (err) {
    ElMessage.error(`生成失败：${err}`);
  } finally {
    generating.value = false;
  }
}

function formatTimestamp(d) {
  const p = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(
    d.getHours()
  )}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

function buildHistoryLabel() {
  const lawyers = (formValues.lawyers || []).filter(Boolean).join("、");
  const client = formValues.client_name || "";
  const caseNo = formValues.case_no || "";
  const parts = [client, lawyers, caseNo].filter(Boolean);
  return parts.join(" / ");
}

function defaultFilename() {
  const name = currentTemplateName.value || "文档";
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `【${name}】${y}${m}${d}.docx`;
}

function reset() {
  if (fields.value) initValues(fields.value);
  lastOutput.value = "";
}

async function openLast() {
  if (!lastOutput.value) return;
  try {
    await invoke("open_path", { path: lastOutput.value });
  } catch (err) {
    ElMessage.error(`打开失败：${err}`);
  }
}
</script>

<style scoped>
.letter-view {
  max-width: 760px;
  margin: 0 auto;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  padding: 24px;
}
.letter-form {
  margin-bottom: 16px;
}
.template-switcher {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 16px;
  margin-bottom: 16px;
  border-bottom: 1px solid #f0f0f0;
}
.switcher-label {
  font-size: 13px;
  color: #4b5563;
}
.muted-name {
  color: #9ca3af;
  font-size: 12px;
}
.letter-row {
  display: flex;
  align-items: flex-start;
  margin-bottom: 16px;
  gap: 12px;
}
.letter-label {
  width: 120px;
  flex-shrink: 0;
  text-align: right;
  font-size: 14px;
  color: #1f2937;
  line-height: 32px;
  padding-top: 0;
}
.letter-control {
  flex: 1;
  min-width: 0;
}
.field-label {
  cursor: pointer;
  user-select: none;
}
.field-label-required::before {
  content: "*";
  color: #ef4444;
  margin-right: 2px;
}
.vis-popover {
  text-align: center;
}
.vis-row {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-bottom: 8px;
}
.vis-emoji {
  cursor: pointer;
  font-size: 22px;
  padding: 4px 8px;
  opacity: 0.5;
  border-radius: 6px;
  transition: all 0.15s;
}
.vis-emoji:hover {
  opacity: 1;
  background: #f3f4f6;
}
.vis-emoji-active {
  opacity: 1;
  background: #eff6ff;
  box-shadow: 0 0 0 1px #4f8cff;
}
.vis-current-text {
  font-size: 12px;
  color: #4b5563;
}
.letter-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}
.last-output {
  color: #6b7280;
  font-size: 13px;
}
.history-empty {
  text-align: center;
  color: #9ca3af;
  padding: 40px 0;
}
.history-tip {
  margin-top: 12px;
  text-align: center;
  font-size: 12px;
  color: #9ca3af;
}
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
</style>
