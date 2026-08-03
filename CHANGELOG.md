# Docsy 更新日志

本文件记录 Docsy 每个版本的核心变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## [0.8.6] - 2026-08-03

### PDF 原有内容处理
- 页眉、页脚文字和页码从检测到处理拆分为三类独立对象，页顶页码不再被当作普通页眉
- 新增原有内容确认窗口，按文件显示检测文字、页段、来源、编辑前后内容和处理决定
- 支持逐项及批量全选、反选、保留、忽略、删除和编辑；重新检测时保留用户决定
- 顶部仅显示非零的原页眉、原页脚文字、原页码、待删除和待编辑数量，点击即可核对
- 主文件列表移除重复的原页眉、原页脚、原页码、现有处理和状态列
- 普通文本删除继续按文字、页段和位置匹配，不使用白色遮盖；忽略项不进入处理负载
- 普通文本候选以跨页重复、位置稳定和覆盖范围为核心；内容格式只负责分类，不再使用百分比、功率或规格黑名单
- Docsy 写入继续采用标准 Pagination Artifact，并增加 ActualText、内容类型、版本和稳定 ID，便于后续精确识别

### 页眉、页脚文字和页码
- 新页眉、页脚文字、页码使用三套独立设置，页脚文字不再承担页码职责
- 页码支持数字、中文数字、大小写罗马数字、带圈数字和实心带圈数字；中文数字覆盖 2000 页以上的大文件
- 页码格式支持 `{page}`、`{total}`、`{range}` 以及 `-{page}-` 等自定义前后符号
- 支持全部文件连续编号或每个文件单独编号
- 新增页码分段与例外窗口，可排除指定页段，或覆盖样式、格式、位置和编号偏移
- 分段规则按顺序覆盖，并合并为连续输出区间，避免为大 PDF 创建逐页前端对象

### 排序与文件队列
- 所有会影响输出顺序的列表统一使用 Pointer Events 拖拽，不与 Tauri 文件拖入冲突
- PDF 合并、证据文件、证据扫描分组、合并证据页段、基础拆分页段均支持真实拖拽排序
- 图片排版和视频抽帧结果支持拖拽、上移和下移，并保留分页与缩放预览
- 图片排版后端接受显式图片路径顺序，最终 PDF/DOCX 与预览顺序一致

### 外部工具
- Windows Word 探测覆盖 HKLM/HKCU、32/64 位注册表、COM 注册、常见安装目录及带空格路径
- macOS Word 探测覆盖系统/用户 Applications、Launch Services、AppleScript 和 Spotlight
- WPS 增加 COM 注册及实际可执行文件兜底；LibreOffice 增加 Windows/macOS 默认目录
- Word 转 PDF 不再被可执行文件路径探测提前拦截，按 Word、WPS、LibreOffice 顺序实际尝试
- 设置页工具检测并行执行，明确区分尚未检测、检测失败、未安装和可用，并可单独重试
- 长时间 Word/LibreOffice 转换支持继续等待，继续后重新计算等待周期

### 测试
- 新增检测对象决定、页码样式与分段、Pointer Events 排序和图片顺序测试
- 使用 `Test.zip` 场景回归空 Artifact、标准页码、重复处理和无损删除

## [0.8.4] - 2026-08-03

### 修复
- PDF 解锁导入后检测加密状态，未加密文件不再重复处理
- PDF 解锁与合并的长文件列表可以正常滚动
- PDF 合并支持通过拖拽柄调整文件顺序
- PDF 基础工具支持从 Finder/资源管理器拖入文件
- FFmpeg 状态仅显示简短版本号，避免撑宽侧栏
- Windows Word/WPS 检测和模板批量填写相关问题

### 改进
- 视频抽帧、图片排版与 PDF 工具共用窗口文件拖放逻辑
- 工作区采用统一的内容滚动布局，长列表不再被截断

### 测试
- Rust: 121 个测试通过
- Frontend: 55 个测试通过（8 个测试文件）

## [0.8.2] - 2026-07-30

### 新增
- **select 字段类型**：下拉选择控件，支持案由/诉讼阶段等预设选项，可搜索可手输
- **批量填写功能**：导出字段表为 Excel → 校验导入 → 批量生成 Word，含模板 ID 匹配校验、错误行跳过、日期单元格解析
- **模板审核报告**：全模块设计 vs 实现对照审核（docs/全模块审核报告-2026-07-30.md）

### 修复
- reference 字段前缀替换问题（值来自引用源时保留模板原文）
- reference 字段允许用户显式覆盖前后缀
- 案号前缀/后缀在无用户覆盖时自动从 optionalRule 兜底
- 预览全文按段落聚合（不再每个 run 一行）
- 回收站模板的历史记录不再显示（自动检测孤立记录）
- Excel 日期单元格解析（calamine DateTime/DateTimeIso 类型）
- WPS Writer 检测改为查找实际 wps.exe 路径（不再返回 COM ProgID）
- Microsoft Word 注册表查询添加 5 秒超时
- image-paddler filename_remove_text 不再被覆盖为空字符串

### 测试
- Rust: 121 个测试通过
- Frontend: 55 个测试通过（8 个测试文件）

## [0.8.1] - 2026-07-21

### 新增
- 模板引擎从正则表达式改写为 **quick-xml 结构化 XML 树**
- 坐标管线：scan 和 save 共用 paragraph/run 索引，消除错位
- 拆分字段（同 run 子串）按 start/end 偏移精确定位
- 文本框中标黄可被扫描到
- 格式保留：渲染时保留原 run 的 `w:rPr`（字体/加粗/斜体/符号字体）
- Doclet 工作动画 API（showLoading/hideLoading）
- WebKit NetworkCache 启动时自动清理

