# Docsy 案件看板设计

> 基于飞书多维表格 `JYWEb4e0BayQrrsw5Tdct0c3n5g` 的完整分析  
> 数据位置：`docs/feishu-base/`（52+9+15+11+7=94 字段，59+67+55+19+9=209 条记录）

## 0. 飞书表模型

### 0.1 表间引用关系

```
案件主表 ──事件记录↔──→ 办案日志
案件主表 ──开庭记录↔──→ 庭审信息  
案件主表 ──官方人员联系方式↔──→ 官方人员联系方式
案件主表 ──关联案件→──→ 案件主表（自引用）

办案日志 ──案号🔍──→ 案件主表.案号
庭审信息 ──案号🔍──→ 案件主表.案号
庭审信息 ──审理机关🔍──→ 案件主表.审理机关
庭审信息 ──审级🔍──→ 案件主表.审级
任务管理 ──关联项目→──→ 案件主表
```

### 0.2 核心公式业务逻辑

| 公式 | 触发条件 | 计算规则 |
|---|---|---|
| **案件状态** | 案件结果≠空 | 结案/胜诉/败诉/撤案→"已完结"，否则→"进行中" |
| **提交答辩状期间** | 案由≠专利无效 且 收到起诉状时间≠空 | 收到起诉状+15个工作日 |
| **预估审限** | 立案≠空 | 专利无效→立案+5×30天；简易→立案+3个月；普通→立案+6个月 |
| **请求人补充意见期限** | 案由=专利无效 且 请求人提交补充意见时间≠空 | 提交补充意见+1个月（工作日调整） |
| **请求人答复意见期限** | 案由=专利无效 且 请求人收到专利权人意见≠空 | 收到意见+1个月（工作日调整） |
| **专利权人陈述意见期限** | 案由=专利无效 且 专利权人收到受通≠空 | 收到受通+1个月（工作日调整） |
| **专利权人补充意见时间** | 案由=专利无效 且 专利权人收到补充意见≠空 | 收到补充意见+1个月（工作日调整） |
| **距离截止日** | 任务未完成 且 截止日期≠空 | TODAY-截止日→"🕑还有X天"或"⁉️已延期" |

**公式模式总结**：
- `EDATE(date, N) + WORKDAY调整` = 从某日期起 N 个月后的工作日
- `IF(AND(案由="专利无效", NOT(ISBLANK(触发日期))), 计算, "")` = 专利无效案专属期限
- `IF(OR(结果 IN (closed_states)), "已完结", ...)` = 状态聚合

## 1. 设计方案

### 1.1 SQLite 本地模型

```
案件主表 cases
├── id, case_no(案号), case_name(案件信息), internal_no(内部卷号), jinzhuli_no
├── cause_action(案由), case_level(审级), case_status(案件状态), case_progress(案件进展)
├── court(审理机关), judge_panel(合议庭), clerk(书记员|助理), judge_phone(法院电话)
├── client_name(客户名称), our_role(我方诉讼地位)
├── opponent_name(对方名称), opponent_role(诉讼地位), opponent_law_firm, opponent_agent
├── patent_name, patent_app_no, procedure_type(诉讼程序)
├── filing_date(立案), complaint_received_date, dapian_deadline(提交答辩状期间)
├── trial_date(开庭|口审), trial2_date, trial3_date
├── verdict_type, verdict_date, case_result
├── estimated_limit(预估审限), relief_deadline(救济期限)
├── petitioner_first_invalid_date, petitioner_supplement_deadline
├── petitioner_reply_deadline, patentee_statement_deadline, patentee_supplement_deadline
├── stay_date(裁定中止日)
├── notes(备注), completed_text, xieyi_item(管辖异议)
├── attorneys(办案人), created_at, updated_at

办案日志 case_log
├── id, case_id→cases.id
├── event_summary(事件概述), event_name(事件名称), event_type(类型:任务/交文/收文/记录)
├── event_date(发生时间), content(操作内容)
├── files📎(附件路径)

庭审信息 hearings
├── id, case_id→cases.id
├── hearing_record(开庭记录), hearing_name(开庭名称), hearing_date(开庭时间)
├── venue(开庭地点), attendees(出庭人员)
├── judges(审判人员), actual_status(实际开庭情况:已开/未开)
├── status(状态:已开/待开), files📎(附件路径)

任务管理 tasks
├── id, case_id→cases.id
├── task_name, description, created_date, deadline
├── countdown(距离截止日), priority(优先级:四象限)
├── completed(完成状态), assignee(任务执行人), completion_note(完结记录)

官方人员联系方式 officials
├── id, name, role(身份:法官/法官助理/书记员/法院)
├── court(所属机关), contact_detail, contact_record
├── case_id→cases.id
```

