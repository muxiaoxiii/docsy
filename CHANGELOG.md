# Docsy 更新日志

本文件记录 Docsy 每个版本的核心变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## [0.9.6] - 2026-08-06

### 新增
- **检测对话框排序**：所有可排序列支持点击排序 + 自然排序（Intl.Collator）
- **Shift+click 范围选择**：检测对话框中支持 Shift 范围选择 + 视觉反馈
- **模板语法提示**：页眉页脚文本输入框显示模板标记语法 tooltip
- **文本字段自动补全恢复**：编辑页文本字段重新接入 autocomplete 建议
- **检测算法增强**：单页文件低置信度过滤、跨文件验证、全局进度显示

### 修复
- **模板模块 critical 修复**：renderableTemplateFields 不再直接修改 manifest 内部对象，改用浅拷贝避免数据污染
- **字段行重排修复**：normalizeFieldRows 调用时传入 documentRuns 参数，字段列表顺序与文档位置一致
- **partyItemsToValues 去重**：消除 TemplateView / TemplateRenderTab / fieldRowUtils 三处重复定义，统一从 fieldRowUtils 导入
- **typeLabel 修复**：现在正确搜索 subTypes（checkbox、radio_group、prefix 等子类型标签不再退化为原始值）
- **stableFieldId 标准化**：slug 生成增加 NFC 标准化，避免 CJK 字符编码不一致
- **证据处理类型修复**：reorderOverlayFiles 移除冗余的字符串赋值
- **检测对话框样式**：低置信度行使用 subtle 样式标记
- **单页文件检测修复**：页眉检测在单页文件上不再错误跳过
- **页脚 [文件名] 模板解析**：resolveTextTemplate 正确处理页脚中的文件名标记
- **Doclet 动画连续播放**：检测期间 Doclet 不再闪烁，使用 tauriCallQuiet 避免重复触发
- **检测进度节流**：进度更新限制为 500ms 间隔，避免频繁重渲染
- **两阶段检测**：detect all → apply all 消除 UI 闪烁
- **artifact 空文本跳过**：检测算法不再对空文本 artifact 回退到 content-text
- **repeating 阈值提升**：content-text repeating 阈值从 count>=2 提升到 count>=3
- **模板字段类型持久化**：填写页字段类型切换正确保存到 manifest
- **重复字段自动引用**：同名后续字段自动设为 reference 类型 + 分组颜色条位置修正
- **保存模板源文件修复**：sourceDocx 为空时使用 package 作为源

## [0.9.2] - 2026-08-04

### 新增
- **一键清除页眉页脚**：新增流水线按钮，自动检测→弹窗选择→确认→输出到 `_cleaned` 文件夹
- **关于页面**：GitHub 链接、小红书链接及二维码、版权声明、MIT 协议
- **页码格式预设**：5 种预设样式（1/35、第1页共35页、-1-、— 1 —、1）
- **显示总页数开关**：关闭后预设自动切换为不含 total 的样式
- **页眉/页脚独立开关**：可单独控制页眉、页脚文字、页码的插入
- **原有内容弹窗增强**：文件筛选下拉框、"同文件"批量选择按钮

### 修复
- **H1 取消功能接线**：SubprocessRegistry 现在注册 qpdf 子进程 PID，取消按钮生效
- **H2 run 索引统一**：save.rs 的 run_has_text() 匹配 scan.rs 行为，修复空 `<w:t>` 导致的字段错位
- **M6 中文数字统一**：core/numberFormat.js 支持 0-9999，消除两套实现的不一致
- **M7 页码预览样式一致**：预览现在正确显示中文/罗马/带圈数字样式
- **Per-file 页码偏移修复**：每个文件单独编号时，后续文件页码不再全部显示为 1
- **布局优化**：左侧面板加宽、预览面板可拖拽调整宽度、全局 padding/max-width 统一
- **L9 正则缓存**：batch.rs 日期正则改为 LazyLock 静态缓存
- **L10 死代码清理**：删除未使用的 getModule/getModuleSettings/get_module_registry
- **L12 恒等操作修复**：TemplateView 恒等三元表达式

