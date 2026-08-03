# project-flyshu

> 基于飞书多维表格的本地法律案件管理系统 + AI Skill 工作流平台  
> 当前数据：59 案件 / 67 日志 / 55 庭审 / 19 任务 / 9 联系人  
> 栈：Tauri 2 + Vue 3 + SQLite + Rust
> 案件类型：民事侵权 / 行政诉讼 / 专利无效程序

## 行业调研参考

调研了 8 个开源法律科技项目，关键发现：

| 项目 | 启示 |
|---|---|
| **LawLink**（中文本地CMS） | 收案→冲突检索→转正→跟进→财务→归档 的完整状态机 |
| **ef-cms (DAWSON)**（美国税务法院） | 庭审日历的法庭/法官/当事人三实体模型 |
| **vincent9306/law-case-manager**（Py+SQLite） | 8 阶段状态机 + Word模板 + 离线日历，最接近飞鱼架构 |
| **tpt-court**（Next.js法院系统） | 按案件/庭审/文书/通知/收费 拆 API 资源 |
| **legal-case-report**（AI案件报告） | "断点确认"模式：AI 生成 → 人类律师三节点确认 |
| **Docassemble**（法律文书自动化） | YAML+Jinja2 模板 + 变量类型推导 + 插件生态 |
| **LawFirmSystem**（Vue3律所系统） | 利冲检索 + 合同审核 + 结案归档，明确不做HR/财务 |
| **justice-os**（社区驱动法律OS） | 插件市场架构：技能/模板/知识库可热插拔 |

**project-flyshu 的差异化**（这些项目都没做）：
- 专利无效程序专属期限（国知局 1 个月意见陈述期/15 天答辩期/2 个月补充证据期）
- 同一专利的多轨并行案件关系网络（无效+侵权+行政同时进行）
- 国知局口审 vs 法院庭审双轨开庭模型
- 中文专利实践的"交文/收文/记录"三态文书流转

---

## 目录