### 修复
- 模板字段结构在渲染期间保留
- 证据 PDF 页眉页脚编辑稳定性
- 模板历史在设置页操作后刷新

### 技术
- Tauri 2 升级到最新版本
- CI/CD 改进：DMG 打包、Windows NSIS、前端构建验证

## [0.7.5] - 2026-07-14

### 修复
- PDF 已有页眉页脚的页码编辑
- 证据 PDF 页眉页脚编辑流程
- macOS 应用签名

### 改进
- Windows FFmpeg 下载备用镜像
- 证据页眉规则简化

## [0.6.0] - 2026-06-28

### 新增
- 证据 PDF 页眉页脚完整工作流：检测/删除/插入分离
- 页眉页脚插入开关
- 独立数字页脚检测
- PDF 预览中显示删除标记
- 证据表格自然排序
- 已有页眉页脚显式删除

### 改进
- 证据页眉模板规则简化
- 批量合并导入重命名
- Windows 仅构建 NSIS 安装包

### 修复
- macOS 应用签名
- 纯文本页眉页脚转换加固

## [0.5.3] - 2026-06-12

### 新增
- 字典 Excel 导入导出：6 个 sheet（courts/causes/firms/lawyers/stages/parties）
- 模板 .docsytpl 打包：save() → write_placeholders() → pack_docsytpl()
- 前端 Vitest 测试：11 个测试通过（模块注册、标记重叠检测、键生成、文本范围）
- 批量生成：Step wizard + Excel 导入 + 字段映射 + 批量生成 + 进度条
- 配置导入导出：.docsybundle zip 包（模板/字典/当事人/设置）
- 字典自动录入：标记字段时 options 自动写入 field_history
- 高级字段设置：references/infer_from/exclude/dict_source/style
- 共享组件迁移：FieldText/Date/Select/Party/Reference/List + DictEditor + InferRuleEditor

### 技术
- rust_xlsxwriter 导出 Excel，calamine 导入 Excel
- 模板占位符写入基于 quick-xml 事件处理
- merge/overwrite 两种导入模式
- .docsybundle: zip 容器 + manifest.json

### 测试
- Rust: 11 个测试通过
- Frontend: 11 个测试通过（4 个测试文件）

## [0.5.2] - 2026-06-12

### 新增
- 设置页：外部工具状态检测（qpdf/ffmpeg/libreoffice）、诊断信息展示
- 首页：真实模板列表和最近记录、快捷入口对接模块注册表
- PDF 子页面：独立的解锁、合并、证据整理视图
- 图片排版视图：文件夹选择、参数配置、分析结果展示

### 改进
- 首页卡片和菜单从 moduleRegistry 动态生成
- 设置页集中管理外部工具路径和状态
- PDF 工具页子路由完善

### Git 提交记录
- `81b271f` 改进设置/首页/PDF/图片排版视图

## [0.5.1] - 2026-06-12

### 重构启动
- 将旧代码整体归档至 `Archived/` 目录（前端、后端源码、旧文档）
- 建立 v0.5 版本线，从零重建架构

### 新增
- 统一模块注册系统（前端 `moduleRegistry.js` + 后端 `ModuleDescriptor`）
- vue-router 替代手动 shallowRef 路由
- Pinia 状态管理（app / dictionary / template stores）
- 外部工具统一抽象层（`ExternalTool` trait）：qpdf, ffmpeg, libreoffice
- Tauri 命令自动收集机制（`commands/mod.rs`）
- 前端 `tauriBridge.js` 统一调用封装
- 8 个功能模块注册入口（home, doc-gen, template-editor, template-mgmt, pdf-tools, image-paddler, video-extract, settings）
- SQLite 数据库 schema（global_dictionaries, template_dictionaries, field_history, parties, generation_records, template_meta）
- 字典三层叠加查询引擎（global → template → history）
- docx 引擎：quick-xml 模型解析 + 占位符渲染（支持 {{key}}, {{?key:text}}, {{*key}}, {{#row}}），7 个单元测试
- PDF 证据整理：文件夹扫描、自然排序、DOC/DOCX 转换、分组合并、身份重命名、页眉页脚叠加
- PDF 页眉页脚：printpdf 文字层生成 + qpdf --overlay 合成、CJK 字体自动检测、{page}/{total} 占位符
- 图片排版：文件夹分析、维度检测、前缀分组、docx/pdf 输出、fit/fill/original 缩放，4 个单元测试
- 模板编辑器：加载 docx、标记字段、配置属性、保存 .docsytpl
- 视频抽帧：FFmpeg 检测、视频信息、抽帧设置、时间戳叠加
- 模板管理：模板列表、字段/字典/记录查看、固定/删除
- 外部工具检测/安装统一接口

### 测试
- Rust: 11 个测试通过（docx render 7 + image paddler 4）
- Frontend: vite build 通过

### Git 提交记录
- `ce73fb5` 归档旧代码 + 建立版本基础
- `fe09a13` 完整后端服务层 + 前端模块骨架
- `9bacfd0` 修复编译错误，验证构建
- `5d94962` 实现 PDF 证据整理和页眉页脚模块
- `d4ad609` 实现 docx 渲染引擎（quick-xml）
- `6628179` 实现图片排版模块
- `9ac7e77` 改进前端表单（完整字段类型支持）
- `4499e2c` 更新 CHANGELOG
- `dad36fb` 实现模板编辑器和视频抽帧模块
- `a34f5cc` 实现模板管理页面