### 文档
- README：修正字段类型数量（7→8）、补充实心带圈数字
- architecture.md：版本更新到 v0.9.2
- pdf-evidence-processing-design.md：修正区域覆盖兜底、A4 规范化、对象层删除描述
- template-system-design.md：补充 reference 字段类型

## [0.9.1] - 2026-08-04

### 打磨
- **错误提示改进**：qpdf 解锁/合并失败时现在显示具体原因（退出码和 stderr 文本）
- **错误来源标识**：前端错误弹窗自动检测出错工具并标注名称（如"qpdf（PDF 处理引擎）报错：…"）
- **设置页工具管理**：Docsy 托管的工具（qpdf/Poppler/FFmpeg）新增"清除此工具"按钮
- **设置页说明**：补充工具问题排查指引
- **操作计时显示**：Doclet 宠物动画显示已用时（如"已用时 2分15秒"）
- **操作取消支持**：长耗时操作显示"取消"按钮；qpdf 子进程已注册到进程组，可通过 SIGTERM/taskkill 可靠终止
- **证据处理状态列**：新增"状态"列，悬停显示详细错误（tooltip）
- **模板静默失败修复**：renderTemplate/exportBatch/batchRender 的 openPath 增加错误反馈
- **错误提示全面改进**：所有模块的错误提示增加具体原因和操作指引
- **Lint 修复**：window.clearInterval/setInterval 避免 no-undef，移除未使用变量

### 测试
- Rust: 141 个测试通过
- Frontend: 70 个测试通过（13 个测试文件）

## [0.9.0] - 2026-08-03

### 概述
0.9.0 是 Docsy 的首个成熟发布版本。所有模块经过全面审核和打磨，代码超过 41,000 行（Rust 20,600+ / 前端 20,400+），141 个 Rust 测试 + 70 个前端测试全部通过。

### 文档
- 重写 README，完整覆盖所有 6 个功能模块、技术架构、外部工具和开发指南
- 清理内部设计文档，将 10 份过时设计文档和飞书数据归档至 `Archived/docs-0.8/`
- 保留 4 份当前设计文档：架构总览、模板系统设计、PDF 证据处理设计、PDF 工作台设计

### PDF 原有内容处理
- 页眉、页脚文字和页码从检测到处理拆分为三类独立对象，页顶页码不再被当作普通页眉
- 新增原有内容确认窗口，按文件显示检测文字、页段、来源、编辑前后内容和处理决定
- 支持逐项及批量全选、反选、保留、忽略、删除和编辑；重新检测时保留用户决定
- 顶部仅显示非零的原页眉、原页脚文字、原页码、待删除和待编辑数量，点击即可核对
- 主文件列表移除重复的原页眉、原页脚、原页码、现有处理和状态列
- 普通文本删除继续按文字、页段和位置匹配，不使用白色遮盖；忽略项不进入处理负载
- 普通文本候选以跨页重复、位置稳定和覆盖范围为核心；内容格式只负责分类
- Docsy 写入继续采用标准 Pagination Artifact，并增加 ActualText、内容类型、版本和稳定 ID

### 页眉、页脚文字和页码
- 新页眉、页脚文字、页码使用三套独立设置，页脚文字不再承担页码职责
- 页码支持数字、中文数字、大小写罗马数字、带圈数字和实心带圈数字
- 页码格式支持 `{page}`、`{total}`、`{range}` 以及 `-{page}-` 等自定义前后符号
- 支持全部文件连续编号或每个文件单独编号
- 新增页码分段与例外窗口，可排除指定页段，或覆盖样式、格式、位置和编号偏移
- 分段规则按顺序覆盖，并合并为连续输出区间