### 1.2 公式引擎（Rust 侧）

飞书的 `IF/AND/OR/ISBLANK/EDATE/WORKDAY/TODAY/DATEADD` 在 Rust 侧需要一个轻量公式引擎：

```
src-tauri/src/formula.rs
├── enum FormulaExpr { If(cond, then, else_), And(Vec), Or(Vec), ... }
├── fn evaluate(expr, context: &EvalContext) -> Result<Value>
└── built-in: WORKDAY(date, offset), EDATE(date, months), ISBLANK(v), TODAY()
```

**不需要完整的公式解析器**：SQLite 端的公式通过 `chrono` 库实时计算，不存飞书公式字符串。每条记录的期限字段在插入/更新时触发计算。

### 1.3 日历+提醒

```
src-tauri/src/calendar.rs
├── fn upcoming_events(days: i32) -> Vec<CalendarEvent>
│   └── UNION: 开庭日期 + 任务截止日期 + 预估审限 + 各种意见期限
├── fn events_in_range(from, to) -> Vec<CalendarEvent>
└── 提醒: tauri-plugin-notification 在事件当天 9:00 弹通知
```

日历视图（Vue 前端）：一个可滚动的月视图，每个日期格子里显示案件简称+事件类型。

### 1.4 与 Docsy 模板系统的连接

```
案件详情页 → 点击"生成文书" → 打开模板填充页，预填当事人数据
├── cases.client_name + cases.our_role → template.parties 数组
├── cases.opponent_name + cases.opponent_role → template.parties 数组  
├── cases.case_no + cases.court → template 文书抬头
├── cases.cause_action → template 案由建议
└── cases.trial_date → template 日期字段
```

### 1.5 前端路由规划

```
/cases             → 案件列表（表格+筛选）
/cases/:id         → 案件详情（时间轴+文档列表）
/cases/:id/generate → 从案件生成文书（复用 template 模块）
/calendar          → 全案件日历视图
/tasks             → 任务四象限看板
/officials         → 官方人员通讯录
```

## 2. 实施阶段

### Phase 1: 本地数据模型（1-2天）
- `src-tauri/src/case_dashboard.rs` — SQLite 建表+CRUD
- `src-tauri/src/formula.rs` — WORKDAY/EDATE/TODAY/ISBLANK 实现
- 迁移脚本：飞书 JSON → SQLite

### Phase 2: 案件列表+详情（2-3天）
- `src/modules/case-dashboard/` — Vue 模块
- CaseListView + CaseDetailView
- 案件筛选（按案由/进展/审级/负责人）

### Phase 3: 日历+提醒（1天）
- calendar.rs + CalendarView.vue
- tauri-plugin-notification 接入

### Phase 4: 模板连接（1天）
- 案件→模板数据填充
- 生成文书后自动关联到 `case_docs` 表

### Phase 5: 任务+联系人（1天）
- 任务四象限看板
- 官方人员通讯录

## 3. 不做的

- ❌ 飞书双向同步（先本地独立运行）
- ❌ 完整飞书公式解析器（只实现用到的 8 个函数）
- ❌ 工作流自动化（状态机/审批流/自动交文——那是法院系统的事）
- ❌ 在线协作（单机版先跑通）
