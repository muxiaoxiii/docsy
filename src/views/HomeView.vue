<template>
  <div class="home-view">
    <section class="home-hero">
      <div class="home-hero-copy">
        <h2 class="home-greeting">
          <img class="home-title-logo" src="../assets/docsy-logo.png" alt="" />
          <span>欢迎使用 Docsy</span>
        </h2>
        <p class="home-subtitle">轻量、高效的文档处理工具箱</p>
      </div>
      <img class="home-mascot" src="../assets/doclet-mascot.png" alt="" />
    </section>

    <el-row :gutter="16" class="home-quick">
      <el-col :span="6">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'letter')">
          <div class="home-card-icon">📝</div>
          <div class="home-card-title">生成所函</div>
          <div class="home-card-desc">生成律师事务所函</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'template')">
          <div class="home-card-icon">🛠</div>
          <div class="home-card-title">制作模板</div>
          <div class="home-card-desc">制作 Docsy 模板</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'pdf:unlock')">
          <div class="home-card-icon">🔓</div>
          <div class="home-card-title">PDF 解锁</div>
          <div class="home-card-desc">移除PDF编辑密码</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="home-card" @click="emit('navigate', 'imagePaddler')">
          <div class="home-card-icon">🖼️</div>
          <div class="home-card-title">图片排版</div>
          <div class="home-card-desc">图片批量排版 A4 文档</div>
        </el-card>
      </el-col>
    </el-row>

    <el-divider />

    <div class="home-section">
      <div class="home-section-title">最近模板</div>
      <div v-if="recentTemplates.length" class="template-cards">
        <el-card
          v-for="t in recentTemplates"
          :key="t.id"
          shadow="hover"
          class="template-card"
          @click="openTemplate(t)"
        >
          <div class="template-card-name">{{ t.name }}</div>
          <div class="template-card-meta">
            <el-tag v-if="t.builtin" size="small" type="info" effect="plain">内置</el-tag>
            <span v-else class="template-card-type">自制模板</span>
          </div>
        </el-card>
      </div>
      <el-empty v-else description="还没有模板，去模板制作页创建一个吧" :image-size="80" />
    </div>

    <div class="home-section">
      <div class="home-section-title">最近记录</div>
      <el-table
        v-if="recentRecords.length"
        :data="recentRecords"
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
        <el-table-column prop="templateName" label="模板" width="120" show-overflow-tooltip />
        <el-table-column prop="label" label="说明" min-width="200" show-overflow-tooltip />
        <el-table-column label="操作" width="120">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="loadRecord(row)">
              载入
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-else description="暂无生成记录" :image-size="80" />
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const emit = defineEmits(["navigate"]);

const recentTemplates = ref([]);
const recentRecords = ref([]);

onMounted(async () => {
  await loadRecentTemplates();
  await loadRecentRecords();
});

async function loadRecentTemplates() {
  try {
    const list = [];
    // 内置所函
    try {
      const enabled = await invoke("is_template_enabled", { templateId: "letter" });
      if (enabled) {
        list.push({ id: "letter", name: "律师事务所函", builtin: true });
      }
    } catch {}
    // 用户模板
    try {
      const users = await invoke("list_user_templates");
      for (const t of users || []) {
        try {
          const enabled = await invoke("is_template_enabled", { templateId: t.id });
          if (enabled) list.push({ id: t.id, name: t.name, builtin: false });
        } catch {
          list.push({ id: t.id, name: t.name, builtin: false });
        }
      }
    } catch {}
    recentTemplates.value = list;
  } catch {
    recentTemplates.value = [];
  }
}

async function loadRecentRecords() {
  try {
    const allRecords = [];
    const templateIds = ["letter"];
    try {
      const users = await invoke("list_user_templates");
      for (const t of users || []) {
        templateIds.push(t.id);
      }
    } catch {}

    // 模板 id → 名称映射
    const nameMap = { letter: "律师事务所函" };
    try {
      const users = await invoke("list_user_templates");
      for (const t of users || []) {
        nameMap[t.id] = t.name;
      }
    } catch {}

    for (const tid of templateIds) {
      try {
        const recs = await invoke("list_generation_records", { templateId: tid });
        for (const r of recs || []) {
          allRecords.push({ ...r, templateName: nameMap[tid] || tid });
        }
      } catch {}
    }

    // 按时间降序，取最近 10 条
    allRecords.sort((a, b) => (b.timestamp || "").localeCompare(a.timestamp || ""));
    recentRecords.value = allRecords.slice(0, 10);
  } catch {
    recentRecords.value = [];
  }
}

function formatTime(ts) {
  if (!ts) return "";
  const m = String(ts).match(/^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})$/);
  if (!m) return ts;
  return `${m[1]}-${m[2]}-${m[3]} ${m[4]}:${m[5]}:${m[6]}`;
}

function openTemplate(t) {
  if (t.id === "letter") {
    emit("navigate", "letter");
  } else {
    emit("navigate", `tpl:${t.id}`);
  }
}

function loadRecord(row) {
  if (row.template_id === "letter") {
    emit("navigate", "letter");
  } else {
    emit("navigate", `tpl:${row.template_id}`);
  }
}
</script>

<style scoped>
.home-view {
  max-width: 960px;
  margin: 0 auto;
}
.home-hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  min-height: 132px;
  margin-bottom: 20px;
}
.home-hero-copy {
  min-width: 0;
}
.home-greeting {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 24px;
  margin: 0 0 4px;
}
.home-title-logo {
  width: 38px;
  height: 38px;
  object-fit: contain;
  flex: 0 0 auto;
}
.home-subtitle {
  color: #6b7280;
  margin: 0;
}
.home-mascot {
  width: clamp(92px, 14vw, 132px);
  height: clamp(100px, 15vw, 148px);
  object-fit: contain;
  flex: 0 0 auto;
}
.home-quick {
  margin-bottom: 8px;
}
.home-card {
  text-align: center;
  cursor: pointer;
  transition: transform 0.15s ease;
}
.home-card:hover {
  transform: translateY(-2px);
}
.home-card-icon {
  font-size: 32px;
  margin-bottom: 8px;
}
.home-card-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 4px;
}
.home-card-desc {
  font-size: 12px;
  color: #6b7280;
}
.home-section {
  margin: 24px 0;
}
.home-section-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: #374151;
}
.template-cards {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}
.template-card {
  width: 180px;
  cursor: pointer;
  transition: transform 0.15s ease;
}
.template-card:hover {
  transform: translateY(-2px);
}
.template-card-name {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}
.template-card-meta {
  font-size: 12px;
  color: #6b7280;
}
.template-card-type {
  color: #9ca3af;
}
.records-table {
  width: 100%;
}
</style>