### 排序与文件队列
- 所有会影响输出顺序的列表统一使用 Pointer Events 拖拽，不与 Tauri 文件拖入冲突
- PDF 合并、证据文件、证据扫描分组、合并证据页段、基础拆分页段均支持真实拖拽排序
- 图片排版和视频抽帧结果支持拖拽、上移和下移，并保留分页与缩放预览
- 图片排版后端接受显式图片路径顺序，最终 PDF/DOCX 与预览顺序一致

### 外部工具
- Windows Word 探测覆盖 HKLM/HKCU、32/64 位注册表、COM 注册、常见安装目录
- macOS Word 探测覆盖系统/用户 Applications、Launch Services、AppleScript 和 Spotlight
- WPS 增加 COM 注册及实际可执行文件兜底；LibreOffice 增加 Windows/macOS 默认目录
- Word 转 PDF 不再被可执行文件路径探测提前拦截，按 Word、WPS、LibreOffice 顺序实际尝试
- 设置页工具检测并行执行，明确区分尚未检测、检测失败、未安装和可用
- 长时间 Word/LibreOffice 转换支持继续等待

### 模板系统
- Word 标黄字段制作模板，保存为 `.docsytpl` zip 包
- 7 种字段类型：text / date / select / party_list / reference / checkbox / radio_group / checkbox_group
- 当事人列表表格行复制、空值规则（前缀/后缀删除 + 作用域控制）
- 批量填写：导出字段表为 Excel → 校验导入 → 批量生成 Word
- 模板回收站：删除/恢复/彻底删除，支持数据迁移为通用数据
- 历史与建议：lastValues / fieldSuggestions / semanticSuggestions / associationSuggestions
- quick-xml 结构化 XML 树引擎，坐标管线消除错位，格式保留，文本框标黄可扫描

### 图片排版
- 批量图片排版为 A4 文档（PDF / DOCX 输出）
- 文件夹分析、维度检测、前缀分组
- fit / fill / original 缩放模式
- 后端接受显式图片路径顺序

### 视频抽帧
- 按时间范围或帧率导出视频帧
- 视频信息读取、时间戳水印
- FFmpeg 检测与安装

### 首页
- 快捷入口卡片从 moduleRegistry 动态生成
- 真实版本号显示
- Doclet 宠物工作动画（350ms 防闪烁，16 帧 spritesheet）

### 设置
- 外部工具状态检测（qpdf / poppler / ffmpeg / Word / WPS / LibreOffice）
- 工具下载安装到 Docsy 托管目录，或使用系统已安装版本
- 主菜单排序与显示/隐藏
- 模板回收站管理
- 应用设置（LibreOffice 路径、工具清单地址）
- 诊断信息（版本、系统、工具版本）

### 修复
- PDF 解锁导入后检测加密状态，未加密文件不再重复处理
- PDF 解锁与合并的长文件列表可以正常滚动
- PDF 合并支持通过拖拽柄调整文件顺序
- PDF 基础工具支持从 Finder/资源管理器拖入文件
- FFmpeg 状态仅显示简短版本号，避免撑宽侧栏
- Windows Word/WPS 检测和模板批量填写相关问题
- WebKit NetworkCache 启动时自动清理

### 打磨
- **错误提示改进**：qpdf 解锁/合并失败时现在显示具体原因（退出码和 stderr 文本），而非笼统的"qpdf 解锁失败"
- **错误来源标识**：前端错误弹窗现在自动检测是哪个外部工具出错并标注工具名称（如"qpdf（PDF 处理引擎）报错：…"、"Poppler（PDF 文本检测）报错：…"）
- **设置页工具管理**：Docsy 托管的工具（qpdf/Poppler/FFmpeg）现在可以一键清除，清除后自动切换到系统已安装版本
- **设置页说明**：补充了工具问题排查指引——当某个工具导致处理失败时，可以清除托管版本后重新下载
- **操作计时显示**：Doclet 宠物动画现在显示已用时（如"已用时 2分15秒"），用户可以知道应用还在工作
- **操作取消支持**：长耗时操作显示"取消"按钮，点击可终止正在运行的子进程；qpdf 子进程已注册到进程组，SIGTERM/taskkill 可可靠终止
- **错误提示全面改进**：所有模块的错误提示都增加了具体原因和操作指引（如"PDF 合并失败，请确认文件未损坏且未被其他程序占用"）

