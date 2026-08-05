# Docsy 0.9.6 完善方案

目标：修复今天讨论的所有遗留问题，全面审查一致性，升级到 0.9.6。

---

## Phase 1: 页脚文字支持特殊格式（[文件名][序号]等）

**现状**：页眉固定文本模式支持 [文件名]、[序号]、[日期]、[分段名] 等占位符，但页脚文字不支持。

**要求**：页脚文字的 text 解析应与页眉完全一致——支持相同的占位符格式。

**实现**：
- 在 `useEvidencePdfSession.js` 中找到 `buildHeaderTextForGroup` 函数
- 将其核心逻辑抽取为通用的 `resolveTextTemplate(text, file, index, rules)` 函数
- 页眉和页脚都调用这个函数解析文本
- `footerTextOverlayConfigForGroup` 调用时传入解析后的文本

## Phase 2: 页眉和页脚文字的分段设置

**现状**：每个文件有 `pageStart`/`pageEnd`，但这是页码分段用的。页眉和页脚文字没有独立的分段功能。

**要求**：
- 页眉 group 应支持 `pageStart`/`pageEnd`（仅对该 group 的文本生效）
- 页脚文字 group 也应支持 `pageStart`/`pageEnd`
- UI 中在页眉和页脚文字的 HeaderFooterRuleFields 组件中展示分段控件
- 文件列表子行中显示分段信息（如 "1-5页"）

**实现**：
- `createDefaultHeaderGroup` 和 `createDefaultFooterTextGroup` 加 `pageStart: 1, pageEnd: 0` 字段
- `HeaderFooterRuleFields` 组件中为页眉和页脚文字组各加分段输入框（复用现有页码分段的 UI 模式）
- `buildHeaderFooterItems` 中根据 group 的 pageStart/pageEnd 过滤适用范围
- 文件列表子行中显示分段标记

## Phase 3: 全局设置分离文本和格式参数

**现状**：全局应用开关同时控制文本和格式参数的共享。但文本（如页脚文字内容、页眉的[文件名]模式）可能是每个文件独立的（序号不同、文件名不同），而格式参数（字体、大小、对齐、边距）应全局统一。

**要求**：
- 格式参数（align/fontSize/fontFamily/marginMm/offsetXMm/color）始终全局统一，不受开关影响
- 文本内容（text/template/prefix/suffix/mode）跟随文件独立解析
- 全局应用开关只控制格式参数的统一

**实现**：
- 修改 `buildHeaderFooterItems`：全局模式下，group 的格式参数从全局 group 读取，但 text/mode 仍从文件自己的 group 读取（或从全局 group 读取后用文件信息解析）
- 实际上当前的实现中 text 已经通过 `buildHeaderTextForGroup(file, index, ...)` 按文件解析了，所以只需要确保格式参数走全局路径

## Phase 4: 批量生成处理所有字段类型

**现状**：`batch.rs` 的 `build_row_values` 只处理 `party_list` 和 `date` 两种特殊类型。`reference`/`checkbox`/`select`/`marker` 走默认的纯字符串路径。

**要求**：批量生成应与单个生成使用完全相同的渲染逻辑。

**实现**：
- `batch.rs` 的 `build_row_values` 中为每种字段类型添加正确的值构建：
  - `reference`：从引用源获取值
  - `checkbox`：布尔值 true/false
  - `marker`/`prefix`/`suffix`：保留原始文本
  - `select`：选项值
- 或者更好的方式：让 `build_row_values` 调用与单个渲染相同的值规范化函数
- 确保 `engine::render_docx` 收到正确的 values HashMap

## Phase 5: 撤销重做图标修正

**现状**：`UndoRedoButtons.vue` 使用 `Back` 和 `Right` 图标（直角箭头）。

**要求**：使用圆箭头（`RefreshLeft` 和 `RefreshRight`），符合撤销重做的通用视觉习惯。

**实现**：
- `src/components/UndoRedoButtons.vue` 中将 `Back` 改为 `RefreshLeft`，`Right` 改为 `RefreshRight`

## Phase 6: 全面审查一致性

完成上述修改后，进行全面 review：
1. 页眉和页脚文字的功能对称性检查（特殊格式、分段、多组支持）
2. 全局/独立模式的一致性检查
3. 撤销重做功能在所有设置区域的正常工作
4. 批量生成与单个生成的结果一致性
5. 文件列表子行的信息完整性（分段信息、编辑状态）
6. 版本号统一（package.json、tauri.conf.json、Cargo.toml）

## Phase 7: 版本升级和发布

1. 版本号改为 0.9.6（package.json、src-tauri/tauri.conf.json、src-tauri/Cargo.toml）
2. 更新 README.md 或 CHANGELOG（如有）
3. `cargo test` + `npm test` + `npm run build` 全过
4. `git commit` + `git push` + `git tag v0.9.6`

---

**注意事项**：
- 所有修改只涉及证据处理模块（EvidencePdfWorkbench.vue、useEvidencePdfSession.js、HeaderFooterRuleFields.vue）、模板批量生成（batch.rs）、撤销重做组件（UndoRedoButtons.vue）
- 不要改动渲染引擎（render.rs、engine.rs）的核心逻辑
- 每个 Phase 完成后运行 `npm test` + `npm run build` 验证
