
# **Docsy 产品设计文档（修订版）**

> 修订日期：2026/06/03
> 本文档定位为 Docsy 的**开发指引**，覆盖产品定位、技术栈、模块拆分、UI、数据结构、文件格式、开发路线图。
>
> 本版相对初稿的主要变化：
>
> 1. 把"所函生成"细化为可配置模板生成器，给出完整字段建模与示例。
> 2. 新增"模板制作功能"详细设计：从一份现有 Word 文档出发，圈选文本→打标签→标注字段属性→保存为模板。
> 3. 明确字段属性体系（是否必填、是否多值、可选列表、引用前序字段、字体/下划线保留等）。
> 4. 自制模板可挂载为标签页，与内置"所函"标签页能力对等。
> 5. 增加 PDF 解锁等独立小工具的归位策略。
> 6. **去掉团队同步 / 服务端中枢**，第一阶段定位为**纯本地单机软件**，通过"配置导入导出"实现跨电脑迁移与团队共享。

---

## **1. 产品定位**

**Docsy** 是一款轻量、高效、跨平台的文档处理工具箱，主要面向 Word、PDF 等办公文档场景。

软件初期核心功能是"生成所函"，但实现上不应硬编码为"所函生成器"，而应做成**通用 Word 模板生成器 + 可视化模板制作器**，"所函"只是其中一个内置模板。

后续可扩展为：批量文档生成、PDF 处理、文档转换等功能。

设计目标：

- 跨平台支持 macOS 与 Windows
- 文件体积小，启动快，运行效率高
- 使用 Rust 作为核心实现语言（Tauri 壳）
- 工具箱式 + 标签页式 UI，方便后续增加功能
- 本地记录存储（SQLite）
- 通过**配置文件导入导出**实现模板和数据的跨电脑迁移、备份、团队共享（不依赖任何服务端）

---

## **2. 技术方向**

### **2.1 客户端技术栈**

推荐：**Rust + Tauri + Vue**

理由：

- 比 Electron 更轻量，打包体积更小
- Rust 处理文件 IO、docx XML、模板渲染、SQLite、PDF 调用性能更好
- 前端用 Vue 写 UI，开发速度快
- macOS / Windows 双端覆盖

### **2.2 关键依赖（建议）**

- docx 解析与重写：`docx-rs` 或基于 `quick-xml` 自实现，**直接操作 docx 内部 XML，不要中转 HTML / Markdown / 纯文本**
- PDF 处理（解锁、合并、拆分）：`pdfium-render` 或 `lopdf`
- SQLite：`rusqlite` / `sqlx`
- 序列化：`serde` + JSON（模板定义、记录）
- 前端：Vue 3 + Pinia + 组件库（Element Plus / Naive UI 任选）

---

## **3. 软件整体结构**

Docsy 采用"**工具箱 + 标签页**"形式。

主界面分为：

1. 顶部导航栏（Logo、版本、设置）
2. 左侧功能区（标签页/工具列表）
3. 中间工作区（表单 / 编辑 / 预览）
4. 右侧参数区（字段属性 / 文档预览）
5. 底部状态栏

### **3.1 标签页策略**

标签页分为两类：

- **内置标签页**：所函生成、模板制作、PDF 工具、记录中心、设置等。这些是程序自带功能。
- **用户挂载标签页**：用户用"模板制作"功能做出来的任意模板，都可以"固定到标签页"，从而获得与内置"所函生成"完全一致的使用体验（左侧填表、右侧预览、一键生成、保留历史记录）。

> 设计原则：内置"所函"模板本质上也是一份普通 Docsy 模板，只不过出厂自带、不可删除。这样就保证了"自制模板和内置模板完全对等"。

### **3.2 初期保留的内置标签页**

| 标签页 | 说明 |
|---|---|
| 所函生成 | 出厂内置模板的快捷入口（也可被用户挂载的同类模板补充） |
| 模板制作 | 从 Word 文档生成 Docsy 模板的可视化编辑器 |
| 模板管理 | 列出所有模板，支持编辑、删除、导入导出、固定到标签页 |
| PDF 工具 | 解锁、合并、拆分等 |
| 记录中心 | 历史生成记录、重用上次输入 |
| 设置 | 默认导出方式、字体检查、配置导入导出等 |

> "PDF 解锁"放在 **PDF 工具** 标签页内作为子菜单，同时在首页提供快捷入口。布局上 PDF 工具页采用左侧子菜单（解锁 / 合并 / 拆分 / 压缩 / 转换…），中间主操作区。

---

## **4. 核心范式：通用 Word 模板生成器**

虽然初期目标是生成"所函"，但**功能不写死**。

设计上分两层：

```text
[模板制作器]  →  产出 Docsy 模板（.docsytpl + 原始 .docx）
       ↓
[模板生成器]  →  根据字段表单填值，渲染为新 .docx / .pdf
```

任何用模板制作器做出来的模板，都自动具备和"所函"一样的生成体验。

---

## **5. 模板制作功能（重点新增）**

这是本次完善的核心功能。

### **5.1 设计目标**

- 用户拿到任意一份现有的 `.docx`，都能在 Docsy 内**可视化**地把其中的部分文本"标记"成可变字段。
- 标记时给字段打**标签**（label），用于在生成时显示为表单字段名。
- 字段可以记录"成分（component）"，也可以不记录——"成分"指字段的组成结构（例如所函里的"原告"是一个可重复的人名列表，每一项可以是自然人或公司），用于驱动表单的渲染方式。
- 字段可设置是否必填、是否允许多值、是否引用其他字段值、默认值、可选列表（用于老客户复用）等。
- 保存为 Docsy 模板后，可以"固定到标签页"，使用方式和内置"所函生成"完全一致。

### **5.1.1 统一编辑器原则**

模板制作和模板编辑本质上是同一个 `TemplateEditor`，不应实现为两套互相分叉的流程。

统一编辑器的核心状态是一个 `TemplateEditorSession`：

```text
TemplateEditorSession
  templateId           # 稳定模板身份；新建时可为空，保存时生成
  versionId            # 当前编辑的内部内容版本；新建保存后生成
  manifest             # 名称、类型、版本等元数据
  sourceDocxBase64     # 当前可编辑的 docx 源
  plainText            # 与 sourceDocx 对齐的纯文本
  marks                # 字段标记：start/end/key/label/type/visibility...
  fields               # 生成表单字段定义
  dictionaries         # 模板内置候选值
  builderState         # 制作/编辑经验，用于再次恢复 session
```

两种入口只是初始化方式不同：

| 入口 | 初始化来源 | 初始呈现 |
|---|---|---|
| 制作模板（create） | 用户选择的原始 `.docx` | 原文作为固有字显示，`marks` 为空，用户逐步标记字段 |
| 编辑模板（edit） | 稳定 `templateId` 的当前版本 | 已标记字段显示为 `<<标签>>`，固有字可继续修改、取消标记或重新标记 |

保存逻辑必须统一：无论从 create 还是 edit 进入，保存都是把当前 `TemplateEditorSession` 写成该 `templateId` 的新当前版本，并确保生成页、模板管理、字典和历史读取同一份当前版本。

具体重构执行方案见 `docs/template-editor-refactor.md`。

### **5.2 制作流程**

1. **选择 Word 文件**：用户选择本地 `.docx`。
2. **预览原文**：右侧渲染文档原貌（保留字体、字号、加粗、下划线、段落样式）。
3. **选中文本 → 标记为字段**：用户在预览中选中一段文字，弹出字段属性面板，填写：
   - 字段 key（程序内唯一标识，例如 `court_name`）
   - 字段标签 label（表单显示名，例如"法院"）
   - 字段类型（见 5.3）
   - 是否必填
   - 是否允许多值（例如"原告"可有多人）
   - 是否允许从前序字段复用（例如"委托人姓名"可从前面填过的"原告/被告/第三人"中选）
   - 默认值 / 可选列表（用于老客户复用，下次直接选）
   - 是否保留下划线 / 加粗 / 字体（默认保留原占位文字的样式）
4. **重复 3**，直到所有可变文本都已被标记。
5. **保存模板**：模板包含原始 docx + 字段定义 JSON + 元数据。
6. **可选：固定到标签页** → 模板从此出现在左侧标签页列表里，使用方式与内置所函等同。

### **5.2.1 模板身份与版本规则**

Docsy 的模板身份分为两层：

| 层级 | 名称 | 示例 | 用户是否感知 | 作用 |
|---|---|---|---|---|
| 稳定身份 | `templateId` | `letter`、`tpl_contract` | 是 | 对应用户在软件中看到的那份"模板"，用于菜单、模板管理、生成历史、字典配置和字段配置 |
| 内容版本 | `versionId` | `20260605-101530` 或内部 hash | 否 | 对应该模板某一次编辑后保存的具体内容版本，用于恢复、回退、审计 |

规则：

1. 用户在模板管理中点击"编辑模板"，编辑对象是稳定 `templateId` 当前激活的内容版本。
2. 保存后，新的内容版本成为该 `templateId` 的当前版本；在文件生成、模板管理、字段配置、字典配置、历史记录中必须一致生效。
3. 内置模板（如 `letter`）也遵守同一规则：编辑后不是修改出厂资源，而是在用户数据目录保存一份当前版本。它在体验上取代出厂版本，但必须保留恢复到出厂版本的能力。
4. 用户可见的菜单项、历史记录、模板配置仍使用稳定 `templateId`，不因内部版本变化而改变。
5. 旧版本可归档；当前实现可先以 `user_templates/<templateId>.docsytpl` 表示当前激活版本，后续再扩展为 `templateId/versionId.docsytpl + active.json`。

### **5.2.2 模板编辑器行为**