1. [项目定位](#1-项目定位)
2. [从飞书表格到本地系统：为什么](#2-从飞书表格到本地系统为什么)
3. [数据模型](#3-数据模型)
4. [公式引擎](#4-公式引擎)
5. [飞书同步引擎](#5-飞书同步引擎)
6. [UI 设计](#6-ui-设计)
7. [提醒系统](#7-提醒系统)
8. [Skill 工作流平台](#8-skill-工作流平台)
9. [与 Docsy 的关系](#9-与-docsy-的关系)
10. [实施计划](#10-实施计划)
11. [附录：飞书表格完整字段清单](#11-附录飞书表格完整字段清单)

---

## 1. 项目定位

**不是 CRM**。不是 Clio/MyCase 那种通用律所管理系统。

**不是文档编辑器**。文档生成交给 Docsy（已有模板引擎），本项目的连接点是"从案件一键跳转到 Docsy 生成文书"。

**是三个东西的集合**：

```
┌────────────────────────────────────────────────────┐
│                                                    │
│   📋 案件全景                                       │
│   解决飞书表格"字段太多看不到""关联案件散乱"的问题       │
│   → 聚合视图 + 时间线 + 关联网络 + 日历               │
│                                                    │
│   ⏰ 期限卫士                                       │
│   8 个飞书公式的本地复现 + 主动提醒                    │
│   → WORKDAY/EDATE/TODAY 引擎 + 通知                  │
│                                                    │
│   🧠 Skill 工作流                                    │
│   法律文书工作流 = 提示词 + 模板 + 知识库的组合         │
│   → 专利无效流程 / 起诉状生成 / 证据清单编制            │
│                                                    │
└────────────────────────────────────────────────────┘
```

---

### 1.1 三种案件轨道

你的 59 件案件实际上运行在三条不同的程序轨道上，每条有独立的审理机关、期限规则和文书要求：

| 轨道 | 审理机关 | 主要期限规则 | 数据占比 |
|---|---|---|---|
| **专利无效** | 国知局 | 1月陈述/15天答辩/2月补证（专利审查指南第4部分第3章） | 24/59 |
| **行政诉讼** | 知产法院→最高法 | 15工作日答辩/15天上诉/3-6月审限 | 9/59 |
| **民事侵权** | 中院→高院 | 15天答辩/6月审限（可延长）/10天上诉 | 11/59 |

同一专利常有 3-5 件并行案件：无效在国知局、侵权在知产法院、无效决定后的行政上诉在最高法。这就是 §3.4 案件关系网络的设计动机。

---

## 2. 从飞书表格到本地系统：为什么

飞书多维表格的优点：灵活、多人协作、公式字段、关联表。

飞书多维表格作为案件管理系统的问题：

| 飞书短板 | 本地系统方案 |
|---|---|
| 一个案件 52 个字段，一个页面看不下 | 分组折叠面板：基本信息/期限/专利/对方/结果 |
| 关联案件要跳到另一个表才能看到 | 关联网络图：同一客户/同一专利/双向关联一站显示 |
| 开庭/日志/任务散在不同子表 | 案件详情页时间线：融并日志+庭审+任务一条轴 |
| 期限公式写了 8 个但大部分为空 | 公式引擎实时计算，不依赖飞书 |
| 没有主动提醒 | 系统级 notification |
| 没法自定义工作流 | Skill 系统：预制 + 自定义流程 |
| API 频率限制，离线不可用 | 本地 SQLite 秒开 |

---

## 3. 数据模型

### 3.1 表结构（从飞书完整映射）

#### 案件表 `cases`

```sql
CREATE TABLE cases (
  id              TEXT PRIMARY KEY,
  track           TEXT NOT NULL,              -- ★ 案件轨道: civil_tort / admin_litigation / patent_invalidation
  case_name       TEXT NOT NULL,              -- 案件信息
  case_no         TEXT,                       -- 案号
  cause_action    TEXT,                       -- 案由（单选：专利无效/侵权/行政等10项）
  internal_no     TEXT,                       -- 内部卷号
  jinzhuli_no     TEXT,                       -- 金助理案号
  case_status     TEXT,                       -- 案件状态（公式计算：已完结/进行中/未知）
  case_progress   TEXT,                       -- 案件进展（多选14项）
  case_level      TEXT,                       -- 审级（单选：一审/二审/再审等5项）
  case_result     TEXT,                       -- 案件结果（单选：胜诉/败诉/撤诉等7项）
  
  -- 当事人
  client_name     TEXT NOT NULL,              -- 客户名称
  our_role        TEXT,                       -- 我方诉讼地位
  opponent_name   TEXT NOT NULL,              -- 对方名称
  opponent_role   TEXT,                       -- 诉讼地位（10项）
  opponent_firm   TEXT,                       -- 对方代理律所
  opponent_agent  TEXT,                       -- 对方代理人
  
  -- 审理
  court           TEXT,                       -- 审理机关（单选14项）
  judge_panel     TEXT,                       -- 合议庭（Lookup→庭审信息）
  clerk           TEXT,                       -- 书记员|助理
  attorneys       TEXT,                       -- 办案人（多选）
  
  -- 专利
  patent_name     TEXT,                       -- 专利名称
  patent_app_no   TEXT,                       -- 专利申请号
  procedure_type  TEXT,                       -- 诉讼程序（单选：普通/简易）
  
  -- 日期里程碑
  filing_date     TEXT,                       -- 立案
  complaint_received_date TEXT,               -- 收到起诉状时间
  xieyi_item      TEXT,                       -- 管辖异议
  trial_date      TEXT,                       -- 开庭|口审
  trial2_date     TEXT,                       -- 二次开庭|口审
  trial3_date     TEXT,                       -- 三次开庭丨口审
  verdict_type    TEXT,                       -- 收到判决/裁定/决定类型（6项）
  verdict_date    TEXT,                       -- 收到判决/裁定/决定时间
  stay_date       TEXT,                       -- 裁定中止日
  relief_deadline TEXT,                       -- 救济期限
  
  -- 专利无效专属
  petitioner_first_invalid TEXT,              -- 请求人首次无效时间（公式）
  petitioner_supp_deadline TEXT,              -- 请求人补充意见期限（公式）
  petitioner_submit_date   TEXT,              -- 请求人提交补充意见时间
  petitioner_received_date TEXT,              -- 请求人收到专利权人意见时间
  petitioner_reply_deadline TEXT,             -- 请求人答复意见期限（公式）
  patentee_received_date   TEXT,              -- 专利权人收到受通时间
  patentee_statement_deadline TEXT,           -- 专利权人陈述意见期限（公式）
  patentee_received_supp_date TEXT,           -- 专利权人收到补充意见时间
  patentee_supp_deadline TEXT,                -- 专利权人补充意见时间（公式）
  patentee_submit_supp_date TEXT,             -- 专利权人提交补充意见时间
  
  -- 期限（公式计算，本地引擎实时算，不存库）
  dapian_deadline TEXT,                       -- 提交答辩状期间
  estimated_limit TEXT,                       -- 预估审限
  
  -- 进度
  completed_text  TEXT,                       -- 已完成
  notes           TEXT,                       -- 备注
  
  created_at      TEXT DEFAULT (datetime('now')),
  updated_at      TEXT DEFAULT (datetime('now'))
);
```

#### 办案日志 `case_logs`

```sql
CREATE TABLE case_logs (
  id            TEXT PRIMARY KEY,
  case_id       TEXT REFERENCES cases(id) ON DELETE CASCADE,
  event_summary TEXT NOT NULL,               -- 事件概述
  event_name    TEXT,                        -- 事件名称
  event_type    TEXT NOT NULL,               -- 类型（单选：任务/交文/收文/记录）
  event_date    TEXT NOT NULL,               -- 发生时间
  content       TEXT,                        -- 操作内容
  files_json    TEXT,                        -- 附件（JSON: [{name, url}])
  created_at    TEXT DEFAULT (datetime('now'))
);
```

#### 庭审信息 `hearings`

```sql
CREATE TABLE hearings (
  id              TEXT PRIMARY KEY,
  case_id         TEXT REFERENCES cases(id) ON DELETE CASCADE,
  hearing_record  TEXT NOT NULL,             -- 开庭记录
  hearing_name    TEXT,                      -- 开庭名称（口审/开庭/二次开庭等）
  hearing_date    TEXT NOT NULL,             -- 开庭时间
  venue           TEXT,                      -- 开庭地点
  attendees       TEXT,                      -- 出庭人员
  judges          TEXT,                      -- 审判人员（多选，59个选项）
  court           TEXT,                      -- 审理机关（Lookup→案件主表）
  case_level      TEXT,                      -- 审级（Lookup→案件主表）
  contact_info    TEXT,                      -- 联系方式（Lookup→官方人员联系方式）
  actual_status   TEXT,                      -- 实际开庭情况（已开/未开）
  status          TEXT,                      -- 状态（公式：已开/待开）
  files_json      TEXT,                      -- 附件
  created_at      TEXT DEFAULT (datetime('now'))
);
```

#### 任务管理 `tasks`

```sql
CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  case_id       TEXT REFERENCES cases(id) ON DELETE CASCADE,
  task_name     TEXT NOT NULL,               -- 任务名称
  description   TEXT,                        -- 任务详细描述
  created_date  TEXT NOT NULL,               -- 创建日期
  deadline      TEXT,                        -- 截止日期
  priority      TEXT,                        -- 优先级（四象限）
  completed     INTEGER DEFAULT 0,           -- 完成状态
  assignee      TEXT,                        -- 任务执行人
  countdown     TEXT,                        -- 距离截止日（公式）
  finish_note   TEXT,                        -- 完结记录
  created_at    TEXT DEFAULT (datetime('now'))
);
```

#### 官方人员联系方式 `officials`

```sql
CREATE TABLE officials (
  id              TEXT PRIMARY KEY,
  name            TEXT,                      -- 姓名
  role            TEXT NOT NULL,             -- 身份（法官/法官助理/书记员/法院）
  court           TEXT NOT NULL,             -- 所属机关（13个法院）
  contact_detail  TEXT NOT NULL,             -- 具体联系方式
  contact_text    TEXT,                      -- 联系方式
  contact_record  TEXT,                      -- 联系记录
  created_at      TEXT DEFAULT (datetime('now'))
);
```

#### 关联表（自引用 + 多对多关联）

```sql
CREATE TABLE case_links (
  id            TEXT PRIMARY KEY,
  case_id       TEXT REFERENCES cases(id),
  linked_case_id TEXT REFERENCES cases(id),
  UNIQUE(case_id, linked_case_id)
);

CREATE TABLE case_officials (
  case_id       TEXT REFERENCES cases(id),
  official_id   TEXT REFERENCES officials(id),
  PRIMARY KEY (case_id, official_id)
);
```

### 3.2 表间引用关系（与飞书完全对齐）

```
案件主表 ──事件记录──→  办案日志（1:N）    办案日志.案件名称 ←── 双向关联
案件主表 ──开庭记录──→  庭审信息（1:N）    庭审信息.案件信息 ←── 双向关联
案件主表 ──官方人员──→  官方人员（M:N）    官方人员.关联案件 ←── 双向关联
案件主表 ──关联案件──→  案件主表（自引用）  case_links 单向关联
任务管理 ──关联项目──→  案件主表（N:1）    任务管理.关联项目 ←── 单向关联

Lookup 字段：
  办案日志.案号        → 案件主表.案号
  庭审信息.案号        → 案件主表.案号
  庭审信息.审理机关     → 案件主表.审理机关
  庭审信息.审级        → 案件主表.审级
  庭审信息.联系方式     → 官方人员联系方式.联系方式
  案件主表.合议庭      → 庭审信息.审判人员
  案件主表.未来开庭     → 庭审信息.开庭时间（筛选：开庭时间 > TODAY）
  案件主表.最近已开庭   → 庭审信息.开庭时间（筛选：开庭时间 < TODAY）
```

### 3.3 期限规则表（声明式配置，替代硬编码公式）

**核心设计决策**：不用 if/else 逻辑判断"案由=专利无效 → 计算A"、"案由=专利侵权 → 计算B"。而是用声明式 JSON 规则，每种案件轨道有自己的一套期限配置。新增案件类型时只加配置，不写代码。

```sql
CREATE TABLE deadline_rules (
  id            TEXT PRIMARY KEY,
  track         TEXT NOT NULL,               -- 适用轨道: civil_tort / admin_litigation / patent_invalidation
  trigger_field TEXT NOT NULL,               -- 触发字段: filing_date / complaint_received_date / petitioner_submit_date ...
  offset_days   INTEGER NOT NULL,            -- 偏移天数
  offset_unit   TEXT DEFAULT 'day',          -- day / workday / month
  rule_name     TEXT NOT NULL,               -- 期限名称: "补充意见期限" / "答辩状期间"
  priority      INTEGER DEFAULT 0,           -- 排序优先级
  condition     TEXT,                        -- 额外条件 (JSON): {"cause_action": "专利无效"}
  created_at    TEXT DEFAULT (datetime('now'))
);
```

**三种轨道的期限规则预设**：

| track | 触发字段 | 偏移 | 单位 | 规则名称 |
|---|---|---|---|---|
| patent_invalidation | petitioner_submit_date | 1 | month | 请求人补充意见期限 |
| patent_invalidation | petitioner_received_date | 1 | month | 请求人答复意见期限 |
| patent_invalidation | patentee_received_date | 1 | month | 专利权人陈述意见期限 |
| patent_invalidation | patentee_received_supp_date | 1 | month | 专利权人补充意见时间 |
| patent_invalidation | filing_date | 150 | day | 预估审限（无效） |
| admin_litigation | complaint_received_date | 15 | workday | 提交答辩状期间 |
| admin_litigation | filing_date | 3 | month | 预估审限（简易） |
| admin_litigation | filing_date | 6 | month | 预估审限（普通） |
| civil_tort | complaint_received_date | 15 | workday | 提交答辩状期间 |
| civil_tort | filing_date | 6 | month | 预估审限 |
| 全部 | verdict_date | 10 | day | 救济期限 |

所有期限由公式引擎统一计算，计算结果不存库，每次查询实时算。

### 3.4 案件关系网络表

同一专利可能在国知局做无效宣告、在北京知产法院做侵权诉讼、在最高法做行政上诉——三案并行。用关系表建模：

```sql
CREATE TABLE case_relations (
  id              TEXT PRIMARY KEY,
  source_case_id  TEXT REFERENCES cases(id),
  target_case_id  TEXT REFERENCES cases(id),
  relation_type   TEXT NOT NULL,             -- same_patent / same_party / cross_reference / appeal_of
  label           TEXT,                      -- 用户自定义标签: "244号专利系列"
  created_at      TEXT DEFAULT (datetime('now')),
  UNIQUE(source_case_id, target_case_id, relation_type)
);
```

### 3.5 数据导入

从 `docs/feishu-base/feishu-full-dump.json` 一次性导入。飞书 API 数据字段已包含 record_id，直接映射到本地 id。

---

## 4. 公式引擎

模板参考：飞书表格有 8 个公式字段，本地改为声明式期限规则表（§3.3）+ 实时计算公式引擎。核心函数：

```
src-tauri/src/formula.rs
├── evaluate(expr, context) → String
├── fn if_cond(cond, then, else_) → String
├── fn workday(date, offset) → String       // 工作日偏移
├── fn edate(date, months) → String         // N个月后
├── fn today() → String
├── fn isblank(v) → bool
└── fn and(...conditions) / or(...conditions) → bool
```

### 4.1 公式 → Rust 映射表

| 飞书公式 | 触发条件 | Rust 逻辑 |
|---|---|---|
| **案件状态** | 始终 | `IF(案件结果 IN [结案,胜诉,败诉,对方撤案], "已完结", IF(ISBLANK(案件结果), "未知", "进行中"))` |
| **提交答辩状期间** | 案由≠专利无效 且 收到起诉状非空 | `WORKDAY(收到起诉状+14, 1)` |
| **预估审限** | 立案非空 | `IF(案由=专利无效, 立案+5×30d, IF(诉讼程序=简易, EDATE(立案,3), EDATE(立案,6)))` |
| **请求人首次无效时间** | 案由=专利无效 | `立案` |
| **请求人补充意见期限** | 案由=专利无效 且 请求人提交补充意见非空 | `EDATE(请求人提交补充意见时间, 1) 工作日调整` |
| **请求人答复意见期限** | 案由=专利无效 且 请求人收到专利权人意见非空 | `EDATE(请求人收到专利权人意见时间, 1) 工作日调整` |
| **专利权人陈述意见期限** | 案由=专利无效 且 专利权人收到受通非空 | `EDATE(专利权人收到受通时间, 1) 工作日调整` |
| **专利权人补充意见时间** | 案由=专利无效 且 专利权人收到补充意见非空 | `EDATE(专利权人收到补充意见时间, 1) 工作日调整` |
| **距离截止日** | 任务未完成 且 截止日非空 | `IF(TODAY() <= 截止日, "🕑还有{截止日-TODAY()}天到期", "⁉️已延期")` |

### 4.2 期限预警机制

每天早上 9:00 计算所有活跃案件的期限，生成预警列表，按紧迫度排序：

```rust
fn generate_deadline_warnings(conn: &Connection) -> Vec<DeadlineWarning> {
    let today = today();
    let cases = get_active_cases(conn);
    let mut warnings = vec![];
    for case in cases {
        for (deadline_name, deadline_date) in evaluate_all_deadlines(&case) {
            let days_left = days_between(today, deadline_date);
            warnings.push(DeadlineWarning {
                case_name: case.case_name,
                deadline_type: deadline_name,
                due_date: deadline_date,
                days_left,
                urgency: classify_urgency(days_left),
            });
        }
    }
    warnings.sort_by_key(|w| w.days_left);
    warnings
}
```

---

## 5. 飞书同步引擎

### 5.1 设计原则

1. **飞书是"展示层"，本地是"业务层"**。复杂计算（期限/关系网络/AI 生成）在本地完成，结果推送回飞书。
2. **能用飞书 API 写的字段才同步**，公式字段（`案件状态`/`预估审限` 等）只读不写。
3. **冲突按时间戳裁决**，不做三方合并。
4. **同步是异步的**，不阻塞 UI，失败时降级为标记冲突、等用户手动处理。

### 5.2 同步模型

```
┌──────────────────────┐         ┌──────────────────────┐
│   project-flyshu     │   PUSH  │   飞书多维表格        │
│   (SQLite 本地)       │ ←────→ │   (Bitable API)      │
│                      │   PULL  │                      │
│   ┌─────────────┐    │         │   公式字段(只读)       │
│   │ sync_map    │    │         │   附件需上传         │
│   │ sync_queue  │    │         │   限流 100次/秒/应用  │
│   └─────────────┘    │         └──────────────────────┘
└──────────────────────┘
```

### 5.3 映射表

```sql
-- 记录本地 ID 和飞书 record_id 的对应关系
CREATE TABLE sync_map (
  id              TEXT PRIMARY KEY,
  local_table     TEXT NOT NULL,             -- cases / case_logs / hearings / tasks / officials
  local_id        TEXT NOT NULL,
  feishu_record_id TEXT,                     -- 飞书的 rectXXXXXXXX
  feishu_table_id TEXT,                      -- 飞书的 tblXXXXXXXX
  local_updated   TEXT,                      -- 本地最后修改时间
  feishu_updated  TEXT,                      -- 飞书最后修改时间（PULL 时更新）
  sync_status     TEXT DEFAULT 'synced',     -- synced / local_newer / feishu_newer / conflict / push_failed
  conflict_fields TEXT,                      -- 冲突字段列表（JSON array）
  last_synced_at  TEXT,
  UNIQUE(local_table, local_id)
);

CREATE INDEX idx_sync_status ON sync_map(sync_status);
```

### 5.4 同步队列

同步操作不直接执行，而是入队 → 定时消费 → 失败重试：

```sql
CREATE TABLE sync_queue (
  id            TEXT PRIMARY KEY,
  direction     TEXT NOT NULL,               -- push / pull
  local_table   TEXT NOT NULL,
  local_id      TEXT,
  feishu_table  TEXT,
  feishu_record TEXT,
  payload_json  TEXT,                        -- 要推送的字段值
  attempts      INTEGER DEFAULT 0,
  max_attempts  INTEGER DEFAULT 3,
  last_error    TEXT,
  status        TEXT DEFAULT 'pending',      -- pending / processing / done / failed
  created_at    TEXT DEFAULT (datetime('now'))
);
```

### 5.5 Rust 侧 API 设计

```rust
// src-tauri/src/sync.rs

/// 从飞书 PULL 所有表的更新记录
pub fn pull_all(conn: &Connection, token: &str, app_token: &str) -> Result<SyncReport>

/// PUSH 本地修改到飞书（只推 sync_status != 'synced' 的记录）
pub fn push_all(conn: &Connection, token: &str, app_token: &str) -> Result<SyncReport>

/// PUSH 单条记录（新建/修改后立即调用）
pub fn push_record(
    conn: &Connection,
    token: &str,
    app_token: &str,
    local_table: &str,
    local_id: &str,
) -> Result<()>

/// 检测冲突：比较本地 updated_at 和飞书 modified_time
pub fn detect_conflicts(conn: &Connection) -> Vec<Conflict>
```

**PUSH 流程**：

```
1. 查询 sync_map WHERE sync_status = 'local_newer'
2. 对每条记录：
   a. 从本地 SQLite 读取完整记录
   b. 字段名转换为飞书 field_name
   c. 字段值转换为飞书 API 格式（日期→timestamp, 多选→array, 文件→attachment token）
   d. POST/PUT 飞书 API
   e. 成功 → 更新 sync_map.sync_status = 'synced', feishu_updated = now
   f. 失败 → 记录 error, attempts+1, 超3次 → status='failed'
3. 返回 SyncReport { pushed, failed, conflicts }
```

**PULL 流程**：

```
1. 对每张表：
   a. GET 飞书 API records?page_size=200
   b. 对每条飞书记录：
      - 查 sync_map 是否有对应 local_id
      - 无 → 新建本地记录 + sync_map
      - 有 → 比较 feishu_updated vs local_updated
        - 飞书更新 → 覆盖本地
        - 本地更新 → 标记 conflict
        - 同步 → 跳过
2. 返回 SyncReport { new, updated, conflicted, skipped }
```

### 5.6 冲突处理规则

```
本地 updated_at > 飞书 modified_time  → 本地胜出，标记 push 队列
飞书 modified_time > 本地 updated_at  → 飞书胜出，覆盖本地
两者同时（秒级）→ 字段级对比：
  - 不同字段被修改 → 自动合并（各取各的）
  - 同一字段被修改 → 标记 conflict_fields，UI 提示用户手动选择
```

### 5.7 不能同步的内容

| 飞书字段类型 | 可 PULL | 可 PUSH | 备注 |
|---|---|---|---|
| Text/Number/Select/Checkbox/User | ✅ | ✅ | 直接映射 |
| DateTime | ✅ | ✅ | 两边都是毫秒时间戳 |
| Attachment | ✅ | ⚠️ | PUSH 需先上传文件到飞书，获取 file_token 再关联 |
| Formula | ✅ | ❌ | 只读 |
| Lookup | ✅ | ❌ | 只读，由源表驱动 |
| DuplexLink | ✅ | ⚠️ | PUSH 需传 record_id 数组 |
| Button | ❌ | ❌ | 不可读写 |
| AutoNumber | ✅ | ❌ | 只读 |
| CreatedBy/ModifiedBy | ✅ | ❌ | 只读 |

### 5.8 限流与重试

飞书 API 限流：**100 次/秒/应用**。本地队列处理器：

```rust
struct SyncRateLimiter {
    max_per_second: u32,
    last_batch_time: Instant,
    tokens: u32,
}

impl SyncRateLimiter {
    fn acquire(&mut self) -> bool {
        // Token bucket: 每秒补充 max_per_second 个 token
        // 消耗一个 token 才能发一次请求
        // 不够 → 等待或跳过
    }
}
```

### 5.9 同步触发时机

| 触发方式 | 时机 |
|---|---|
| **即时 PUSH** | 本地新建/修改/删除记录后 5 秒内异步推送 |
| **定时 PULL** | 应用启动时 + 每 15 分钟后台拉一次 |
| **手动触发** | UI 按钮"立即同步"，显示进度条 |
| **周期性全量校验** | 每周一次静默 PULL 全部数据，校验本地没有漏的记录 |

### 5.10 用户界面

```
设置页 → 飞书同步
┌──────────────────────────────────────────────┐
│ 🔗 飞书同步                                      │
│                                                │
│ 状态：✅ 已连接（上次同步：2025-07-20 10:35）       │
│                                                │
│ ┌──────────┬──────────┬──────────┐            │
│ │ PUSH 待推  │ PULL 待拉  │ 冲突      │            │
│ │    3 条   │    0 条   │   1 条    │            │
│ └──────────┴──────────┴──────────┘            │
│                                                │
│ [立即同步]  [查看冲突]  [同步日志]                │
│                                                │
│ 同步规则：                                      │
│  ☑ 案件主表  ☑ 办案日志  ☑ 庭审信息              │
│  ☑ 任务管理  ☑ 官方人员联系方式                   │
│  ☐ 仅在 WiFi 下同步                             │
└──────────────────────────────────────────────┘
```

---

## 6. UI 设计

```
src/modules/case-dashboard/
├── index.js
├── views/
│   ├── DashboardHome.vue        # 首页面板
│   ├── CaseListView.vue         # 案件列表（分组+筛选）
│   ├── CaseDetailView.vue       # 案件详情（聚合视图）
│   ├── CaseTimeline.vue         # 案件时间线
│   ├── CaseNetworkView.vue      # 关联网络
│   ├── CalendarView.vue         # 全局日历
│   ├── DeadlinePanel.vue        # 期限预警面板
│   ├── TasksView.vue            # 任务四象限
│   └── OfficialsView.vue        # 官方人员通讯录
└── composables/
    ├── useCaseStore.js           # Pinia store
    ├── useDeadlines.js           # 期限计算
    └── useCalendar.js            # 日历逻辑
```

### 5.1 首页面板 `DashboardHome`

```
┌─────────────────────────────────────────────────────────────┐
│  📊 案件总览                                    2025年7月20日 │
│  59件 · 39在进行中 · 14已完结                                │
│                                                             │
│  ┌──────────────┬──────────────┬──────────────┬───────────┐ │
│  │ ⚠️ 期限预警   │ 📅 近期开庭   │ 📋 待办任务   │ 👤 联系人  │ │
│  │              │              │              │           │ │
│  │ 隆基244 7天  │ 3/5 隆基口审  │ 准备起诉状    │ 法官A      │ │
│  │ 钛金 14天    │ 3/12 威灵开庭 │ 整理证据清单  │ 书记员B    │ │
│  │ 威灵 -3天⚠️  │ 3/18 钛金口审 │ 联系当事人    │           │ │
│  └──────────────┴──────────────┴──────────────┴───────────┘ │
│                                                             │
│  📈 案件分布                                                 │
│  ████████████░░░░ 专利无效 24件                              │
│  ██████░░░░░░░░░░ 专利侵权 11件                              │
│  ████░░░░░░░░░░░░ 专利行政  9件                              │
│  ██░░░░░░░░░░░░░░ 其他     15件                              │
│                                                             │
│  🗺️ 审理机关分布                                             │
│  国知局 24 · 北京知产 10 · 最高法 8 · 其他 17                 │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 案件列表 `CaseListView`

**分组维度**（自由切换）：按案由 / 按客户 / 按审级 / 按审理机关

**列**（可配置显示/隐藏）：
```
☑ 案件信息  ☑ 案号  ☑ 客户名称  ☑ 对方名称  ☑ 案由
☑ 审理机关  ☑ 审级  ☑ 案件进展  ☑ 预估审限  ☑ 立案
☐ 专利名称  ☐ 开庭日期  ☐ 案件结果  ☐ 办案人
```

**筛选器**：案由(pull-down) / 案件进展(pull-down) / 审级(pull-down) / 审理机关(pull-down) / 客户名称(pull-down/search) / 立案日期(date range)

**行颜色**：案件状态 = 已完结 → 灰色；5 天内有期限 → 红色；15 天内 → 黄色

### 5.3 案件详情 `CaseDetailView`

三栏布局：

```
┌─ 左栏（基本信息）──────────┬─ 中栏（时间线）──────────┬─ 右栏（关联）────┐
│                            │                         │                 │
│ 案件信息                    │ 📅 2024-06-15 立案      │ 关联案件        │
│ 案号: (2024)京73行初1号     │                        │ ├ 二审案        │
│ 案由: 专利无效              │ 📤 2024-08-20 交文     │ ├ 侵权关联案    │
│ 内部卷号: INV-2024-001      │    提交无效宣告请求书    │ └ 行政关联案    │
│                            │                        │                 │
│ 当事人                      │ 📥 2024-09-10 收文     │ 同一客户        │
│ 客户: 隆基绿能              │    收到专利权人陈述意见   │ ├ 隆基244号侵权 │
│ 我方: 专利权人              │                        │ ├ 隆基46号无效  │
│ 对方: 晶科能源 (被告)       │ 📅 2024-10-15 无效口审  │ └ 隆基8号无效   │
│ 对方代理: XX律师事务所      │                        │                 │
│                            │ 📥 2024-11-01 收文     │ 同一专利(244号) │
│ 审理                        │    收到无效决定书        │ ├ 244号无效     │
│ 审理机关: 国知局             │                        │ └ 244号侵权     │
│ 合议庭: 张法官, 李审查员    │ ✅ 2024-12-01 已完结    │                 │
│ 书记员: 王书记员            │                        │ 联系人          │
│                            │                        │ 张法官          │
│ 案件进展                    │                        │ 王书记员        │
│ ████████░░░░░░ 审理中      │                        │                 │
│ 立案 → 口审 → 等决定 → 已完结│                        │                 │
└────────────────────────────┘                         └─────────────────┘
```

### 5.4 日历视图 `CalendarView`

```
     2025年3月
 日  一  二  三  四  五  六
                        1
 2   3   4   5   6   7   8
          🔵隆基口审
 9  10  11  12  13  14  15
     🔴威灵开庭  🔵钛金口审
16  17  18  19  20  21  22
               🟡隆基二审
23  24  25  26  27  28  29
30  31

🔵 无效口审（国知局）    🔴 法院开庭    🟡 二审    📌 任务截止
```

点击任意日期展开当日事件列表，点击事件跳转到案件详情。

### 5.5 期限预警面板 `DeadlinePanel`

三层排序：

| 级别 | 条件 | 颜色 |
|---|---|---|
| 🔴 紧急 | 剩余 ≤ 3 天，或已过期 | 红色 |
| 🟡 注意 | 剩余 4-14 天 | 黄色 |
| 🟢 安全 | 剩余 > 14 天 | 绿色 |

```
🔴 已过期 / 3天内
┌──────────────┬──────────────┬───────────┬──────────┐
│ 案件           │ 期限类型       │ 截止日期    │ 剩余      │
│ 隆基244号无效  │ 补充意见期限   │ 2025-07-13 │ -7天 ⚠️  │
│ 威灵3426号行政 │ 答辩状期间     │ 2025-07-18 │ -2天 ⚠️  │
└──────────────┴──────────────┴───────────┴──────────┘

🟡 4-14天
┌──────────────┬──────────────┬───────────┬──────────┐
│ 钛金专利无效   │ 陈述意见期限   │ 2025-07-30 │ 10天      │
└──────────────┴──────────────┴───────────┴──────────┘
```

### 5.6 任务四象限 `TasksView`

```
          紧急不重要            │           重要紧急
     ┌──────────────┐          │     ┌──────────────┐
     │ □ 打印案卷材料 │          │     │ □ 准备起诉状  │
     │ □ 更新联系人  │          │     │ □ 提交补充意见│
     └──────────────┘          │     └──────────────┘
  ──────────────────────────────┼──────────────────────────────
          不重要不紧急          │           重要不紧急
     ┌──────────────┐          │     ┌──────────────┐
     │ □ 归档已结案  │          │     │ □ 整理证据清单│
     │              │          │     │ □ 研究类案    │
     └──────────────┘          │     └──────────────┘
```

---

## 7. 提醒系统

### 6.1 提醒规则

| 触发类型 | 提醒时机 | 提醒内容 |
|---|---|---|
| 开庭 | 开庭前 1 天 / 当天早 7:00 | "明天 9:30 隆基244号口审，国知局" |
| 期限 | 到期前 7 天 / 3 天 / 当天 | "隆基244号补充意见期限 7 天后到期" |
| 任务 | 截止日当天 7:00 | "今日待办：提交起诉状" |

### 6.2 实现

Tauri 插件 `tauri-plugin-notification`，不需要第三方推送服务：

```rust
fn schedule_reminders(conn: &Connection) {
    let warnings = generate_deadline_warnings(conn);
    for w in warnings {
        if w.days_left <= 7 {
            schedule_notification(
                &w.case_name,
                &format!("{}: {} 天后到期", w.deadline_type, w.days_left),
                w.due_date,
            );
        }
    }
}
```

---

## 8. Skill 工作流平台

### 7.1 核心思想

> Skill = 提示词模板 + 工作流步骤 + 知识库 + 模板调用

用户说的对：如果 token 无限，AI 最终能实现一切。但在 token 有限的情况下，Skill 的价值是把**重复的法律工作流程标准化为可复用的提示词组合**。

### 7.2 Skill 模型

```sql
CREATE TABLE skills (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,              -- 专利无效答辩状生成
  category      TEXT,                       -- 文书生成 / 证据整理 / 法律研究
  description   TEXT,
  workflow_json TEXT,                        -- 步骤定义（见 7.3）
  template_id   TEXT,                       -- 关联 Docsy 模板（可选）
  knowledge_ids TEXT,                        -- 关联知识库条目（逗号分隔）
  created_at    TEXT DEFAULT (datetime('now'))
);
```

### 7.3 AI "断点确认"工作流

参考 **legal-case-report** 项目的人机协作模式：AI 生成的法律文书在三个节点上需要人类律师确认后才进入下一步。避免 AI 全自动生成导致错误文书。

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────────┐
│ 立场锚定      │    │ 内容生成      │    │ 结论审核      │    │ 输出      │
│ AI 提取案件   │───→│ AI 生成文书   │───→│ AI 自查 +    │───→│ 生成 docx│
│ 信息 → 人类   │    │ 草稿 → 人类   │    │ 标注问题 →   │    │          │
│ 确认 ✓       │    │ 确认 ✓       │    │ 人类终审 ✓   │    │          │
└─────────────┘    └─────────────┘    └─────────────┘    └──────────┘
      断点1             断点2              断点3
```

### 7.4 Skill 定义格式（JSON）

```json
{
  "name": "专利无效宣告请求书生成",
  "steps": [
    {
      "order": 1,
      "title": "收集案件信息",
      "ai_prompt": "从案件「{case_name}」中提取以下信息：案件编号、专利名称、专利申请号、请求人名称、专利权人名称、审理机关",
      "source": "case_data",
      "variable": "case_context"
    },
    {
      "order": 2,
      "title": "检索无效证据",
      "ai_prompt": "根据专利「{patent_name}」（申请号{patent_app_no}），检索以下在先技术：\n1. 申请日前的公开专利\n2. 申请日前的公开文献\n3. 申请日前的公开使用\n请列出至少3个对比文件",
      "source": "ai_search",
      "variable": "prior_art"
    },
    {
      "order": 3,
      "title": "生成请求书",
      "ai_prompt": "根据以下信息生成专利无效宣告请求书：\n\n案件信息：{case_context}\n在先技术：{prior_art}\n\n要求：\n1. 格式符合专利法实施细则第65条\n2. 逐条对比权利要求与在先技术\n3. 引用具体对比文件及段落",
      "source": "ai_generate",
      "template_id": "patent_invalidation_request",
      "variable": "document_content"
    },
    {
      "order": 4,
      "title": "填充模板",
      "ai_prompt": "将以下内容填入模板「{template_name}」中的对应字段",
      "source": "docsy_template",
      "template_id": "patent_invalidation_request",
      "fields": {
        "case_number": "{case_no}",
        "petitioner": "{client_name}",
        "patentee": "{opponent_name}",
        "patent_name": "{patent_name}",
        "content": "{document_content}"
      }
    }
  ]
}
```

### 7.4 知识库

```sql
CREATE TABLE knowledge_items (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  category      TEXT,                       -- 法条 / 判例 / 实务指南 / 模板说明
  content       TEXT NOT NULL,
  tags          TEXT,                        -- 逗号分隔
  source_url    TEXT,
  created_at    TEXT DEFAULT (datetime('now'))
);
```

知识库条目在 Skill 步骤中作为 AI 的上下文注入：

```
步骤：生成答辩状
AI 上下文：
  - 知识库：《民事诉讼法》第125条 答辩状要求
  - 知识库：北京知产法院 答辩状格式指南
  - 案件数据：{从案件表提取的当事人/案由/诉讼请求}
```

### 7.6 Skill 预设（三种轨道全覆盖）

**专利无效程序**：
- `patent_invalidation_request` — 专利无效宣告请求书
- `patentee_statement` — 专利权人陈述意见
- `petitioner_supplement` — 请求人补充意见
- `oral_hearing_prep` — 无效口审准备清单

**行政诉讼**：
- `admin_complaint` — 行政起诉状
- `admin_appeal` — 行政上诉状
- `admin_reply` — 行政诉讼答辩状

**民事侵权**：
- `tort_complaint` — 民事起诉状
- `tort_defense` — 民事答辩状
- `evidence_list` — 证据清单
- `pre_trial_brief` — 庭前代理词

**通用**：
- `power_of_attorney` — 授权委托书
- `case_opening_checklist` — 案件立案检查清单
- `jurisdiction_analysis` — 管辖分析

---

## 9. 与 Docsy 的关系

```
project-flyshu                          Docsy
┌─────────────────────┐              ┌──────────────────┐
│ 案件管理             │              │ 模板制作           │
│ 时间线               │   一键生成    │ 模板填写           │
│ 期限计算             │──────────────→│ 文档生成           │
│ 日历提醒             │   skill调用   │ PDF处理            │
│ Skill 工作流         │←──────────────│ 证据页眉页脚       │
│ 知识库               │  模板填充     │ 图片/视频处理      │
└─────────────────────┘              └──────────────────┘
```

**数据流向**：
1. flyshu 案件 → 点击"生成文书" → 调用 Docsy `engine::render_docx`，预填案件字段
2. Docsy 生成 docx 后 → flyshu 记录日志（类型=交文，附件=生成的文件路径）
3. flyshu Skill → 触发 Docsy 模板（通过 `template_id`）

**共用的底层**：
- 都基于 Tauri 2 + Vue 3 + SQLite
- 可以共享 Rust 工具库（文件操作、通知、外部进程）
- 可以作为 Docsy 的一个模块（`src/modules/case-dashboard/`），也可以独立 app

---

## 10. 实施计划

### Phase 1：数据层（1-2天）
- [ ] SQLite 建表（6张表）+ 索引
- [ ] 飞书 JSON 导入脚本
- [ ] Rust CRUD 接口（`src-tauri/src/case_dashboard.rs`）

### Phase 2：同步引擎（1天）
- [ ] `src-tauri/src/sync.rs` — PULL/PUSH/冲突检测
- [ ] `sync_map` + `sync_queue` 表
- [ ] 限流与重试
- [ ] 定时后台同步

### Phase 3：公式引擎（1天）
- [ ] `src-tauri/src/formula.rs` — 8 个公式实现
- [ ] 期限预警计算 + 测试

### Phase 4：核心 UI（2-3天）
- [ ] 案件列表 + 筛选
- [ ] 案件详情（三栏布局）
- [ ] 时间线视图

### Phase 5：日历 + 提醒（1天）
- [ ] 日历视图
- [ ] tauri-plugin-notification 接入

### Phase 6：关联网络 + 任务（1天）
- [ ] 案件关联图（同客户/同专利/双向关联）
- [ ] 任务四象限看板
- [ ] 官方人员通讯录

### Phase 7：Skill 平台（2-3天）
- [ ] Skill CRUD + 预设 10 个 Skill
- [ ] 知识库 CRUD
- [ ] Skill 执行引擎（调用 AI + 填充 Docsy 模板）

---

## 11. 附录：飞书表格完整字段清单

### 案件主表（52 字段）

| # | 字段 | 类型 | 选项/公式 |
|---|---|---|---|
| 1 | 案件信息 | Text | - |
| 2 | 案件状态 | Formula | IF(案件结果 IN[结案/胜诉/败诉/撤案],"已完结",IF(ISBLANK(案件结果),"未知","进行中")) |
| 3 | 案号 | Text | - |
| 4 | 案由 | SingleSelect | 专利无效/专利侵权/技术秘密/著作权权属/专利行政/专利权属/外观侵权/恶意诉讼不正当竞争/商标行政 |
| 5 | 内部卷号 | Text | - |
| 6 | 客户名称 | Text | - |
| 7 | 事件记录 | DuplexLink | ↔ 办案日志 |
| 8 | 未来开庭 | Lookup | 开庭时间 > TODAY |
| 9 | 最近已开庭 | Lookup | 开庭时间 < TODAY |
| 10 | 案件进展 | SingleSelect | 待判决/待再审听证/中止/待口审/待无效决定/待开庭/胜诉/结案/败诉/对方撤案/待补充意见/待立案 |
| 11 | 我方诉讼地位 | Text | - |
| 12 | 专利名称 | Text | - |
| 13 | 专利申请号 | Text | - |
| 14 | 办案人 | MultiSelect | - |
| 15 | 对方名称 | Text | - |
| 16 | 诉讼地位 | SingleSelect | 被告/再审被申请人一审被告/请求人/原告/第三人请求人/上诉人一审原告/原告专利权人/上诉人专利权人/被上诉人一审被告/再审申请人一审原告/请求人/上诉人一审被告/第三人 |
| 17 | 对方代理律所 | Text | - |
| 18 | 对方代理人 | Text | - |
| 19 | 审理机关 | SingleSelect | 北京知产法院/最高法/国知局/黑龙江高院/成都中院/西安中院/苏州中院/哈尔滨中院/济南中院/南昌中院/北京高院/福州中院/四川中院/青岛市中级人民法院 |
| 20 | 合议庭 | Lookup | 庭审信息.审判人员 |
| 21 | 书记员|助理 | Text | - |
| 22 | 审级 | SingleSelect | 一审/再审/二审/结案 |
| 23 | 金助理案号 | Text | - |
| 24 | 立案 | DateTime | - |
| 25 | 管辖异议 | Text | - |
| 26 | 开庭|口审 | DateTime | - |
| 27 | 二次开庭|口审 | DateTime | - |
| 28 | 三次开庭丨口审 | DateTime | - |
| 29 | 收到判决/裁定/决定类型 | SingleSelect | 一审判决/无效决定/裁定/二审判决/和解撤诉/裁驳 |
| 30 | 收到判决/裁定/决定时间 | DateTime | - |
| 31 | 案件结果 | SingleSelect | 胜诉/结案/败诉/对方撤案/胜诉和解/撤诉/解除委托 |
| 32 | 已完成 | Text | - |
| 33 | 备注 | Text | - |
| 34 | 收到起诉状时间 | DateTime | - |
| 35 | 提交答辩状期间 | Formula | IF(案由≠专利无效, WORKDAY(收到起诉状+14,1)) |
| 36 | 裁定中止日 | DateTime | - |
| 37 | 诉讼程序 | SingleSelect | 普通/简易 |
| 38 | 预估审限 | Formula | IF(立案非空,IF(案由=专利无效,立案+150d,IF(普通,EDATE(立案,6),EDATE(立案,3)))) |
| 39 | 救济期限 | Text | - |
| 40 | 请求人首次无效时间 | Formula | 案由=专利无效 → 立案 |
| 41 | 请求人补充意见期限 | Formula | 案由=专利无效 且 请求人补充意见日期非空 → EDATE+1月工作日 |
| 42 | 请求人提交补充意见时间 | DateTime | - |
| 43 | 请求人收到专利权人意见时间 | DateTime | - |
| 44 | 请求人答复意见期限 | Formula | 案由=专利无效 且 收到意见非空 → EDATE+1月工作日 |
| 45 | 专利权人收到受通时间 | DateTime | - |
| 46 | 专利权人陈述意见期限 | Formula | 案由=专利无效 且 受通非空 → EDATE+1月工作日 |
| 47 | 专利权人收到补充意见时间 | DateTime | - |
| 48 | 专利权人补充意见时间 | Formula | 案由=专利无效 且 收到补充意见非空 → EDATE+1月工作日 |
| 49 | 专利权人提交补充意见时间 | DateTime | - |
| 50 | 关联案件 | SingleLink | → 案件主表（自引用） |
| 51 | 开庭记录 | DuplexLink | ↔ 庭审信息 |
| 52 | 官方人员联系方式 | DuplexLink | ↔ 官方人员联系方式 |

### 办案日志（9 字段）

| # | 字段 | 类型 | 选项 |
|---|---|---|---|
| 1 | 事件概述 | Text | - |
| 2 | 案件名称 | DuplexLink | ↔ 案件主表 |
| 3 | 事件名称 | Text | - |
| 4 | 案号 | Lookup | 案件主表.案号 |
| 5 | 发生时间 | DateTime | - |
| 6 | 操作内容 | Text | - |
| 7 | 类型 | SingleSelect | 任务/交文/收文/记录 |
| 8 | 附件 | Attachment | - |
| 9 | 创建任务按钮 | Button | - |

### 庭审信息（15 字段）

| # | 字段 | 类型 | 选项 |
|---|---|---|---|
| 1 | 开庭记录 | Text | - |
| 2 | 案件信息 | DuplexLink | ↔ 案件主表 |
| 3 | 案号 | Lookup | 案件主表.案号 |
| 4 | 开庭名称 | Text | - |
| 5 | 开庭时间 | DateTime | - |
| 6 | 出庭人员 | Text | - |
| 7 | 审理机关 | Lookup | 案件主表.审理机关 |
| 8 | 开庭地点 | Text | - |
| 9 | 审判人员 | MultiSelect | 59 个选项（樊晓东/倪敬涵/刘利芳/潘筝/段若诗/曾凤...） |
| 10 | 状态 | Formula | IF(开庭时间<TODAY(),"已开","待开") |
| 11 | 联系方式 | Lookup | 官方人员联系方式.联系方式 |
| 12 | 审级 | Lookup | 案件主表.审级 |
| 13 | 附件 | Attachment | - |
| 14 | 发送邮件按钮 | Button | - |
| 15 | 实际开庭情况 | SingleSelect | 已开/未开 |

### 任务管理（11 字段）

| # | 字段 | 类型 | 选项 |
|---|---|---|---|
| 1 | 任务名称 | Text | - |
| 2 | 任务详细描述 | Text | - |
| 3 | 创建日期 | DateTime | - |
| 4 | 截止日期 | DateTime | - |
| 5 | 距离截止日 | Formula | IF(未完成,IF(TODAY()≤截止日,"🕑还有X天","⁉️已延期"),"") |
| 6 | 优先级 | SingleSelect | 重要紧急/紧急不重要/重要不紧急/不重要不紧急 |
| 7 | 关联项目 | SingleLink | → 案件主表 |
| 8 | 完成状态 | Checkbox | - |
| 9 | 创建任务 | Button | - |
| 10 | 任务执行人 | User | - |
| 11 | 完结记录 | Text | - |

### 官方人员联系方式（7 字段）

| # | 字段 | 类型 | 选项 |
|---|---|---|---|
| 1 | 姓名 | Text | - |
| 2 | 身份 | SingleSelect | 法官/法官助理/书记员/法院 |
| 3 | 所属机关 | SingleSelect | 13 个法院（北京知产/最高法/国知局/黑龙江高院/成都中院/西安中院/苏州中院/哈尔滨中院/济南中院/南昌中院/北京高院/福州中院/四川中院） |
| 4 | 具体联系方式 | Text | - |
| 5 | 联系记录 | Text | - |
| 6 | 联系方式 | Text | - |
| 7 | 关联案件 | DuplexLink | ↔ 案件主表 |