### 测试
- Rust: 141 个测试通过（docx 模板引擎 / PDF 处理 / 图片排版 / 自然排序 / 历史记录 / 页码检测）
- Frontend: 70 个测试通过（13 个测试文件）

## [0.8.5] - 2026-08-03

### 修复
- Windows PDF processing fixes

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

## [0.8.2] - 2026-07-30

### 新增
- **select 字段类型**：下拉选择控件，支持案由/诉讼阶段等预设选项，可搜索可手输
- **批量填写功能**：导出字段表为 Excel → 校验导入 → 批量生成 Word
- **模板审核报告**：全模块设计 vs 实现对照审核

### 修复
- reference 字段前缀替换问题
- reference 字段允许用户显式覆盖前后缀
- 案号前缀/后缀在无用户覆盖时自动从 optionalRule 兜底
- 预览全文按段落聚合
- 回收站模板的历史记录不再显示
- Excel 日期单元格解析
- WPS Writer 检测改为查找实际 wps.exe 路径
- Microsoft Word 注册表查询添加 5 秒超时
- image-paddler filename_remove_text 不再被覆盖为空字符串

## [0.8.1] - 2026-07-21

### 新增
- 模板引擎从正则表达式改写为 **quick-xml 结构化 XML 树**
- 坐标管线：scan 和 save 共用 paragraph/run 索引，消除错位
- 拆分字段（同 run 子串）按 start/end 偏移精确定位
- 文本框中标黄可被扫描到
- 格式保留：渲染时保留原 run 的 `w:rPr`
- Doclet 工作动画 API（showLoading/hideLoading）
- WebKit NetworkCache 启动时自动清理

### 修复
- 模板字段结构在渲染期间保留
- 证据 PDF 页眉页脚编辑稳定性
- 模板历史在设置页操作后刷新

## [0.7.5] - 2026-07-14

### 修复
- PDF 已有页眉页脚的页码编辑
- 证据 PDF 页眉页脚编辑流程
- macOS 应用签名

## [0.6.0] - 2026-06-28

### 新增
- 证据 PDF 页眉页脚完整工作流：检测/删除/插入分离
- 页眉页脚插入开关
- 独立数字页脚检测
- PDF 预览中显示删除标记
- 证据表格自然排序
- 已有页眉页脚显式删除

## [0.5.3] - 2026-06-12

### 新增
- 字典 Excel 导入导出
- 模板 .docsytpl 打包
- 批量生成：Step wizard + Excel 导入 + 字段映射
- 配置导入导出：.docsybundle zip 包

## [0.5.2] - 2026-06-12

### 新增
- 设置页：外部工具状态检测、诊断信息展示
- 首页：真实模板列表和最近记录、快捷入口
- PDF 子页面：独立的解锁、合并、证据整理视图
- 图片排版视图：文件夹选择、参数配置、分析结果展示

## [0.5.1] - 2026-06-12

### 重构
- 将旧代码整体归档至 `Archived/` 目录
- 建立 v0.5 版本线，从零重建架构

### 新增
- 统一模块注册系统（前端 + 后端）
- vue-router 替代手动路由
- Pinia 状态管理
- 外部工具统一抽象层（ExternalTool trait）
- Tauri 命令自动收集机制
- 8 个功能模块注册入口
- SQLite 数据库 schema
- docx 引擎：quick-xml 模型解析 + 占位符渲染
- PDF 证据整理：文件夹扫描、自然排序、DOC/DOCX 转换、分组合并
- PDF 页眉页脚：printpdf 文字层生成 + qpdf --overlay 合成
- 图片排版：文件夹分析、维度检测、前缀分组、docx/pdf 输出
- 模板编辑器：加载 docx、标记字段、配置属性、保存 .docsytpl
- 视频抽帧：FFmpeg 检测、视频信息、抽帧设置、时间戳叠加