模板编辑器的目标不是只管理字段配置，而是让用户能直观看到"生成文件的形象"并修改模板文字：

1. **字段字显示**：已标记字段在编辑预览中显示为 `<<标签>>` 或同等按钮化样式。它代表可替换字段，不允许直接当普通文字编辑。
2. **取消标记**：用户对字段点击"取消标记"时，应把该字段恢复成模板固有字。恢复后的文字可继续编辑，也可重新标记为字段。
3. **固有字编辑**：未被标记的普通文本可以编辑。第一阶段只要求文字编辑，不要求插入图片、表格结构调整或复杂 Word 排版。
4. **基础格式工具栏**：对选中的固有字提供轻量邮件编辑器式工具栏：字体、字号、加粗、斜体、下划线。工具栏只作用于固有字选区，不作用于字段按钮。
5. **格式保存边界**：编辑器必须把文字和基础格式反写到当前模板版本的 docx 源结构中。不能只改 mammoth HTML 预览，否则生成文件会与管理页预览不一致。
6. **全局一致性**：保存成功后，当前模板版本应立即成为生成、管理、字典和历史相关功能的唯一读取来源。

### **5.3 字段类型体系**

| 类型 key | 名称 | 说明 | 表单控件 |
|---|---|---|---|
| `text` | 单行文本 | 普通文本，如案号 | input |
| `textarea` | 多行文本 | 长事由 | textarea |
| `date` | 日期 | 系统日期选择 | date picker |
| `select` | 单选 | 从可选列表选 | select |
| `multiselect` | 多选 | 从可选列表多选 | multi-select |
| `party` | 当事人 | 可重复的当事人条目（见 5.4） | party-list |
| `reference` | 引用字段 | 从已填字段中选一个值，也可手输 | combobox |
| `number` | 数字 | 数量、金额 | number input |

### **5.4 当事人字段（party）的特殊设计**

所函这种文书里有多个角色：原告、被告、第三人、委托人。它们具有共同结构：

- 角色（role）：原告 / 被告 / 第三人 / 委托人 / 自定义
- 名称（name）：自然人姓名或公司名
- 主体类型（subject_type）：自然人 / 法人 / 其他组织
- 数量可变：0 个、1 个、N 个
- 在原文里通常**带下划线**

模板制作器允许把一段文字标记为 `party` 字段，并指定其角色。生成表单时会渲染成"+ 添加一项"的列表控件，每项含名称和主体类型。生成文档时按"顿号 + 下划线"的规则拼接，保留原占位的样式。

### **5.5 引用字段（reference）与推荐逻辑**

为了支持"委托人姓名可从前序原告/被告/第三人中选"的场景，以及自动推断委托人身份：

**引用关系**（候选来源）：
- 字段定义里声明 `references: ["plaintiffs", "defendants", "third_parties"]`
- 表单渲染时，候选下拉项 = 上述字段当前已填的所有值的并集
- 同时允许"手动输入"，覆盖下拉

**推断关系**（自动填充）：
- 字段定义里声明 `infer_from: { source_field, mapping }`
- 当 `source_field` 变化时，检查它在哪个字段中，根据 `mapping` 自动填充当前字段

**互斥关系**（不能相同）：
- 字段定义里声明 `exclude: ["lawyers", "court"]`
- 当前字段的候选值排除这些字段的值

**配置示例**（所函模板的 client_name 和 client_role）：
```json
{
  "key": "client_name",
  "label": "委托人名称",
  "type": "reference",
  "references": ["plaintiffs", "defendants", "third_parties"],
  "exclude": ["lawyers", "court"]
},
{
  "key": "client_role",
  "label": "委托人身份",
  "type": "reference",
  "infer_from": {
    "source_field": "client_name",
    "mapping": {
      "plaintiffs": "原告",
      "defendants": "被告",
      "third_parties": "第三人"
    }
  }
}
```

**模板管理标签页**：
- 在模板管理中新增"推荐逻辑"标签页
- 可视化配置引用关系、推断关系、互斥关系
- 内置模板（如所函）的配置也可在此查看和修改

### **5.6 字段属性 JSON 示例**

```json
{
  "key": "plaintiffs",
  "label": "原告",
  "type": "party",
  "required": true,
  "multiple": true,
  "min": 1,
  "default_role": "原告",
  "subject_type_options": ["自然人", "法人", "其他组织"],
  "style": {
    "underline": true,
    "font": "仿宋",
    "size": "三号",
    "bold": false
  },
  "options": [
    { "name": "日本制铁株式会社", "subject_type": "法人" },
    { "name": "浦项股份有限公司", "subject_type": "法人" }
  ],
  "remember_history": true
}
```

`options` 用于"老客户直接选"。`remember_history: true` 表示每次填的值会自动加入候选列表。

### **5.7 标签（label）与"是否记录成分"**

> 用户原话："并且记录了标签的成分也可以有也可以没有。"

含义拆解：

- **标签（label）必须有**：用于表单显示。
- **"成分"可有可无**：成分指字段的内部结构（例如 party 字段的"角色 / 名称 / 主体类型"三段）。简单字段（如案号、日期）不需要成分；复杂字段（如当事人列表）需要成分。

在数据结构上体现为：
- 简单字段：只有 `key / label / type / required` 等基础属性。
- 复杂字段：额外含 `components: [...]` 描述其内部组成。

---

## **6. 所函模板（出厂内置）的完整建模示例**

把用户给出的所函作为示例，把它建模成一份 Docsy 模板。

### **6.1 所函原文**

```text
律师事务所函
{{court}}：
原告{{plaintiffs}}与被告{{defendants}}、第三人{{third_parties}}之间{{cause}}一案（案号：{{case_no}}），{{client_role}}{{client_name}}委托本所{{lawyers}}律师为{{stage}}的诉讼代理人，特此告知。


{{firm_name}}
{{date}}
```

### **6.2 字段定义**

| key | label | type | 必填 | 多值 | 可选列表 | 备注 |
|---|---|---|:-:|:-:|---|---|
| court | 法院 | text | 是 | 否 | 可检索的法院列表 | 一份所函只有一个法院 |
| plaintiffs | 原告 | party | 是 | 是 | 历史当事人 | 至少一个 |
| defendants | 被告 | party | 是 | 是 | 历史当事人 | 至少一个 |
| third_parties | 第三人 | party | 否 | 是 | 历史当事人 | 可以没有 |
| cause | 案由 | select | 是 | 否 | 案由列表（可检索） | 例如"发明专利无效行政纠纷" |
| case_no | 案号 | text | 否 | 否 | — | 不一定有 |
| client_role | 委托人身份 | reference | 是 | 否 | 来自 plaintiffs / defendants / third_parties 的角色 | 也可手输 |
| client_name | 委托人名称 | reference | 是 | 否 | 来自上述当事人名称 | 也可手输 |
| lawyers | 代理律师 | text | 是 | 是 | 律师库 | 多人用顿号 |
| stage | 阶段 | select | 是 | 否 | 一审 / 二审 / 再审 / 执行 / 仲裁… | 默认"一审阶段" |
| firm_name | 律所 | text | 是 | 否 | 默认"北京志霖律师事务所" | 可改 |
| date | 日期 | date | 是 | 否 | 默认今天 | 可改，可留空年月 |

### **6.3 字体与样式规则**

模板制作时，每个占位符的样式从原 docx 中**直接保留**，不需要用户手填。所函示例中：

- 标题"律师事务所函"：黑体 加粗 二号
- 法院名、律所署名、日期行：仿宋 加粗 三号
- 正文其他文字：仿宋 三号 不加粗
- 所有当事人姓名（原告/被告/第三人/委托人姓名）：**带下划线**

实现要点：

- 字段渲染时**继承占位符所在 run 的 rPr**（run properties），不要自己重设字体。
- 当 party 字段被渲染为多个名字（顿号分隔）时，每个名字单独成一个 run，并继承下划线样式；顿号本身不加下划线。
- 标题和署名行的字号/字体来自原模板段落和 run 属性，Docsy 不改写 styles.xml。

### **6.4 渲染示例**

填写：

- court = "北京知识产权法院"
- plaintiffs = ["日本制铁株式会社"]
- defendants = ["国家知识产权局"]
- third_parties = ["浦项股份有限公司", "安赛乐米塔尔公司", "上海蔚来汽车有限公司"]
- cause = "发明专利无效行政纠纷"
- case_no = "（2026）京73行初6547号"
- client_role = "第三人"
- client_name = "浦项股份有限公司"
- lawyers = ["李月春"]
- stage = "一审阶段"
- firm_name = "北京志霖律师事务所"
- date = "2026 年 月 日"

渲染结果即与用户提供的原文一致，且当事人名字保留下划线。

---

## **7. 文档生成流程（通用）**

适用于所有模板（内置所函、用户自制模板）。

### **7.1 步骤**

1. 用户在左侧标签页选中模板（例如"所函生成"或自定义模板）。
2. 中间区域根据模板的 `fields` 自动渲染表单。
3. 表单具备以下能力：
   - 必填校验
   - 老客户记忆：候选下拉来自 `options` + `remember_history` 累积值
   - 引用字段：`reference` 类型自动从已填字段拉取候选
   - 当事人列表：`party` 字段支持"+ 添加一项"、删除、上移下移
4. 右侧实时预览：把当前表单值填入模板，渲染出 docx 预览（前端可用 docx → HTML 的轻量预览，或简化文本预览）。
5. 点击"生成"：
   - Rust 侧打开 `original.docx`，按字段渲染，生成新 `.docx`
   - 如果用户选择"导出 PDF"，调用 PDF 导出策略（见第 14 章）
6. 文件保存到用户选择的位置 / 默认输出目录，同时写入本地记录（SQLite）。

### **7.2 老客户复用体验**

每个 `text` / `party` 字段都可设置 `remember_history: true`。一旦开启：

- 每次生成成功，新值会去重后写入该字段的 `options` 列表
- 下次填写时，该字段下拉里直接出现历史值，鼠标点选即可
- 在所函场景下：法院、案由、原告、被告、第三人、律所、律师都会因此变成"选选就行"

### **7.3 复用上次输入**

记录中心里每条历史记录都可"重新打开"，把当时的全部表单值带回到生成页，用户改几个字段就能再生成一份。

---

## **8. 自制模板挂载到标签页**

### **8.1 思路**

左侧标签页列表 = 内置标签页 + 用户固定到标签页的模板。两者在 UI 上完全等价。

### **8.2 数据来源**

- 内置标签页：硬编码在程序中（所函生成、模板制作、模板管理、PDF 工具、记录中心、设置）。
- 用户挂载标签页：来自模板表 `pinned_to_tab = true` 的记录。

### **8.3 操作入口**

- 模板制作完成时，弹窗询问"是否固定到标签页"。
- 模板管理页中，每条模板支持"固定 / 取消固定"切换。
- 标签页右键菜单支持"取消固定"。

### **8.4 行为对齐**

被挂载的模板在标签页里打开时：

- 左中右布局与所函生成页一致
- 同样支持表单校验、预览、生成、导出 PDF、写记录、复用上次输入

也就是说，"所函生成"标签页本质上就是固定挂载了内置所函模板的实例；用户的自制模板挂上去后获得相同体验。

---

## **9. PDF 工具（含 PDF 解锁）**

### **9.1 现成参考与移植**

用户已有现成的 PDF 解锁工具：`/Users/only/Documents/PythonProgram/crackleaf`，关键信息：

- 实现：Rust + eframe/egui + 调用 `qpdf` 命令
- 跨平台打包方案：macOS 用 `cargo-bundle`，Windows 用 GitHub Actions 构建 + `tools/qpdf.exe`
- 已具备：单文件 / 多文件 PDF 解锁、状态显示、动画反馈、qpdf 自检与缺失提示
- 体量小、独立运行良好

**移植策略**：

- 把 crackleaf 中"调用 qpdf 解锁 PDF"的核心逻辑（`Command::new("qpdf")` 调用、参数拼接、错误处理、qpdf 自检）抽出为 Docsy 内的一个 Rust 模块，例如 `src-tauri/src/pdf/qpdf.rs`。
- crackleaf 当前的 egui UI 不直接复用——Docsy 主体使用 Tauri + Vue（见第 2 章），UI 由前端重写为 PDF 工具页中的子页面。
- 沿用 crackleaf 的 qpdf 分发方式：build.rs 把 `tools/qpdf.exe`（Windows）或系统 brew 安装的 qpdf（macOS）打包到产物旁边；运行时优先找同目录的 qpdf，找不到再回退到 PATH，再找不到给出友好提示与安装引导。

### **9.2 PDF 工具页布局**

```text
左侧子菜单：解锁 / 合并 / 拆分 / 压缩 / Word 转 PDF
中间主操作区：拖入文件 → 操作 → 结果列表
右侧（可选）：参数与日志
```

首页提供"PDF 解锁"快捷入口（一键直达 PDF 工具页的解锁子页）。

### **9.3 PDF 解锁功能要求**

- 支持单文件、多文件、文件夹批量
- 文件状态：等待 / 处理中 / 成功 / 失败（带原因）
- 失败时区分：密码受限 / 文件损坏 / qpdf 不可用 / IO 错误
- 输出文件命名：`<原名>_unlocked.pdf`，输出位置可选（同目录 / 自选）
- 完成后允许"打开输出目录"

### **9.4 后续 PDF 子工具**

| 子工具 | 实现思路 |
|---|---|
| PDF 合并 | qpdf `--pages` 或 `lopdf` |
| PDF 拆分 | qpdf `--split-pages` |
| PDF 压缩 | 调用 ghostscript / qpdf 重写流压缩 |
| Word 转 PDF | 与所函生成共用导出策略（Word/WPS/LibreOffice） |

---

## **10. 批量生成**

### **10.1 适用场景**

同一模板需要生成多份文档（例如同一所函模板换不同当事人/案号），且数据量较大、不便逐份手填。

### **10.2 流程**

1. 在生成页右上角切换为"批量模式"。
2. 选择数据源：
   - 上传 Excel / CSV 文件
   - 或粘贴表格
3. 字段映射：把表头列映射到模板字段。`party` 这种复杂字段允许"按行号分组"或"按列名匹配"两种映射方式。
4. 预览前 3 行结果，确认无误。
5. 选择输出目录与命名规则（见 10.3）。
6. 点击"批量生成"，进度条显示当前进度，每行成功/失败可单独查看。
7. 完成后允许"全部打包为 zip"。

### **10.3 命名规则**

支持模板变量，例如：

```text
{{date}}_{{plaintiffs[0].name}}_所函.docx
```

未填项以 `_` 占位；非法文件名字符自动替换。

### **10.4 批量记录**

每行写入一条记录到记录中心，附 `batch_id`，方便事后筛选、重新生成。

---

## **10A. 图片排版成文档 / PDF（PicPaddler 模块） ✅ 已实现**

迁移自 `PicPaddler.py`，将截图批量排版成 A4 docx/pdf。

**参数**：
- 输出格式：docx / pdf / both
- 页面方向：auto / 横向 / 竖向
- 布局模式：
  - **按张数**：每页 1/2/3/4 张（自动计算行列）
  - **按行列**：指定行数 × 列数（如 2×3=6 张/页）
- 顺序：z / n / z_rev / n_rev
- 缩放模式：
  - **适应页面**（默认）：缩放图片以完全显示在区域内，保持比例
  - **填满裁切**：裁切图片以填满整个区域，可能丢失边缘
  - **原始大小**：使用图片实际像素大小，超出区域时自动缩小
- DPI：orig / 600 / 400 / 300 / 150
- 嵌入格式：auto / png / jpeg
- 文件名开关、页边距/间距

### **参数优先级与冲突处理**

当参数组合不合理时，按以下优先级调整：

1. **页面尺寸 > 布局 > 缩放 > 页边距**
   - 页面尺寸（A4 横/竖）是固定的
   - 布局决定每个 cell 的大小
   - 缩放决定图片在 cell 内的显示方式
   - 页边距影响可用区域

2. **冲突场景处理**：
   - **原始大小 + 多张/页**：如果原始图片太大，单张就超出 cell，则自动缩小；如果多张能放下，则按原始大小显示
   - **填满裁切 + 页边距大**：cell 变小，图片被裁切更多
   - **适应页面 + 页边距大**：cell 变小，图片自动缩小以适应

3. **预览框**：
   - 固定在界面右侧，左侧区域上下分成参数与分析/图片列表
   - 在 A4 背景上按真实页边距、间距和图片区域显示位置
   - 预览计算必须与 Rust 生成路径保持同一模型：页面方向、行列、图片可用高度、文件名预留高度、fit/fill/original 缩放
   - 显示每页能放多少张、共多少页，超出边界时显示警告

### **预览框设计**

```
┌─────────────────────────────────────┐
│  A4 页面（灰色背景）                   │
│  ┌─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐  │
│  │  页边距区域                      │  │
│  │  ┌─────────────┐ ┌─────────────┐│  │
│  │  │             │ │             ││  │
│  │  │   图片 1    │ │   图片 2    ││  │
│  │  │  320×180    │ │  320×180    ││  │
│  │  │             │ │             ││  │
│  │  └─────────────┘ └─────────────┘│  │
│  │  文件名：xxx    文件名：xxx      │  │
│  │  ┌─────────────┐ ┌─────────────┐│  │
│  │  │             │ │             ││  │
│  │  │   图片 3    │ │   图片 4    ││  │
│  │  │             │ │             ││  │
│  │  └─────────────┘ └─────────────┘│  │
│  └─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘  │
└─────────────────────────────────────┘

信息显示：
- 页面：210×297mm（竖向）
- 页边距：10mm
- 可用区域：190×277mm
- 布局：2×2 = 4 张/页
- 每个 cell：92×133mm
- 图片大小：320×180px → 68×38mm（适应页面）
- 总页数：3 页（12 张图片）
```

**推荐逻辑**：分析扫描到的图片清单，按中位宽高、透明/JPEG 比例 → 推荐页面方向、布局、DPI、嵌入格式。

**输出**：默认写入 `<输入目录>/_docsy_image_out/`，输出文件名使用源文件名前缀（去掉编号），返回图片数、页数、校验结果。

**入口**：一级菜单”图片排版”+ 首页快捷卡片。

### **10A.5.1 当前修复要求（2026/06/04）**

本轮修复针对 `/Users/only/Downloads/test video` 中 644×1398 竖图样本暴露的问题：

1. **Word 文件名不得分页**：docx 输出必须给表格行设置固定高度，并从图片区域中明确扣除文件名高度，确保同一张图和文件名留在同一个 cell 中。
2. **Word 与 PDF 布局一致**：docx 和 pdf 必须使用同一套行列、页面方向、页边距、间距、图片可用高度和缩放规则；自定义行列不能回退到 2×2。Word 表格还必须写入精确 `tblW` 和 `tblGrid`，不能只依赖 `tcW`，否则 Microsoft Word/WPS 可能重新分配列宽。
3. **填满裁切一致**：PDF 已实际裁切图片；docx 也必须在写入媒体文件前裁切图片字节，不能只计算裁切尺寸后让 Word 拉伸原图。
4. **图片列表完整**：分析结果返回实际扫描到的图片清单，前端只负责滚动展示，不能只展示前 20 张抽样。
5. **预览可信**：右侧预览必须显示真实 A4 比例、页边距框、文件名预留区和图片缩放结果，避免“预览正常、输出错位”。
6. **PDF 文件名居中**：PDF 手写文本时要按文件名宽度估算居中位置，不能固定从 cell 左侧 2pt 开始。
7. **Word/WPS 实际打开优先于 LibreOffice 渲染**：LibreOffice 渲染可能显示 3×2 正常，但 Microsoft Word 中会变成每页 3 张单排，WPS 也可能第一页正常、后续分页漂移。docx 路径不能让表格总高度刚好贴满可打印区域，也不能在满高表格后追加分页段落；应给 Word/WPS 预留分页余量，并把分页符放在下一页表格之前。

### **10A.6 文件名分组功能（2026/06/04 新增）**

**场景**：用户截图通常来自同一视频，文件名前缀一致、只有编号不同。但同一文件夹中可能混入不同来源的截图（如不同日期、不同案件、不同视频平台）。

**设计**：

1. **分析阶段**：扫描目录中所有图片，提取文件名”前缀”（去掉末尾编号和扩展名）。
   - 例：`20251231-毕道钦-抖音视频-硅宝-取证-20251231-0001.png` → 前缀 `20251231-毕道钦-抖音视频-硅宝-取证-20251231`
   - 编号识别规则：去掉文件名末尾的 `-数字` 或 `_数字` 或空格+数字
2. **分组提示**：如果同一目录中发现 ≥2 种前缀，分析结果中返回 `groups` 数组，每组包含前缀、文件数、示例文件。
3. **用户选择**：
   - **按前缀分组**（默认）：每种前缀生成独立文档，输出文件名为前缀.docx
   - **全部合并**：忽略前缀差异，合并成一个文档
   - **手动选择**：勾选哪些前缀参与（高级选项）
4. **前端 UI**：分析完成后，如果检测到多组，在分析面板下方显示分组列表和选择控件。

---

## **10B. 视频抽帧模块（FFmpeg） ✅ 已实现**

从视频中批量提取截图，支持时间轴叠加。主要用于取证视频的帧提取。

### **10B.1 功能概述**

1. **FFmpeg 检测与安装**：检测系统 FFmpeg，支持在线下载（可配置私有地址）
2. **视频信息读取**：通过 ffprobe 读取视频的帧率、分辨率、时长等
3. **抽帧执行**：按设定的帧率提取截图，支持时间轴叠加

### **10B.2 抽帧参数**

| 参数 | 说明 |
|---|---|
| 抽帧模式 | "每秒N张" 或 "间隔秒数" |
| 输出格式 | JPG / PNG |
| JPEG 质量 | 1-100 |
| 时间轴叠加 | 可选，支持位置、字体、颜色、背景框 |

### **10B.3 时间轴渲染**

时间轴使用 FFmpeg 的 drawtext 滤镜，参数通过相对坐标表达式实现跨分辨率适配：

```
drawtext=text='%{pts\:gmtime\:0\:%T}':fontfile=/path/to/font.ttf:x=w*0.03:y=h*0.03:fontsize=32:fontcolor=red:box=1:boxcolor=black@0.6:boxborderw=8
```

- `x=w*0.03` 表示视频宽度的 3%（相对位置）
- `y=h*0.03` 表示视频高度的 3%（相对位置）
- 字体文件通过系统字体扫描获取

### **10B.4 跨平台注意事项**

| 项目 | macOS | Windows |
|---|---|---|
| ffmpeg 二进制 | ffmpeg | ffmpeg.exe |
| 常见安装位置 | /opt/homebrew/bin | C:\Program Files\ffmpeg\bin |
| 系统字体目录 | /Library/Fonts, ~/Library/Fonts | C:\Windows\Fonts |
| 控制台窗口 | 无 | 需要 CREATE_NO_WINDOW |

### **10B.5 下载源配置**

支持在 settings.json 中配置 ffmpeg 下载源：

```json
{
  "ffmpeg_download": {
    "macos_arm": "https://evermeet.cx/ffmpeg/ffmpeg-7.0.zip",
    "macos_x64": "https://evermeet.cx/ffmpeg/ffmpeg-7.0.zip",
    "windows": "https://github.com/BtbN/FFmpeg-Builds/releases/...",
    "private_url": ""  // 私有地址覆盖，优先使用
  }
}
```

---

## **11. 本地存储与配置导入导出**

### **11.1 数据位置**

| 操作系统 | 路径 |
|---|---|
| macOS | `~/Library/Application Support/Docsy/` |
| Windows | `%APPDATA%\Docsy\` |

目录结构：

```text
Docsy/
├── docsy.db               # SQLite 数据库
├── templates/             # 模板包（每份一个 .docsytpl，见 13.3）
│   ├── builtin_letter.docsytpl
│   └── user_xxx.docsytpl
├── output/                # 默认输出目录（用户可改）
├── logs/                  # 运行日志
└── settings.json          # 用户设置
```

### **11.1.1 运行日志**

Docsy 必须保留本地运行日志，用于定位“加载失败、保存失败、模板编辑异常、外部工具不可用”等问题。

当前日志策略：

- 日志位置：`logs/docsy-YYYYMMDD.log`，位于 Docsy 用户数据目录下。
- 日志格式：JSON Lines，一行一条记录，包含 `ts`、`level`、`target`、`message`、`context`。
- 设置页提供“打开日志文件”入口。
- 前端模板编辑器 API 层记录 Tauri 命令的开始、成功、失败。
- 后端模板编辑关键命令记录模板 id、字段数量、标记数量、docx/base64 长度、偏移范围、错误信息。

日志边界：

- 允许记录：命令名、模板 id、路径、数量、长度、偏移、错误字符串。
- 禁止记录：完整 docx/base64、完整 HTML/XML、用户文书正文、批量生成的完整字段值。
- 日志写入失败不能影响主流程，只能降级到控制台或忽略。

详细规范见 `docs/logging-observability.md`。新增模块、Tauri 命令或复杂前端流程时，必须按该文档补齐开始、成功、失败和降级日志。

### **11.2 SQLite 表结构（建议）**

```sql
-- 模板索引（实际模板内容存放于 templates/<id>.docsytpl）
CREATE TABLE templates (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    type            TEXT,                -- letter / contract / custom ...
    builtin         INTEGER NOT NULL DEFAULT 0,
    pinned_to_tab   INTEGER NOT NULL DEFAULT 0,
    version         TEXT,
    file_path       TEXT NOT NULL,       -- 相对 templates/ 的路径
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 字段历史值（用于"老客户记忆"，按字段独立累积）
CREATE TABLE field_history (
    template_id     TEXT NOT NULL,
    field_key       TEXT NOT NULL,
    value_json      TEXT NOT NULL,       -- 简单字段是字符串，party 字段是对象
    last_used_at    TEXT NOT NULL,
    use_count       INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (template_id, field_key, value_json)
);

-- 当事人主档（跨模板共享，方便"老客户直接选"）
CREATE TABLE parties (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    subject_type    TEXT NOT NULL,       -- 自然人 / 法人 / 其他组织
    aliases_json    TEXT,                -- 别名列表
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 生成记录
CREATE TABLE generation_records (
    id              TEXT PRIMARY KEY,
    template_id     TEXT NOT NULL,
    template_version TEXT,
    input_json      TEXT NOT NULL,
    output_path     TEXT,
    pdf_path        TEXT,
    batch_id        TEXT,                -- 批量生成时填
    created_at      TEXT NOT NULL
);

-- 字典表（法院、案由、阶段等可检索列表的来源）
CREATE TABLE dictionaries (
    name            TEXT NOT NULL,       -- courts / causes / stages / firms ...
    key             TEXT NOT NULL,
    label           TEXT NOT NULL,
    pinyin          TEXT,                -- 用于检索
    extra_json      TEXT,
    PRIMARY KEY (name, key)
);
```

> 字段历史与当事人主档分开：`field_history` 是模板内字段的"用过哪些值"，`parties` 是跨模板共用的当事人档案。两者通过 UI 可以互相导入。

### **11.3 配置导入导出（替代同步）**

第一阶段不做团队同步与服务端，团队共享通过文件交换实现。

**导出：**

- 设置页提供"导出配置"按钮，弹窗让用户勾选要导出的内容：
  - [x] 模板（可多选具体模板）
  - [x] 字段历史（field_history）
  - [x] 当事人主档（parties）
  - [x] 字典（dictionaries：法院/案由/阶段/律所/律师等）
  - [ ] 生成记录（默认不导出，含敏感信息）
  - [ ] 用户设置（settings.json）
- 输出为单个 `Docsy-Bundle-<时间戳>.docsybundle` 文件（zip 容器，结构见下）。
- 用户可通过任何方式（U 盘、IM、邮件）发给同事。

**Bundle 结构：**

```text
Docsy-Bundle-20260603-1530.docsybundle  (实为 zip)
├── manifest.json          # 版本、导出时间、内容清单、Docsy 版本
├── templates/
│   ├── tpl_xxx.docsytpl
│   └── tpl_yyy.docsytpl
├── data/
│   ├── parties.json
│   ├── field_history.json
│   └── dictionaries.json
└── settings.json          # 可选
```

**导入：**

- 设置页"导入配置"按钮，选择 `.docsybundle` 文件。
- 弹窗显示清单（多少模板、多少当事人、多少历史值），允许逐项勾选。
- 冲突处理策略（每项可独立选择）：
  - 跳过：保留本地
  - 覆盖：用 Bundle 替换
  - 合并：模板按 id 去重，列表按主键去重；当事人按 `name + subject_type` 去重
  - 重命名导入：模板生成新 id，避免与本地冲突
- 导入前自动备份当前数据到 `Docsy/backups/`，可一键回滚。

**单模板导入导出：**

- 模板管理页里每条模板可"导出此模板"，输出单文件 `.docsytpl`。
- 接收方在模板管理页"导入模板"即可。

### **11.4 备份**

- 软件每次升级前自动备份整个 `Docsy/` 目录到 `Docsy/backups/<YYYYMMDD-HHMMSS>/`。
- 设置页提供"立即备份 / 恢复备份"按钮。

---

## **12. UI 布局**

### **12.1 首页**

```text
┌────────────────────────────────────────────────────┐
│ Docsy                                       设置   │
├──────────┬─────────────────────────────────────────┤
│ 标签页    │  欢迎使用 Docsy                          │
│           │                                         │
│ ▶ 所函生成 │  快捷入口：                              │
│   模板制作 │   [生成所函]  [导入模板]  [PDF 解锁]    │
│   模板管理 │                                         │
│   PDF 工具 │  最近模板：                              │
│   记录中心 │   - 所函模板（内置）                    │
│   设置    │   - 委托代理合同（自制）                 │
│           │                                         │
│ — 我的模板 │  最近记录：                              │
│   委托合同 │   - 浦项 所函 2026-06-03                │
│   ...     │   - 张三 委托合同 2026-06-02            │
└──────────┴─────────────────────────────────────────┘
```

### **12.2 所函生成页 / 自制模板页（同一布局）**

```text
┌─模板列表─┬──────表单填写──────┬──────预览──────┐
│ 所函     │ 法院：[       ]    │  律师事务所函   │
│ 委托合同 │ 原告：+ 添加        │  北京……法院：   │
│ ...     │   [日本制铁株式会社] │  原告 日本制铁  │
│         │ 被告：+ 添加        │  …            │
│         │ 第三人：+ 添加      │                │
│         │ 案由：[选择/输入]   │                │
│         │ 案号：[         ]   │                │
│         │ 委托人：[选择]      │                │
│         │ 律师：[       ]    │                │
│         │ 律所：[       ]    │                │
│         │ 日期：[2026-06-03] │                │
├──────────┴────────────────────┴─────────────────┤
│ [保存草稿] [生成 docx] [导出 PDF] [复用上次输入] │
└──────────────────────────────────────────────────┘
```

### **12.3 模板制作页**

```text
┌─模板属性─┬──────Word 原文预览──────┬──字段属性面板──┐
│ 模板名   │ 律师事务所函              │ key  : court    │
│ 类型     │ ───────────              │ label: 法院     │
│ 描述     │ [北京知识产权法院]：←选中  │ type : text     │
│ 出厂图标 │ 原告 [日本制铁株式会社]   │ 必填 : ☑        │
│         │ 与被告 [国家知识产权局]   │ 多值 : ☐        │
│         │ ...                      │ 列表 : 法院库     │
│         │                          │ 样式 : 仿宋三号   │
│         │                          │       下划线 ☐   │
├──────────┴──────────────────────────┴──────────────┤
│ [+标记为字段] [取消标记]   [保存模板] [固定到标签页] │
└────────────────────────────────────────────────────┘
```

### **12.4 模板管理页**

```text
┌────模板列表────┬──────详情/字段──────┐
│ ☆ 所函（内置） │ 字段：12 个            │
│ ★ 委托合同    │ 历史使用：23 次        │
│   ...        │ 创建：2026-05-10      │
│              │ [编辑] [复制] [导出]   │
│              │ [删除]   ☑ 固定到标签页 │
└──────────────┴────────────────────────┘
```

### **12.5 PDF 工具页**

```text
┌──子工具──┬────────主操作区────────┐
│ ▶ 解锁   │ 拖入 PDF 或点击选择      │
│   合并   │ ┌───────────────────┐ │
│   拆分   │ │ 文件 1.pdf  ✅    │ │
│   压缩   │ │ 文件 2.pdf  ⏳    │ │
│ Word→PDF │ └───────────────────┘ │
│         │ [开始]  [打开输出目录]  │
└──────────┴────────────────────────┘
```

### **12.6 记录中心页**

```text
┌────搜索/筛选────┐
│ 模板 ▼  时间 ▼ │
├────────────────┤
│ 浦项 所函 06-03 │
│ 张三 合同 06-02 │
│ ...            │
├────────────────┤
│ 详情：表单值快照 │
│ [在文件夹中显示] │
│ [复用此输入]    │
│ [重新生成]     │
└────────────────┘
```

---

## **13. 模板包格式（.docsytpl）**

### **13.1 设计目标**

- 单文件即可分享、导入、导出
- 内含原始 docx，保证生成时可直接做 XML 字段替换、不破坏版式
- 字段定义独立成 JSON，便于版本对比与人工查看

### **13.2 容器结构**

`.docsytpl` 实为 zip 容器，建议结构：

```text
my_letter.docsytpl  (zip)
├── manifest.json     # 模板元数据：id、name、type、version、Docsy 兼容版本
├── template.docx     # 原始 Word 文件（保留全部样式、页眉页脚、图片等）
├── fields.json       # 字段定义（见 5.6 / 6.2）
├── dictionaries.json # 该模板私有的字典（可选，例如默认案由列表）
└── preview.png       # 可选：模板缩略图，用于模板管理页展示
```

### **13.3 manifest.json 示例**

```json
{
  "id": "tpl_letter_default",
  "name": "律师事务所函",
  "type": "letter",
  "version": "1.0.0",
  "docsy_min_version": "0.1.0",
  "builtin": true,
  "icon": "letter",
  "created_at": "2026-06-03T10:00:00",
  "updated_at": "2026-06-03T10:00:00"
}
```

### **13.4 字段定义文件**

`fields.json` 是一个数组，每项的 schema 见 5.6。简单字段与复杂字段（含 components / references / options）使用同一 schema，靠 `type` 区分。

### **13.5 占位符策略**

- 模板制作器在保存 `template.docx` 时，把用户标记的文本替换为 `{{key}}` 形式的占位符。
- 占位符必须落在**单个 run 内**，否则字段渲染会丢失样式。模板制作器在标记时若发现选区跨多个 run，应自动合并 run（合并时统一采用第一个 run 的 rPr）。
- 占位符不允许跨段落、跨表格单元格。模板制作器拒绝此类标记并提示。

---

## **14. 文档格式兼容性与 PDF 导出**

### **14.1 核心原则**

Docsy 不重新排版文档，只做"原始模板结构 + 字段内容替换"。

```text
原始模板结构 + 字段内容替换 = 新文档
```

避免：

- 把 Word 内容转纯文本后再生成
- 用 HTML / Markdown 中转生成 docx
- 用自研排版引擎重绘文档
- 通过复制粘贴式逻辑拼接

推荐：

- 直接修改 docx 内部 XML
- 只替换文本节点，保留 `document.xml` 的结构
- 保留 `styles.xml` / `numbering.xml` / `header*.xml` / `footer*.xml`
- 保留 `media/` 目录与 `_rels/` 关系文件

### **14.2 第一阶段只支持 .docx**

- MVP 仅支持 `.docx` 编辑与生成
- `.doc` 不直接做字段替换；后续阶段允许导入时先转换成 `.docx`
- `.docm` 不作为模板生成格式；可作为参考文件读取其中的 XML/JSA/VBA 片段，但 Docsy 写出的模板文件应降级为 `.docx`，不复制宏。
- 不要为了支持 `.doc` 而引入复杂的二进制解析

### **14.2.1 写出文件的质量门**

“代码成功执行”不等于“Word/PDF 能正常打开”。Docsy 写出的文件必须分层校验：

| 等级 | docx | pdf |
|---|---|---|
| 容器级 | zip 可打开，核心部件存在，关系文件完整 | `%PDF-` 头、`%%EOF` 尾、文件大小合理 |
| 结构级 | `document.xml` 基础 XML 可解析，图片关系能找到 media 文件 | 能读取页数，交叉引用表可解析 |
| 打开级 | Word/WPS/LibreOffice 可打开，不提示修复 | PDF 阅读器可打开，无空白页 |
| 视觉级 | 生成缩略图/截图，与预期样式比对 | 生成缩略图/截图，与预期排版比对 |

当前实现应先完成容器级校验；后续把 Word/WPS/LibreOffice 或 PDF 渲染引擎接入为打开级/视觉级校验。

### **14.3 字段替换的样式继承**

替换 `{{key}}` 时，新文本继承占位符所在 run 的：字体、字号、加粗、斜体、下划线、字体颜色、字符间距等。段落级属性（行距、对齐、缩进）由占位符所在段落决定，不动。

`party` 字段渲染多个名字时：

- 每个名字单独成一个 run，继承占位符的 rPr（保留下划线）
- 顿号作为独立 run，**不带下划线**
- 名字之间不另起段落

### **14.4 PDF 导出策略**

第一版不自研 PDF 渲染。Docsy 提供三档优先级，用户在设置中可自选：

```text
优先：本机 Microsoft Word / WPS 导出（保真度最高）
回退：LibreOffice Headless（跨平台、可自动化）
仅生成 docx，不导出 PDF（最低依赖）
```

| 方案 | 优点 | 缺点 |
|---|---|---|
| 本机 Word/WPS | 与用户实际打开效果一致，保真度最好 | 依赖本机已安装；macOS / Windows 调用方式不同；需处理自动化权限 |
| LibreOffice Headless | 跨平台、可批处理 | 中文字体、分页、表格可能与 Word 略有差异；打包体积变大 |
| 仅 docx | 零外部依赖 | 用户需自行用 Word/WPS 另存 PDF |

平台调用建议：

- macOS：通过 `osascript` 调 Word / WPS，或调用 `/Applications/.../soffice` 走 LibreOffice。
- Windows：用 COM 调 Word（`pywin32` 思路在 Rust 侧用 `windows` crate），或调用 `soffice.exe`。

### **14.5 团队版（远期）**

未来如有需要，可由维护者部署一个 Docsy Server 统一导出 PDF，保证团队内输出一致。**第一版不做。**

---

## **15. 字体管理**

### **15.1 问题**

Word 与 WPS 在不同电脑上打开同一份 docx，若字体缺失会自动替换字体，导致版式偏移。

### **15.2 Docsy 的处理**

- **生成前检查**：解析模板用到的字体清单，检查本机是否安装；缺失的字体在生成对话框中标黄并提示。
- **常用字体清单**：宋体、仿宋、黑体、楷体、微软雅黑、Times New Roman。模板规范鼓励只用这些。
- **导出缺失字体报告**：可一键导出当前模板缺失字体清单，方便 IT 一次性补齐。
- **不替换字体**：Docsy 自身永远不修改 docx 的字体声明。

---

## **16. 兼容性测试**

每个内置模板与建议的自制模板都应至少在以下环境抽测一次：

- Windows + Microsoft Word
- Windows + WPS
- macOS + Microsoft Word
- macOS + WPS

测试维度：

- 字体一致
- Word/WPS 打开不提示“发现不可读取的内容”或“需要修复”
- `.docx` zip 结构完整，图片、页眉页脚、关系文件不丢失
- PDF 可被常见阅读器打开，页数和输出记录一致
- 页边距 / 行距一致
- 表格不变形
- 页眉页脚保留
- 图片 / 印章不错位
- 分页位置稳定
- 当事人下划线保留
- PDF 导出与 Word 中显示一致

---

## **17. 模板规范**

为提高跨端兼容性，鼓励模板作者遵循：

- 统一使用 `.docx`
- 不使用复杂嵌套文本框
- 减少浮动对象，优先用表格控制版式
- 占位符不跨 run、不跨段落、不跨单元格
- 字段内容长度对版式影响较大的，提前留出空间或预设缩字策略
- 涉及分页的字段（长事由、当事人列表）提前测试满载情形
- 字段命名采用英文小写下划线（`court_name`、`case_no`），避免中文 key

---

## **18. 开发路线图**

### **第 0 阶段：原型（2 周）**

- Tauri + Vue 工程骨架跑通
- 把 crackleaf 的 qpdf 调用逻辑迁入 `src-tauri/src/pdf/qpdf.rs`，前端做最简 PDF 解锁页
- 内置一份所函模板（手工制作 docx + fields.json），实现"填表→生成 docx"

### **第 1 阶段：MVP（4 周）**

- 模板制作器：选 docx → 圈选文本 → 标字段 → 保存 `.docsytpl`
- 模板管理页 + 固定到标签页
- 字段类型完整：text / textarea / date / select / multiselect / party / reference / number
- 老客户记忆（field_history）+ 当事人主档（parties）
- PDF 解锁、所函生成、记录中心、复用上次输入
- 配置导入导出（`.docsybundle`）

### **第 2 阶段：完善（4 周）**

- 批量生成（Excel / CSV）
- PDF 合并 / 拆分 / 压缩
- Word → PDF 三档导出（Word/WPS、LibreOffice、仅 docx）
- 字体检查与缺失报告
- 备份与一键回滚

### **第 3 阶段：远期**

- `.doc` 导入并自动转 `.docx`
- 模板版本管理（diff、回滚）
- 可选的服务端中枢（团队版）
- 字段 / 模板 marketplace（团队内共享）

---

## **19. 产品名称与品牌**

- **名称**：Docsy
- **定位语**：Your Tiny Document Buddy
- **中文描述**：轻量、高效、可爱的文档处理工具箱
- **图标方向**：圆角纸张 + 右上角折角 + 两个小眼睛 + 微笑表情，蓝白配色
- **配色**：主色 `#4F8CFF`，辅色 `#A5C8FF`，背景 `#FFFFFF`，文字 `#1F2937`

---

## **附录 A：与初稿的差异速查**

| 主题 | 初稿 | 本版 |
|---|---|---|
| 团队同步 / 服务端 | 第 9、10 章详细设计 | **移除**，改为 `.docsybundle` 文件交换 |
| 所函建模 | 仅举例字段名 | 完整 12 字段表，含 party / reference / 字典 |
| 模板制作 | 仅简述选文本→变量 | 新增第 5 章详设流程、字段类型体系、样式继承规则 |
| 自制模板挂标签页 | 未明确 | 新增第 8 章：与内置模板能力完全对等 |
| PDF 解锁 | 提了一句 | 新增第 9 章，明确从 crackleaf 迁移路径 |
| 模板包格式 | 未定义 | 新增第 13 章 `.docsytpl` 格式 |
| 路线图 | 未给 | 新增第 18 章四阶段计划 |

---

## **附录 B：实现进度与接手指南（Handoff）**

> 更新于 2026/06/04。本附录用于把当前代码现状写清楚，让任何后续接手开发者（人或 AI）能快速接力，**无需口头交接**。
>
> **最近更新（2026/06/04）**：模板编辑功能、WPS 嵌套段落修复、跨 run 文本标记、标签输入优化、颜色区分、日期识别、文件名规则、表单状态持久化。

### **B.1 总体架构**

实际实现选型：**Tauri 2 + Vue 3 + Rust + Element Plus**。代码就在仓库根目录：

```
Docsy/
├── package.json                # 前端 deps：vue, element-plus, mammoth, @tauri-apps/*
├── vite.config.js              # 前端构建（端口 1420）
├── index.html
├── src/                        # 前端
│   ├── main.js
│   ├── App.vue                 # 一级菜单 + 视图路由（手写 shallowRef，不用 Vue Router）
│   ├── styles.css
│   ├── views/
│   │   ├── HomeView.vue        # 首页（最近模板 + 最近记录，真实数据）
│   │   ├── LetterView.vue      # 模板生成页（接 prop templateId 决定走哪个模板）
│   │   ├── TemplateView.vue    # 模板制作器（mammoth 预览 + 划选打气泡 + 点击编辑）
│   │   ├── ManageView.vue      # 模板管理（左侧模板列表 + 字段/字典/历史/生成记录 4 tab）
│   │   ├── RecordsView.vue     # 记录中心（全局生成记录聚合 + 筛选）
│   │   ├── PdfToolsView.vue    # PDF 工具壳（含子菜单）
│   │   ├── PdfUnlock.vue       # PDF 解锁实现
│   │   ├── SettingsView.vue    # 设置页（菜单可见性 + 历史上限）
│   │   └── PlaceholderView.vue # 待开发占位
│   └── components/             # 字段组件（Field*.vue）+ 字典编辑器
└── src-tauri/                  # 后端 Rust
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── capabilities/default.json
    ├── icons/                  # 占位 icon（PIL 生成的纯色块）
    ├── templates/              # 内置资源（编译时 include_bytes!）
    │   ├── letter.docx         # 律师事务所函内置模板
    │   ├── letter.fields.json  # 内置字段定义
    │   └── dictionaries.json   # 内置字典
    └── src/
        ├── main.rs
        ├── lib.rs              # Tauri 命令注册总入口
        ├── docx/
        │   ├── mod.rs
        │   └── render.rs       # docx 字段替换核心
        ├── pdf/
        │   ├── mod.rs
        │   └── qpdf.rs         # 调系统 qpdf 解锁/检测
        ├── templates.rs        # 模板配置存储 + 归档 + 软删除
        ├── template_builder.rs # 模板制作（解 docx 文本 + 写占位符 + 打 docsytpl）
        ├── dict_xlsx.rs        # 字典 Excel 导入/导出
        └── history.rs          # 生成记录 + 应用设置
```

### **B.2 数据存储位置（运行时）**

macOS：`~/Library/Application Support/Docsy/`

```
Docsy/
├── settings.json                       # 全局设置（history_max 等）
├── enabled.json                        # 模板软删除状态 { "letter": false }
├── templates/
│   ├── letter.json                     # 内置所函的字段覆盖配置（用户编辑）
│   ├── dictionaries.json               # 内置所函的字典覆盖
│   ├── <user_id>.json                  # 用户模板的字段覆盖
│   ├── dict_<user_id>.json             # 用户模板的字典覆盖
│   ├── letter/                         # 内置所函的字段归档
│   │   └── <YYYYMMDD-HHMMSS>.json
│   ├── dictionaries/                   # 内置字典的归档
│   ├── <user_id>/                      # 用户模板的字段归档
│   ├── dict_<user_id>/                 # 用户模板的字典归档
│   └── backups/                        # 预留（暂未使用）
├── user_templates/
│   └── <user_id>.docsytpl              # 用户自制模板（zip）
└── history/
    └── <template_id>/
        └── <YYYYMMDD-HHMMSS>.json      # 一次生成记录
```

`.docsytpl` 内部结构（zip）：
```
manifest.json     # { id, name, type, version, created_at }
fields.json       # { fields: [...] }
dictionaries.json # 可选：模板自带的初始字典
template.docx     # 含 {{key}}/{{*key}}/{{#row}}/{{?key:text}} 占位符
builder_state.json # 可选：模板制作器状态；新模板保存，用于再次编辑
```

### **B.3 后端 Tauri 命令一览**

| 命令名 | 输入 | 输出 | 用途 |
|---|---|---|---|
| `check_qpdf` | — | QpdfStatus | 探测 qpdf 是否可用 |
| `inspect_pdf` | `input: String` | InspectResult | 检测 PDF 是否加密 |
| `unlock_pdf` | `input: String` | UnlockResult | qpdf 解锁，输出到原目录 *_unlocked.pdf |
| `open_path` | `path: String` | () | 系统默认应用打开文件 |
| `read_file_bytes` | `path: String` | Vec\<u8\> | 前端读 docx 二进制（给 mammoth） |
| `get_letter_fields` | — | Value | （旧接口）取所函字段+字典覆盖合并 |
| `get_dictionaries` | `template_id: Option<String>` | Value | 按模板取字典；letter 走内置覆盖，否则取 docsytpl 内 + dict_\<id\> 覆盖 |
| `get_template_meta` | `template_id: String` | TemplateMeta | 统一取模板元数据：内置走 BUILTIN + 覆盖；用户走 docsytpl |
| `get_builtin_letter_fields` | — | Value | 取内置出厂版字段（不合并） |
| `get_builtin_dictionaries` | — | Value | 取内置出厂版字典 |
| `save_template_config` | `args: SaveTemplateArgs` | SaveResult | 写覆盖配置 + 同步归档 |
| `list_template_archives` | `template_id: String` | Vec\<ArchiveInfo\> | 列归档 |
| `restore_template_archive` | `template_id, archive_id` | () | 把指定归档设为当前 |
| `read_template_archive` | `template_id, archive_id` | Value | 读归档 JSON |
| `delete_template_archive` | `template_id, archive_id` | () | 删一个归档 |
| `is_template_enabled` | `template_id: String` | bool | 模板启用状态 |
| `set_template_enabled` | `template_id, enabled` | () | 软删除/恢复 |
| `export_dictionaries_xlsx` | `path: String` | String | 导出当前字典到 xlsx（单 sheet 横排）|
| `import_dictionaries_xlsx` | `args: ImportDictArgs` | Value | 从 xlsx 合并/覆盖字典 |
| `extract_docx_text` | `path: String` | DocxText | 提取 docx 纯文本 + 段落数 |
| `list_user_templates` | — | Vec\<UserTemplate\> | 扫 user_templates/ 目录 |
| `save_user_template` | `args: SaveTemplateArgs` | UserTemplate | 把 base64 docx + marks 写为 docsytpl |
| `delete_user_template` | `id: String` | () | 物理删除 docsytpl |
| `rename_user_template` | `id, new_name` | () | 更新 docsytpl 内 manifest.json 的 name |
| `get_user_template_fields` | `id: String` | Value | 取用户模板的 fields.json |
| `save_generation_record` | `args: SaveRecordArgs` | GenerationRecord | 生成成功后记录 |
| `list_generation_records` | `template_id: String` | Vec\<GenerationRecord\> | 列某模板的生成历史 |
| `read_generation_record` | `template_id, record_id` | GenerationRecord | 读一条记录（载入表单用）|
| `delete_generation_record` | `template_id, record_id` | () | 删 |
| `get_app_settings` | — | AppSettings | history_max 等 |
| `set_app_settings` | `settings: AppSettings` | () | 写 |
| `generate_letter` | `args: GenerateLetterArgs` | String | **核心**：渲染 docx，args 含 template_id 决定模板源 |

### **B.4 docx 占位符语法**

渲染器位于 `src-tauri/src/docx/render.rs`，识别四种占位符：

| 语法 | 行为 | 适用 |
|---|---|---|
| `{{key}}` | 简单文本替换；party 字段拆 run（顿号去下划线）| 所有字段 |
| `{{?key:text}}` | 条件前缀：值非空时输出 `text`，否则连同所在 run 一起删除 | 例：`{{?third_parties:、第三人}}` |
| `{{*key}}` | 行重复：所在 `<w:tr>` 按 list 长度克隆 N 份，每份用对应值替换 | 表格列表 |
| `{{#row}}` | 行号自动编号（基于 1，仅在 row-repeat 行内有效） | 表格列表 |

替换顺序：`process_row_repeats` → `process_runs`（含条件前缀和 party）→ `process_text`。

### **B.5 当前实现摘要**

> 完整最新状态见 `docs/project-health-report.md`。下表为快照，可能滞后。

当前版本已完成以下主干能力，详细流程以代码和专题文档为准，不再在本文维护逐条流水账：

| 模块 | 当前状态 | 主要位置 |
|---|---|---|
| 模板生成 | 内置所函、字段 schema、字典候选、生成历史、历史回填已可用 | `LetterView.vue`、`docx/render.rs` |
| 模板制作/编辑 | 支持 mammoth 预览、划选打标、`<<标签>>` 预览、重复位置候选、取消标记、插入字段、固有字纯文本编辑、保存 `.docsytpl` | `TemplateView.vue`、`template_builder.rs` |
| DocxDocumentModel | source map（段落/run/textNode 路径）、路径式文本替换、占位符扫描、diagnostics | `docx/model.rs` |
| 模板编辑器 source map | 预览节点带 `data-source-path`、mark 绑定 `markId + sourcePath`、保存时自动补全 source map | `useTemplatePreview.js`、`templateEditorMappers.js` |
| 模板管理 | 字段/字典/归档/生成记录 4 tab，支持内置模板编辑、软删除/恢复、用户模板重命名/删除 | `ManageView.vue`、`templates.rs` |
| 当前版本路由 | `templateId` 作为用户可见稳定身份，用户当前版本优先于出厂内置版本参与生成、管理、字段和字典读取 | `lib.rs`、`template_builder.rs` |
| 字典与归档 | 模板字典、用户覆盖、Excel 导入导出、字段/字典归档已接入 | `DictEditor.vue`、`dict_xlsx.rs`、`templates.rs` |
| 记录中心与首页 | 全局历史聚合、筛选、载入；首页显示最近模板和最近记录 | `RecordsView.vue`、`HomeView.vue`、`history.rs` |
| PDF 解锁 | qpdf 检测、加密检查、解锁 | `PdfUnlock.vue`、`pdf/qpdf.rs` |
| PDF 证据整理 | 扫描子文件夹 → 按组合成 → 整理身份（拖拽/命名） → 页眉页脚 overlay → 合并 | `PdfEvidenceView.vue`、`pdf/evidence.rs`、`pdf/overlay.rs` |
| PDF 页眉页脚 | printpdf 生成透明文字层 + qpdf overlay，CJK 字体自动加载，页码全局计算 | `pdf/overlay.rs` |
| 图片排版 | 文件夹分析、A4 布局、fit/fill/original 缩放、docx/pdf 输出 | `ImagePaddlerView.vue`、`image_paddler.rs` |
| 视频抽帧 | FFmpeg 检测/安装、视频信息读取、抽帧 + 时间戳水印 | `VideoExtractView.vue`、`ffmpeg/` |
| 模板编辑器重构阶段1 | 结构拆分：`TemplateView.vue` 变为薄壳，核心逻辑迁移到 `features/template-editor/`，抽出 API/mapper/preview/utils 层 | `TemplateEditorView.vue`、`templateEditorApi.js`、`templateEditorMappers.js`、`docxPreviewService.js` |
| 模板编辑器重构阶段2 | 引入 `TemplateEditorSession`：散落 refs 合并为统一 session，`pickFile`→`loadFromFile`，`loadExistingTemplate`→`loadFromTemplateId` | `useTemplateEditorSession.js`、`TemplateEditorView.vue` |
| 模板编辑器重构阶段3 | 拆分 preview 和 mark 逻辑：预览渲染和字段操作从 View 抽离到独立 composable | `useTemplatePreview.js`、`useTemplateMarks.js` |
| 模板编辑器重构阶段4 | 拆分固有字编辑：纯文字替换逻辑封装到独立 composable | `useTemplateTextEdit.js` |
| 模板编辑器重构阶段5 | 拆分 UI 组件：Toolbar、PreviewPane、MarkAside、MarkPopover 独立组件 | `components/*.vue` |
| 模板编辑器重构阶段6 | 后端单元测试：base64、文本替换、占位符写入、嵌套段落、相邻 run | `template_builder.rs` tests |
| 模板编辑器重构补充 | 保存逻辑抽离到 useTemplateSave.js；Unicode 字符计数修复；条件前缀预览优化 | `useTemplateSave.js`、`textRange.js`、`useTemplatePreview.js` |
| 推荐逻辑通用化 | 引用关系、推断关系、互斥关系配置化，LetterView 使用通用推断逻辑 | `letter.fields.json`、`LetterView.vue` |
| 推荐逻辑配置界面 | 模板管理新增"推荐逻辑"标签页，可视化配置引用、推断、互斥关系 | `ManageView.vue` |
| 推断规则编辑器 | 推断规则编辑对话框，支持源字段选择和映射关系配置 | `InferRuleEditor.vue` |

模板编辑器的细化拆分、会话模型、组件边界和迁移步骤见 `docs/template-editor-refactor.md`。

### **B.6 未实现 / 已知问题**

| 项 | 现状 | 优先级 |
|---|---|---|
| 批量生成（Excel/CSV → 多份文档）| 未实现，设计文档第 10 章 | 低 |
| PDF 合并/拆分/压缩 | 部分实现：证据整理流程中有合并（qpdf merge）和页眉页脚 overlay；通用 PDF 合并/拆分工具未做 | 低 |
| Word→PDF 三档导出 | 未实现 | 低 |
| 字体检查与缺失报告 | 未实现 | 低 |
| 配置导入导出 .docsybundle | 未实现 | 低 |
| 段落级删除（hideable 字段值为空时整段消失）| 未实现，只能删 run | 中 |
| 表格"对象数组多字段"行重复 | 未实现，仅单字段 | 中 |
| 选中已存在标记文本时提示关联 | 已实现基础版：相同/相近文本可复用已有字段；原文多位置可手动确认 | 中 |
| 模板预览模式（标记显示为《标签》）| 已实现：标签预览用 plainText + mark 偏移渲染，避免重复文本全局替换误导 | 低 |
| 模板当前版本全局一致 | 内置模板编辑后必须以用户当前版本取代出厂版本参与生成、管理、字段和字典读取；不能出现管理页已改、生成仍用旧模板 | 高 |
| 模板固有字编辑 | 已支持未标记普通文字的纯文字替换，并反写 docx 当前版本；字体、字号、加粗、斜体、下划线工具栏尚未实现 | 高 |
| 模板制作器精确定位 | 已实现 DocxDocumentModel source map（段落/run/textNode 路径）+ path-based 文本替换 + source-mapped 预览；mammoth HTML 仅作 fallback。页眉页脚/文本框/脚注未覆盖，跨 text node 选区仍 fallback | 高 |
| 旧模板编辑经验恢复 | 旧 `.docsytpl` 没有 `builder_state.json` 时先从占位符反推；取消标记时必须让用户输入恢复后的固有字或留空删除，保存后新版本会写入 `builder_state.json` | 高 |
| 历史直接重渲染文档 | 当前已支持字段摘要预览和载入历史；还未做到一键重新渲染临时 docx 预览 | 中 |
| 图片排版 PDF 中文文件名 | 已改善：非 ASCII 字符移除而非替换为 `_`，保留数字等 ASCII 部分（如"证据截图001"→"001"）；完整中文支持需嵌入 CIDFont 或走 Word/WPS 导出 | 低 |
| 图片排版 docx fill 裁切 | 已实现实际裁切/转码；后续可用 DrawingML `a:srcRect` 优化文件体积 | 低 |

### **B.6.1 当前系统化优化路线（2026/06/04 补充）**

Docsy 后续应围绕“模板闭环”继续改，而不是为单份模板打补丁：

```
原始 docx
  ↓
模板制作器：选区 + 标签 + 字段属性
  ↓
.docsytpl：template.docx + builder_state.json + fields.json + dictionaries.json
  ↓
标签预览：检查漏选字段
  ↓
模板编辑：补选、改标签、复用已有字段
  ↓
文档生成：字段表单 + 候选值 + 历史值
  ↓
历史记录：按“标签：值”预览、载入、重新生成、反哺字段推荐
  ↓
模板设置：字段、字典、默认值、必填、隐藏、版本归档
```

短期优先级：

1. **统一 TemplateEditorSession**：制作模板和编辑模板必须共享同一套 session、预览、标记、固有字编辑和保存逻辑；只允许入口初始化和初始呈现不同。
2. **定位底座**：当前先在纯文本偏移层补齐重复位置候选、空白归一匹配和保存校验；后续把 `extract_docx_text` 扩展为字符级源位置索引，前端预览节点携带 docx 源位置。
3. **经验复用**：制作器选中相同/相近文本时推荐复用已有 key；生成页合并模板字段 options、字典、历史值；历史记录后续应支持一键重新生成。
4. **当前版本路由**：所有按 `templateId` 读取模板内容的地方，都必须先找用户当前版本；没有用户当前版本时才回退出厂内置版本。
5. **模板对等**：内置所函应逐步拆成普通模板能力 + 出厂字段配置，用户模板也能使用相同的字段规则、候选和设置页。
6. **失败显性化**：保存 `.docsytpl` 时，后端必须确认所有 mark 都实际写入 `template.docx`；不能静默丢字段。

### **B.7 关键约定（接手必读）**

1. **不动表格结构**：渲染器全部只动 `<w:t>` 和 `<w:r>`，不修改 `<w:tc>/<w:tr>/<w:tbl>`。

2. **占位符须在单 `<w:t>` 内**：模板制作时通过 `merge_adjacent_runs` + `merge_adjacent_text_nodes` 预处理，确保标记文本合并到单个 `<w:t>` 后再写入占位符。

3. **template_id 路由**：`templateId` 是稳定身份。所有模板读取先查用户当前版本 `user_templates_dir()/<templateId>.docsytpl`；不存在时才回退出厂内置模板。字典命名空间当前仍保留 `"dictionaries"`（内置默认）和 `"dict_<id>"`（用户覆盖），但编辑后的当前版本必须优先作为模板字段/文档源。

4. **状态归属**：用户数据在 `~/Library/Application Support/Docsy/`。**不要**写到工程目录或 `src-tauri/templates/`。

5. **mammoth 仅用于预览**：生成路径是 `template.docx zip → process_*** → 写 zip`，不走 HTML 中转。

6. **自动归档**：`save_template_config` 每次写生成归档，超过 max 删最旧。

7. **timestamp 格式**：`YYYYMMDD-HHMMSS`。

8. **必填和可见性是会话级**：不写盘，每次进页面重置。

9. **制作器状态与生成模板分离**：`template.docx` 是占位符化后的生成源；`builder_state.json` 是再次编辑的源，保存原始 docx base64、原始选区 marks 和字段属性。编辑模板时必须优先读 `builder_state.json`，没有时才从 `template.docx` 占位符反推；若用户取消旧模板里的占位符标记，应让用户输入恢复后的固有字或留空删除。

10. **制作-校验-生成-历史是一条链**：模板制作器负责建立字段和样本文本经验；标签预览负责检查漏选；生成页负责把字段经验变成候选；历史记录负责把一次输入按“字段标签：值”回看和复用。

11. **字段字与固有字边界**：字段字在编辑器中以 `<<标签>>`/按钮形式展示，不直接编辑；取消标记后恢复为固有字，才能作为普通文本编辑和重新标记。

12. **制作与编辑同源**：`TemplateView.vue` 当前同时承载制作/编辑，但后续命名和内部结构应围绕 `TemplateEditorSession` 收敛。不要为 create/edit 复制两套状态机；所有修复应优先落到统一 session 层。

### **B.8 启动方式**

```bash
cd /Users/only/Documents/PythonProgram/Docsy
npm install                                  # 第一次
export https_proxy=http://127.0.0.1:7890     # 国内网络
export http_proxy=http://127.0.0.1:7890
export all_proxy=socks5://127.0.0.1:7890
npx tauri dev
```

依赖：Rust 1.88+、Node 20+、qpdf（macOS：`brew install qpdf`）。

### **B.9 接手者的下一步建议**

按优先级：

1. **docx 字符级源位置索引**：让制作器从“文本匹配”升级为“源位置选择”，这是解决 `田  力`、重复姓名、跨 run 选区的底座。
2. **历史重新生成**：记录中心和生成页的历史记录直接重填并重新生成，历史值继续反哺字段候选。
3. **内置所函普通模板化**：减少 `letter` 特殊逻辑，让所函经验沉淀到字段 schema、字典和模板设置。
4. **批量生成**：选模板 + Excel 数据源 → 一键多份（设计文档第 10 章）
5. **PDF 合并/拆分**：模仿 PDF 解锁那一套调 qpdf
6. **配置导入导出**：`.docsybundle` 打包/解包，跨电脑迁移（设计文档第 11.3 节）
7. **测试**：给 `template_builder` 和 `docx::render` 写覆盖真实样本的回归测试

---

## **附录 C：实现入口地图**

本附录只保留接手时最常用的入口。模板编辑器的详细架构、迁移步骤和验收标准统一维护在 `docs/template-editor-refactor.md`；docx XML 细节统一维护在 `docs/docx-research.md`。

### **C.1 前端入口**

| 文件 | 责任 |
|---|---|
| `src/App.vue` | 一级菜单、用户模板入口、keep-alive 路由、右键菜单 |
| `src/views/HomeView.vue` | 首页最近模板、最近记录、快捷入口 |
| `src/views/LetterView.vue` | 模板生成表单、候选合并、历史载入、生成调用 |
| `src/views/TemplateView.vue` | 当前模板制作/编辑器；后续应按 `docs/template-editor-refactor.md` 拆分 |
| `src/views/ManageView.vue` | 模板管理：字段、字典、归档、生成记录、编辑入口 |
| `src/views/RecordsView.vue` | 全局生成记录聚合、筛选、载入 |
| `src/views/ImagePaddlerView.vue` | 图片排版 UI |
| `src/views/VideoExtractView.vue` | 视频抽帧 UI |
| `src/views/PdfUnlock.vue` | PDF 解锁 UI |

### **C.2 后端入口**

| 文件 | 责任 |
|---|---|
| `src-tauri/src/lib.rs` | Tauri 命令注册、模板当前版本路由、生成入口 |
| `src-tauri/src/template_builder.rs` | `.docsytpl` 读写、docx 文本提取、标记写入、固有字纯文本替换 |
| `src-tauri/src/docx/render.rs` | docx 占位符渲染、表格行重复、party 字段拆 run |
| `src-tauri/src/templates.rs` | 模板字段配置、字典覆盖、归档、启用状态 |
| `src-tauri/src/history.rs` | 生成记录、应用设置 |
| `src-tauri/src/dict_xlsx.rs` | 字典 Excel 导入导出 |
| `src-tauri/src/image_paddler.rs` | 图片排版分析与 docx/pdf 输出 |
| `src-tauri/src/ffmpeg/` | FFmpeg 检测、下载、视频信息读取、抽帧执行 |
| `src-tauri/src/pdf/` | qpdf 检测、PDF 加密检查和解锁 |

### **C.3 用户数据位置**

```
~/Library/Application Support/Docsy/
├── settings.json
├── enabled.json
├── templates/
├── user_templates/
└── history/
```

`.docsytpl` 是 zip 包，当前约定包含：

```
manifest.json
fields.json
dictionaries.json
template.docx
builder_state.json
```

### **C.4 接手优先级**

1. 模板编辑器按 `TemplateEditorSession` 重构，消除制作/编辑两套状态。
2. docx 字符级源位置索引，替换当前基于 mammoth HTML 与纯文本的模糊定位。
3. 内置所函普通模板化，减少 `letter` 特殊逻辑。
4. 历史记录支持一键重新生成。
5. 为 `template_builder` 和 `docx::render` 补真实样本回归测试。
