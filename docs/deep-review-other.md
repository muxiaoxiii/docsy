# Docsy 深度审阅：video-extract、image-paddler、settings、shared 组件、Rust 外部工具层

> 审阅日期：2026-08-07
> 审阅范围：38 个文件（前端 16 + Rust 后端 22）
> 分支：codex/template-quickxml-0.8

---

## 目录

1. [逐文件分析](#1-逐文件分析)
   - [前端模块](#前端模块)
   - [共享组件](#共享组件)
   - [Rust 命令层](#rust-命令层)
   - [FFmpeg 封装层](#ffmpeg-封装层)
   - [外部工具管理层](#外部工具管理层)
   - [辅助模块](#辅助模块)
2. [外部工具管理机制](#2-外部工具管理机制)
3. [FFmpeg 封装层分析](#3-ffmpeg-封装层分析)
4. [共享组件复用模式](#4-共享组件复用模式)
5. [Bug 与问题汇总](#5-bug-与问题汇总)

---

## 1. 逐文件分析

### 前端模块

#### 1. `src/modules/video-extract/views/VideoExtractView.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 691 |
| **功能** | 视频抽帧工具的主界面。左栏为设置面板（FFmpeg 状态检测、视频文件拖放/选择、抽帧参数配置），右栏为抽帧结果图预览。支持按频率/按间隔两种抽帧模式、时间范围、时间戳水印、输出格式(JPG/PNG)和质量调整。 |
| **依赖** | `vue`(ref/reactive/onMounted), `@tauri-apps/plugin-dialog`(open), `element-plus`(ElMessage/icons), `tauriBridge.js`(tauriCallSafe/openExternalUrl), `ReorderableImageGrid`, `reorderableItems.js`(moveItem), `filePath.js`(fileName), `useWindowFileDrop` |
| **导出** | `<script setup>` 组件，无显式导出 |

**问题：**
- **L424-430 `isVideoPath` 路径解析脆弱**：先用 `split(/[\\/]/)` 取最后一段再 `.split('.').pop()`，如果文件名中没有 `.` 则 `pop()` 返回文件名本身，不会误判，但逻辑链过长且不够直观。建议使用 `filePath.js` 中已有的工具函数统一处理。
- **L373-374 fps 换算逻辑**：`settings.mode === 'fps' ? settings.fps : 1.0 / settings.interval`——当 interval 为 0 时会产生 `Infinity`，虽有 `el-input-number :min="0.1"` 保护，但后端未做二次校验（后端 `extract.rs:32` 会检查 `!fps.is_finite()`，所以不会崩溃，只是前端不友好）。
- **L455-463 `useWindowFileDrop`**：`onEnter`/`onLeave` 回调中直接设置 `dragging`，但 HTML 原生 `@dragover.prevent` 也会设置 `dragging = true`，两者可能重复触发。功能上不影响，但属于冗余逻辑。

---

#### 2. `src/modules/video-extract/index.js`

| 项目 | 内容 |
|------|------|
| **行数** | 31 |
| **功能** | video-extract 模块注册描述符。声明 id、名称、icon、category(order=50)、路由、菜单项和首页卡片。 |
| **依赖** | 无外部依赖 |

无问题。

---

#### 3. `src/modules/image-paddler/views/ImagePaddlerView.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 1077 |
| **功能** | 图片排版工具主界面。支持选择多个文件夹、自动分析图片尺寸并推荐布局参数、实时 A4 页面预览、自定义文件名规则（删除/替换/前缀/后缀/保留成分）、多种排列顺序(Z/N/倒N/自定义)、边框配置。输出支持 DOCX 和 PDF。 |
| **依赖** | `vue`(computed/ref/reactive/watch/onBeforeUnmount), `@tauri-apps/plugin-dialog`(open), `element-plus`(ElMessage), `tauriBridge.js`(openPath/tauriCallSafe), `ReorderableImageGrid`, `reorderableItems.js`(moveItem), `filePath.js`(fileName), `useWindowFileDrop` |
| **导出** | `<script setup>` 组件 |

**问题：**
- **L273 `analysisRequestId` 竞态**：使用 `analysisRequestId` 做请求去重，如果两次 `analyze()` 间隔极短，第二次的结果可能被第一次覆盖。代码通过 `if (requestId !== analysisRequestId) return` 来避免，逻辑正确。
- **L440 `openGeneratedOutput` 只打开第一个路径**：`generatedResult.value?.output_path`——但 `per_folder` 模式下会生成多个文件。应遍历 `generatedOutputPaths` 或提供选择。**Bug**。
- **L612 `createFilenameRule` 使用 `Date.now()`**：作为 Vue reactive 对象的 key，`Date.now()` 在同一毫秒内可能重复。不过此处用作 `v-for :key`，重复概率极低，影响有限。

---

#### 4. `src/modules/image-paddler/index.js`

| 项目 | 内容 |
|------|------|
| **行数** | 31 |
| **功能** | image-paddler 模块注册描述符。order=40，category=media。 |
| **依赖** | 无外部依赖 |

无问题。

---

#### 5. `src/modules/settings/views/SettingsView.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 677 |
| **功能** | 应用设置页。四大区块：① 外部工具状态检测与管理（qpdf/poppler/ffmpeg/word/wps/libreoffice，支持在线安装、本地 zip 安装、清除托管版本）；② 主菜单排序与可见性；③ 应用设置（LibreOffice 路径、工具清单地址）；④ 诊断信息。 |
| **依赖** | `vue`(computed/ref/reactive/onMounted), `@tauri-apps/plugin-dialog`(open), `element-plus`(ElMessage/ElMessageBox), `tauriBridge.js`(openExternalUrl/tauriCallSafe), `moduleRegistry.js`(defaultMenuOrder/getMenuModules) |
| **导出** | `<script setup>` 组件 |

**问题：**
- **L297 `settings.value` 为 ref 对象**：`settings` 声明为 `ref({...})`，后续通过 `settings.value.xxx` 访问。但模板中直接写 `settings.libreoffice_path` 而非 `settings.value.libreoffice_path`——Vue 3 的 `ref` 在模板中会自动解包，所以这是正确的。但 `settings.value = { ...settings.value, ...payload }` 这种写法在复杂操作时容易丢失响应性。
- **L383 `syncDiagnosticToolStatus` 硬编码工具名**：`['qpdf', 'poppler', 'ffmpeg']`——如果未来添加新工具到诊断面板，需要同时修改此处。建议从 `tools` 数组动态生成。

---

#### 6. `src/modules/settings/views/AboutView.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 158 |
| **功能** | 关于页面。展示 logo、版本号、GitHub 链接、小红书二维码、版权声明。 |
| **依赖** | `@tauri-apps/plugin-shell`(open) |

**问题：**
- **L37-39 版本号来源不一致**：`import.meta.env.PACKAGE_VERSION` 作为 fallback `'0.9.6'`，而 HomeView 使用 `tauriCallSafe('get_diagnostic_info')` 获取版本。两处可能不同步。AboutView 应该也从后端获取版本号以保持一致。
- **L41 `openGitHub` 使用 `@tauri-apps/plugin-shell` 的 `open`**：而其他页面使用 `tauriBridge.js` 的 `openExternalUrl`。应统一使用 `openExternalUrl` 以获得 URL 安全校验（HTTPS 校验）。**代码风格不一致**。

---

#### 7. `src/modules/settings/index.js`

| 项目 | 内容 |
|------|------|
| **行数** | 29 |
| **功能** | settings 模块注册描述符。order=900，category=system。注册两个路由：`/settings` 和 `/about`。menuItems 为空数组（设置入口由框架控制）。 |

无问题。

---

#### 8. `src/modules/home/HomeView.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 177 |
| **功能** | 首页视图。展示品牌 hero 区域（logo + 版本号）、快捷入口卡片网格。从 `moduleRegistry.js` 获取卡片列表，支持 `docsy-settings-updated` 事件实时更新。 |
| **依赖** | `vue`(computed/onBeforeUnmount/onMounted/ref), `vue-router`(useRouter), `moduleRegistry.js`(getHomeCards), `tauriBridge.js`(tauriCallSafe) |
| **导出** | `<script setup>` 组件 |

无重大问题。

---

#### 9. `src/modules/home/index.js`

| 项目 | 内容 |
|------|------|
| **行数** | 10 |
| **功能** | home 模块注册描述符。无路由、无菜单项、无首页卡片。category=system。 |

无问题。

---

### 共享组件

#### 10. `src/shared/components/ToolWorkspaceShell.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 117 |
| **功能** | 通用工具工作区外壳组件。提供 header（标题+描述+操作区）、可选 toolbar、内容区（slot）、底部 actions 四个插槽。 |
| **Props** | `title: String(required)`, `description: String(default '')` |
| **依赖** | 无外部依赖 |

无问题。设计简洁，复用性好。

---

#### 11. `src/shared/components/FileQueuePanel.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 204 |
| **功能** | 文件队列列表面板。支持拖拽排序（使用 `usePointerReorder`）、删除、清空、自定义 leading/meta/actions 插槽。 |
| **Props** | `items: Array`, `emptyText: String`, `maxHeight: String`, `clearable: Boolean`, `removable: Boolean`, `sortable: Boolean` |
| **Emits** | `clear`, `remove`, `reorder` |
| **依赖** | `element-plus`(Rank icon), `usePointerReorder` |

**问题：**
- **L93-98 `itemLabel`/`itemKey` 防御性不足**：`item?.name || item?.path || item || ''`——如果 `item` 是数字 0，`item || ''` 会返回空字符串。不过在文件队列场景中 `item` 不太可能是 0，风险极低。

---

#### 12. `src/shared/components/ReorderableImageGrid.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 347 |
| **功能** | 可重排图片网格组件。支持分页、缩放、拖拽排序（`usePointerReorder`）、上下移动按钮、点击预览大图。图片通过 `tauriCallSafe('read_image_data_url')` 按需加载并缓存（3 并发 worker），切换分页时清理不可见图片缓存。 |
| **Props** | `items: Array`, `nameResolver: Function`, `metaResolver: Function`, `pathResolver: Function`, `emptyDescription: String`, `pageSizeOptions: Array`, `initialPageSize: Number`, `initialZoom: Number`, `minZoom: Number`, `maxZoom: Number` |
| **Emits** | `reorder` |
| **依赖** | `vue`(computed/reactive/ref/watch), `element-plus`(ArrowDown/ArrowUp/Rank icons), `tauriBridge.js`(tauriCallSafe), `filePath.js`(fileName), `usePointerReorder` |

**问题：**
- **L171-173 缓存清理策略**：`for (const path of Object.keys(sources)) { if (!keep.has(path)) delete sources[path] }`——删除 reactive 对象的属性不会触发 Vue 响应式更新。应使用 `delete sources[path]` 时 Vue 3 的 Proxy 可以追踪，但若 sources 不是 reactive 的则不行。此处 `sources` 声明为 `reactive({})`，所以是 OK 的。
- **L175-185 并发加载**：使用 `Array.from({ length: Math.min(3, queue.length) })` 创建 worker，`queue.shift()` 在多个 async worker 间竞争——这在 JS 单线程中是安全的（每个 await 后检查 queue），但逻辑上不太明显。

---

#### 13. `src/shared/components/ImagePreviewGrid.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 317 |
| **功能** | 图片预览网格（只读版）。与 `ReorderableImageGrid` 功能类似，但不含排序功能。支持分页、缩放、点击预览。 |
| **Props** | 与 `ReorderableImageGrid` 相同（除无 reorder 相关） |
| **依赖** | `vue`(computed/reactive/ref/watch), `tauriBridge.js`(tauriCallSafe), `filePath.js`(fileName) |

**问题：**
- **与 ReorderableImageGrid 大量重复代码**：两个组件的图片加载逻辑（preloadVisibleImages）、分页逻辑、缩放逻辑几乎完全相同。**建议提取为 composable `useImageGrid`**。这是当前共享组件中最大的代码重复问题。
- **L160 `keep.add(previewSrc.value)`**：预览图片的 path 也会被保留在缓存中，这是一个好设计。但 `ReorderableImageGrid` 中没有这个逻辑（L170-173），切换分页后预览缓存会被清理。**行为不一致**。

---

#### 14. `src/shared/components/DocletWorkingPet.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 141 |
| **功能** | Doclet 工作状态动画宠物组件。使用 spritesheet 帧动画（16 帧 "look around" 动画，5.6s 循环）。支持 `prefers-reduced-motion` 媒体查询。 |
| **Props** | `message: String(default 'Doclet 正在处理…')`, `elapsed: String(default '')` |
| **依赖** | 静态资源 `doclet-v2-spritesheet.webp` |

**问题：**
- **L47 `background-size: 1152px 1716px`**：硬编码 spritesheet 尺寸。如果 spritesheet 图片更换，需要同步修改 CSS。建议通过 props 或 CSS 变量注入。
- **L71-133 `@keyframes` 硬编码 16 帧**：每帧 6.25%（100/16），总时长 5.6s。帧位置完全硬编码，与 spritesheet 布局强耦合。

---

#### 15. `src/shared/components/reorderableItems.js`

| 项目 | 内容 |
|------|------|
| **行数** | 8 |
| **功能** | 导出 `moveItem(items, from, to)` 函数：在数组中移动元素位置，返回新数组。包含边界检查。 |
| **导出** | `moveItem(items, from, to)` |

无问题。简洁实用。

---

#### 16. `src/components/UndoRedoButtons.vue`

| 项目 | 内容 |
|------|------|
| **行数** | 51 |
| **功能** | 撤销/重做按钮组。支持 compact 模式，显示 tooltip 快捷键提示。 |
| **Props** | `canUndo: Boolean`, `canRedo: Boolean`, `compact: Boolean` |
| **Emits** | `undo`, `redo` |
| **依赖** | `element-plus`(RefreshLeft/RefreshRight icons) |

**问题：**
- **L7-8 `compact ? 'small' : 'small'`**：三元表达式两个分支相同，`compact` prop 实际上只影响 `iconSize`（L38: `compact ? 12 : 14`）和外层 class，不影响按钮 size。这是**冗余代码**，应简化为 `size="small"`。
- **L6 `iconSize`**：`compact ? 12 : 14`——差异仅 2px，视觉效果微乎其微。

---

### Rust 命令层

#### 17. `src-tauri/src/commands/video.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 49 |
| **功能** | Tauri 命令：`check_ffmpeg`（检测 FFmpeg 状态+drawtext 支持）、`probe_video`（探测视频信息）、`extract_frames`（执行抽帧）、`list_output_frames`（列出输出帧文件）。 |
| **依赖** | `crate::external::ExternalTool`, `crate::ffmpeg::{detect, extract, probe}` |

**问题：**
- **L14 `check_ffmpeg` 使用 `spawn_blocking`**：在 blocking 线程中执行 `build_ffmpeg_status`，但 `has_drawtext()` 会启动子进程。如果 FFmpeg 路径无效，`binary_path()` 会报错，但 `has_drawtext()` 内部没有捕获这个错误——它会传播 `?` 到 `build_ffmpeg_status`，但由于 `build_ffmpeg_status` 返回 `FfmpegStatus` 而非 `Result`，`unwrap_or(false)` 会吞掉错误。**逻辑正确但信息丢失**——用户看不到为什么 drawtext 检测失败。

---

#### 18. `src-tauri/src/commands/image_paddler.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 23 |
| **功能** | Tauri 命令：`analyze_image_paddler_folder`（分析图片文件夹）、`run_image_paddler`（执行排版生成）。 |
| **依赖** | `crate::image_paddler` |

无问题。简洁的命令桥接层。

---

#### 19. `src-tauri/src/commands/settings.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 62 |
| **功能** | Tauri 命令：`get_app_settings`/`set_app_settings`（读写设置）、`check_external_tool`（检测单个工具）、`install_external_tool`（在线安装）、`install_external_tool_from_package`（本地 zip 安装）、`get_managed_tools_dir`/`open_managed_tools_dir`（工具目录）、`remove_managed_tool`（清除托管工具）。 |
| **依赖** | `crate::external`, `crate::services::history` |

**问题：**
- **L31-39 `install_external_tool_from_package`**：先安装再验证，但安装失败时不会清理。`install_tool_from_package` 内部有 staging 目录机制（`managed.rs:111-143`），失败会清理 staging，所以实际上没问题。
- **L43 `get_managed_tools_dir`**：同步函数，直接返回路径字符串。设计合理。

---

#### 20. `src-tauri/src/commands/system.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 145 |
| **功能** | 系统级 Tauri 命令集合：`open_path`（打开文件/目录）、`open_external_url`（打开 HTTPS URL）、`write_frontend_log`（前端日志写入）、`get_log_file_path`/`open_log_file`/`open_log_dir`（日志管理）、`read_image_data_url`（图片缩略图 base64）、`get_diagnostic_info`（诊断信息）、`list_system_fonts`（系统字体列表）、`respond_conversion_timeout`（转换超时响应）、`cancel_operation`（取消操作）。 |
| **依赖** | `crate::app_log`, `crate::ffmpeg::detect`, `crate::ConversionState`, `crate::SubprocessRegistry`, `image` crate, `base64` crate |

**问题：**
- **L12-14 `open_external_url` 安全限制**：只允许 `https` scheme——但如果用户需要打开 `http` 内网地址则无法使用。这是有意的安全设计。
- **L54-89 `preview_image_data_url`**：所有图片先解码为全尺寸 `image::DynamicImage`，再 `thumbnail(1600, 1600)`。对于超大图片（如 20000×20000），即使有 `MAX_SOURCE_PIXELS` 限制（64M），解码仍可能消耗大量内存。**潜在 OOM 风险**，但 64M 像素 × 4 bytes = 256MB，对现代机器可接受。
- **L87 `let _ = mime;`**：`mime` 变量被声明但未使用（被 `let _ =` 丢弃）。这是**死代码**——原本可能用于选择输出格式，但后来改为统一输出 JPEG。

---

#### 21. `src-tauri/src/commands/mod.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 104 |
| **功能** | 命令模块入口。声明子模块、提供 `run_blocking` 通用异步桥接函数、`build_handler` 注册所有 Tauri 命令。 |
| **导出** | `run_blocking<T, F>`, `build_handler() -> impl Fn(Invoke) -> bool` |

**问题：**
- **L8-17 `run_blocking`**：使用 `spawn_blocking` + 双层 `map_err`——第一层捕获 JoinError（线程 panic），第二层捕获业务错误。设计合理，是 `commands/video.rs` 中 `list_output_frames` 使用的模式。

---

### FFmpeg 封装层

#### 22. `src-tauri/src/ffmpeg/detect.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 48 |
| **功能** | `list_system_fonts()`：扫描系统字体目录返回字体文件名列表。`has_drawtext()`：执行 `ffmpeg -filters` 检查是否支持 drawtext 滤镜。 |
| **依赖** | `crate::external::ExternalTool`, `crate::external::hidden_command` |

**问题：**
- **L4-31 `list_system_fonts`**：只扫描硬编码目录，不递归（只读一层）。Linux 上 `/usr/share/fonts` 下通常有多层子目录，但 `read_dir` 只读顶层。**Bug：Linux 上可能遗漏大部分字体**。macOS 和 Windows 的字体目录通常较平，影响较小。

---

#### 23. `src-tauri/src/ffmpeg/extract.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 417 |
| **功能** | 视频抽帧核心逻辑。`extract()` 构建 FFmpeg 命令执行抽帧，支持 fps/interval 模式、时间范围、drawtext 水印。使用临时文件前缀 + 重命名策略避免文件冲突。包含完整的单元测试。 |
| **依赖** | `crate::external::{ExternalTool, hidden_command, command_output_with_idle_timeout}`, `chrono`, `anyhow` |

**问题：**
- **L28-29 临时文件命名**：`format!(".docsy_tmp_{run_id}")` 使用 `chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")`——毫秒精度。在同一毫秒内多次调用可能冲突，但概率极低。
- **L79 `timeline_offset_seconds` 参数**：`time_range.start.unwrap_or(0.0)`——当只有 endTime 没有 startTime 时，`start` 为 `None`，偏移量为 0，这是正确的。
- **L324-338 `unique_frame_path`**：当文件名冲突时，使用 `{stem}_{idx}.{ext}` 命名——但这会丢失时间戳信息。对于抽帧场景，用户可能期望保留时间戳命名。

---

#### 24. `src-tauri/src/ffmpeg/probe.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 110 |
| **功能** | 视频信息探测。使用 ffprobe 获取 JSON 格式的流/格式信息，提取 duration/width/height/fps/codec/size。 |
| **依赖** | `crate::external::{ExternalTool, hidden_command, command_output_with_idle_timeout}` |

**问题：**
- **L10-13 ffprobe 路径推导**：从 ffmpeg 二进制路径的父目录拼接 `ffprobe`——这依赖于 ffmpeg 和 ffprobe 在同一目录。对于托管安装（`managed.rs` 要求 `binaries: ["ffmpeg", "ffprobe"]`），这是正确的。对于系统安装，通常也在同一目录。
- **L66-71 fps 解析**：`avg_frame_rate` 优先于 `r_frame_rate`——这是正确的，`avg_frame_rate` 更准确。

---

#### 25. `src-tauri/src/ffmpeg/mod.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 3 |
| **功能** | 模块入口，声明 `detect`、`extract`、`probe` 子模块。 |

无问题。

---

### 外部工具管理层

#### 26. `src-tauri/src/external/mod.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 296 |
| **功能** | 外部工具管理核心。定义 `ToolStatus` 结构体和 `ExternalTool` trait（check/try_install/binary_path）。提供 `hidden_command`（跨平台隐藏命令窗口）、`check_by_name`/`install_by_name`/`validate_tool`（按名称调度工具）、`command_failure_detail`（错误格式化）、`command_output_with_timeout`（超时执行）、`command_output_with_idle_timeout`（空闲超时执行，基于流式读取）。 |
| **导出** | `ToolStatus`, `ExternalTool` trait, `hidden_command`, `check_by_name`, `install_by_name`, `validate_tool`, `command_failure_detail`, `command_output_with_timeout`, `command_output_with_idle_timeout`, 各工具 re-export |
| **依赖** | `serde`, `std::process`, `std::sync::mpsc`, `std::thread` |

**问题：**
- **L121-155 `command_output_with_timeout` 实现**：使用 `try_wait()` 轮询（30ms 间隔），而非 `wait_timeout()`。这是因为 Rust 标准库没有 `wait_timeout`，而 `try_wait` + sleep 是常见模式。但 30ms 间隔意味着超时误差最大 30ms，可接受。
- **L157-202 `command_output_with_idle_timeout`**：使用独立线程读取 stdout/stderr，通过 mpsc channel 通知主线程。空闲超时检测基于 `last_activity` 时间戳。**设计精巧**，能有效处理长时间运行但无输出的命令（如 FFmpeg 抽帧卡住）。
- **L220-233 `spawn_stream_reader`**：每次 `read` 分配一个新的 `Vec<u8>`（`buffer[..read].to_vec()`），高频输出时可能产生大量小分配。但实际使用中 FFmpeg 输出频率不高，影响有限。

---

#### 27. `src-tauri/src/external/managed.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 667 |
| **功能** | 托管工具下载/安装/管理核心。支持：① 从远程 manifest JSON 下载工具包（SHA256 校验）；② 本地 zip 包安装；③ 嵌入式 Windows 工具包配置（qpdf/ffmpeg/poppler 含 SHA256）；④ 自动重定向跟踪（最多 5 次，强制 HTTPS）；⑤ 安装流程（下载→校验→解压→staging→原子替换→写入安装记录）；⑥ 工具查找（托管目录→系统 PATH）。 |
| **依赖** | `reqwest::blocking`, `sha2`, `zip`, `dirs`, `anyhow`, `serde` |

**关键流程：**
```
install_tool(name) → load_package_spec → download_package_to_temp_file
  → verify_sha256_file_if_present → extract_zip_file → make_executable
  → atomic rename (staging → install_dir) → write .docsy-tool.json
```

**问题：**
- **L158-195 `load_package_spec` 优先级**：① 环境变量 `DOCSY_TOOL_MANIFEST_URL` → ② 用户设置 `tool_manifest_url` → ③ 嵌入式配置 → ④ 默认 GitHub manifest。但条件是 `has_sha256`——如果用户自定义清单缺少 SHA256，会静默跳过继续尝试下一个源。**可能导致用户困惑**：明明配了清单却不生效。
- **L212-229 `embedded_package_spec`（非 Windows）**：返回的 `sha256` 为空字符串，`has_sha256` 检查会失败。这意味着**macOS 上嵌入式配置永远不会被使用**——必须依赖远程 manifest 或用户自定义清单。**Bug 或设计缺陷**。
- **L265-313 `download_package` 和 L331-389 `download_package_to_temp_file`**：两个函数高度重复（URL 校验、重定向逻辑、大小限制）。应提取公共下载逻辑。
- **L370 `response.take(max_bytes + 1)`**：取 `max_bytes + 1` 字节来检测是否超限——这是 `reqwest::Response::take` 的标准用法，多读 1 字节用于边界检测。
- **L469-502 `extract_zip_file`**：使用 `enclosed_name()` 防止 zip slip 攻击——这是 `zip` crate 的安全特性。**安全设计良好**。
- **L504-529 `find_binary_in_dir`**：先检查直接路径和 `bin/` 子目录，再 DFS 搜索整个目录树。DFS 在大型工具包（如 FFmpeg）中可能较慢，但只在安装后验证时调用，可接受。

---

#### 28. `src-tauri/src/external/ffmpeg.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 93 |
| **功能** | FFmpeg 工具实现。`check()` 执行 `ffmpeg -version` 检测可用性。`binary_path()` 查找顺序：托管目录 → 已知 macOS 路径（/opt/homebrew, /usr/local）→ PATH。 |
| **依赖** | `super::{ExternalTool, ToolStatus, hidden_command, command_output_with_timeout, managed}` |

**问题：**
- **L64-68 已知路径硬编码**：`["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"]`——只适用于 macOS。Windows 用户完全依赖托管安装或 PATH。
- **L80-88 `binary_name` 函数**：`match name { "ffmpeg" => "ffmpeg.exe", _ => "ffmpeg.exe" }`——两个分支返回相同值。**冗余代码**，应简化为 `if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }`。

---

#### 29. `src-tauri/src/external/libreoffice.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 183 |
| **功能** | LibreOffice 工具实现。`check()` 只检测路径可用性（不获取版本）。`binary_path()` 查找顺序：用户设置路径 → 已知安装路径 → PATH (soffice)。支持 macOS/Windows/Linux 三平台已知路径。`resolve_libreoffice_path` 支持传入目录自动定位 soffice 二进制。 |
| **依赖** | `super::{ExternalTool, ToolStatus, hidden_command, command_output_with_timeout}` |

**问题：**
- **L35-40 设置路径优先级**：如果用户在设置中配置了 LibreOffice 路径，会优先使用。但 `resolve_libreoffice_path` 对无效路径返回 `None`，然后静默 fallthrough 到已知路径。**用户配置了错误路径时不会报错**，可能造成困惑。

---

#### 30. `src-tauri/src/external/word.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 265 |
| **功能** | Microsoft Word 工具实现。Windows 检测策略：注册表 → 已知路径 → PATH → COM 注册。macOS 检测策略：已知路径 → mdfind → LaunchServices。 |
| **依赖** | `super::{ExternalTool, ToolStatus, hidden_command, command_output_with_timeout}` |

**问题：**
- **L55-57 `windows_com_registered`**：使用 PowerShell 检测 COM 注册——这是最慢的检测方式（需要启动 PowerShell），但作为最后的 fallback 可以接受。
- **L68-69 macOS fallback**：`macos_launch_services_has_word()` 使用 `open -Ra "Microsoft Word"` 检测——这会尝试激活 Word（如果已安装），可能产生副作用（Word 窗口闪现）。**应优先使用 mdfind**。

---

#### 31. `src-tauri/src/external/wps.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 258 |
| **功能** | WPS Writer 工具实现。Windows 检测策略：注册表（KWPS.Application CLSID → LocalServer32）→ 已知路径 → PATH → COM 注册。macOS 检测策略：已知路径 → mdfind。 |
| **依赖** | `super::{ExternalTool, ToolStatus, hidden_command, command_output_with_timeout}` |

**问题：**
- **L62-63 COM 注册检测**：尝试 `KWPS.Application` 和 `kwps.Application` 两个 ProgID——WPS 不同版本可能使用不同的大小写。**设计合理**。
- **L160-167 `extract_reg_path`**：处理注册表中的带引号路径（`"C:\...\wps.exe" "%1"`）和不带引号路径。逻辑正确。

---

### 辅助模块

#### 32. `src-tauri/src/image_paddler.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 1587 |
| **功能** | 图片排版核心引擎。`analyze()` 扫描图片文件夹、构建分组、推荐设置。`run()` 执行排版（支持 per_folder 模式和 explicit_image_paths），生成 PDF（printpdf）或 DOCX（docx-rs）。包含完整的布局计算、文件名处理、CJK 字体嵌入、单元测试。 |
| **依赖** | `image`, `printpdf`, `docx_rs`, `regex`, `chrono`, `crate::sort_utils::natural_cmp` |

**问题：**
- **L522 `margin_mm` 默认值不一致**：`args.margin_mm.unwrap_or(15.0)` 但 `recommend_settings` 返回 `margin_mm: 12.0`。前端默认也是 12mm。**Bug：如果用户不设置 margin，后端使用 15mm 而前端推荐 12mm**。
- **L525 `filename_without_ext` 默认值**：`unwrap_or(false)` 但前端默认 `true`（ImagePaddlerView.vue:309）。**前后端默认值不一致**。
- **L913-1082 `generate_pdf`**：函数签名有 17 个参数（`#[allow(clippy::too_many_arguments)]`）。虽然功能正确，但可维护性差。建议使用配置结构体。
- **L948-959 CJK 字体处理**：如果找不到 CJK 字体，PDF 中的中文文件名会被省略并生成 warning。DOCX 输出则不受影响（使用系统字体）。**合理的降级策略**。

---

#### 33. `src-tauri/src/sort_utils.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 83 |
| **功能** | 自然排序比较函数 `natural_cmp`。处理字符串中嵌入的数字（前导零归一化、按数值比较）。 |
| **导出** | `natural_cmp(left: &str, right: &str) -> Ordering` |

**问题：**
- **L74-81 测试用例**：`"img_1"` < `"img_001"` < `"img_2"`——前导零较多的排在后面（因为原始长度更长）。这是**有意设计**：先比较归一化后的数值，相等时比较原始长度。

---

#### 34. `src-tauri/src/app_log.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 152 |
| **功能** | 应用日志系统。JSON Lines 格式写入日志文件（按日期分文件）。支持前端日志写入、panic hook 安装、旧日志自动清理（保留 14 天）。 |
| **导出** | `log_dir`, `log_file_path`, `init`, `install_panic_hook`, `write_frontend`, `info`, `error`, `list_log_files` |
| **依赖** | `chrono`, `serde`, `serde_json`, `dirs` |

**问题：**
- **L33-43 `init`**：先清理旧日志再写入启动日志——如果清理失败（权限问题），不影响启动日志写入。
- **L46-72 `install_panic_hook`**：panic hook 中写入日志后调用 `previous(panic_info)` 保留默认行为（打印到 stderr）。**设计合理**。
- **L145-150 日志写入**：每次写入都 `OpenOptions::new().create(true).append(true).open()`——没有文件锁。在多线程并发写入时可能产生交错输出，但 JSON Lines 格式对行交错有一定容忍度。

---

#### 35. `src-tauri/src/lib.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 223 |
| **功能** | 应用入口。定义 `SubprocessRegistry`（子进程 PID 注册/取消）、`ConversionState`（COM 转换超时状态机）、全局 `APP_HANDLE` 和 `SUBPROCESS_REGISTRY`。`run()` 初始化日志、清理 WebKit 缓存、构建 Tauri 应用。 |
| **依赖** | `tauri`, `tauri_plugin_dialog`, `tauri_plugin_fs`, `tauri_plugin_shell`, `dirs` |

**问题：**
- **L59-62 `cancel` 方法**：使用 `kill -TERM` 而非直接 SIGKILL——给予进程优雅退出机会。但没有后续检查进程是否真的退出。
- **L85-96 `spawn_and_wait`**：注册 PID → wait → 注销。如果进程 panic，PID 会被正确注销（因为 `wait_with_output` 会返回）。**设计正确**。
- **L157-180 `wait_for_user_response`**：使用 500ms 轮询等待用户响应——这是一种低效的等待方式，但因为需要跨线程（Tauri 命令线程 → 前端事件 → 命令线程），且 Tauri 没有原生的"等待前端响应"机制，这是合理的折中。
- **L212-222 `cleanup_webkit_cache`**：在独立线程中清理 WebKit 网络缓存——不阻塞应用启动。**好设计**。

---

#### 36. `src-tauri/src/main.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 5 |
| **功能** | 应用入口点。调用 `docsy_lib::run()`。 |

无问题。

---

#### 37. `src-tauri/src/services/mod.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 2 |
| **功能** | 服务模块入口，声明 `history` 和 `module_registry` 子模块。 |

无问题。

---

#### 38. `src-tauri/src/services/module_registry.rs`

| 项目 | 内容 |
|------|------|
| **行数** | 52 |
| **功能** | 后端模块注册表。`all_descriptors()` 返回所有模块的 JSON 描述符（目前标记为 `#[allow(dead_code)]`）。 |
| **依赖** | `serde_json` |

**问题：**
- **L3 `#[allow(dead_code)]`**：函数未被使用——后端模块注册实际上由前端 `src/modules/*/index.js` 驱动。这个函数可能是预留的，或者是历史遗留。**死代码**。
- **描述符与前端不完全一致**：后端有 `sub_modules` 字段（PDF 工具的 unlock/merge/evidence），但前端模块注册中没有对应结构。

---

## 2. 外部工具管理机制

### 架构概览

```
前端 SettingsView.vue
  ├── check_external_tool(name) → ExternalTool::check()
  ├── install_external_tool(name) → ExternalTool::try_install()
  ├── install_external_tool_from_package(name, path) → managed::install_tool_from_package()
  └── remove_managed_tool(name) → managed::remove_managed_tool()

ExternalTool trait
  ├── check() → ToolStatus { available, path, version, managed, source }
  ├── try_install() → Result<String>
  └── binary_path() → Result<PathBuf>

managed.rs (下载/安装引擎)
  ├── install_tool(name) → load_package_spec → download → verify SHA256 → extract → atomic rename
  ├── install_tool_from_package(name, path) → 本地 zip 安装
  ├── tools_root() → {data_dir}/Docsy/tools/{tool_name}
  └── managed_binary_path(tool, binary) → 查找托管二进制
```

### 工具查找优先级

| 工具 | 查找顺序 |
|------|----------|
| **FFmpeg** | 托管目录 → /opt/homebrew/bin → /usr/local/bin → PATH |
| **qpdf** | 托管目录 → PATH |
| **Poppler** | 托管目录 → PATH |
| **LibreOffice** | 用户设置路径 → 已知安装路径 → PATH (soffice) |
| **Word** | 注册表(Windows) / 已知路径(macOS) → PATH → COM/LaunchServices |
| **WPS** | 注册表(Windows) / 已知路径(macOS) → PATH → COM |

### 安全机制

1. **HTTPS-only 下载**：`validate_download_url` 强制所有下载 URL 使用 HTTPS
2. **SHA256 校验**：远程下载的工具包必须包含 SHA256 校验值
3. **重定向安全**：最多 5 次重定向，每次重定向后验证 HTTPS
4. **大小限制**：下载和解压都有工具级别的大小限制（ffmpeg 2GB 下载 / 4GB 解压）
5. **Zip Slip 防护**：使用 `enclosed_name()` 防止路径穿越
6. **原子安装**：staging 目录 → rename 原子替换，失败自动回滚

### 版本管理

- 安装完成后写入 `.docsy-tool.json` 记录（name/platform/version/url/binaries）
- **无版本升级机制**：安装新版本时直接覆盖旧版本（通过 staging → rename），但没有版本比较和自动更新
- **无版本回退**：清除后需重新下载

---

## 3. FFmpeg 封装层分析

### 调用链

```
VideoExtractView.vue
  ├── check_ffmpeg → commands/video.rs::check_ffmpeg → FfmpegTool::check() + has_drawtext()
  ├── probe_video → commands/video.rs::probe_video → ffmpeg/probe.rs::probe_video()
  ├── extract_frames → commands/video.rs::extract_frames → ffmpeg/extract.rs::extract()
  └── list_output_frames → commands/video.rs::list_output_frames → ffmpeg/extract.rs::list_output_frames()
```

### 核心设计

1. **空闲超时**：抽帧使用 5 分钟空闲超时（`FFMPEG_EXTRACT_IDLE_TIMEOUT`），探测使用 45 秒。防止 FFmpeg 卡死。
2. **临时文件策略**：使用 `.docsy_tmp_{timestamp}_{pid}` 前缀，完成后重命名为最终文件名（含时间戳信息）。
3. **drawtext 水印**：通过 `ffmpeg -filters` 检测支持，使用 `drawtext` 滤镜叠加 PTS 时间戳。
4. **质量映射**：前端 quality 1-100 映射到 FFmpeg qscale 31-2（`jpeg_qscale` 函数）。

### 问题

- **probe.rs 不使用空闲超时的错误消息**：`anyhow::bail!("ffprobe 失败")` 没有包含 stderr 信息，用户无法得知具体原因。
- **extract.rs 的 fps 参数校验**：前端传入 `fps: 1.0 / settings.interval`，当 interval 很小时 fps 很大，但后端没有上限检查。

---

## 4. 共享组件复用模式

### 复用关系图

```
ToolWorkspaceShell.vue ─── 被 pdf-tools/template 等模块使用（通用布局壳）
FileQueuePanel.vue ─────── 被 pdf-tools 等模块使用（文件列表+排序）
ReorderableImageGrid.vue ── 被 video-extract、image-paddler 使用（可排序图片网格）
ImagePreviewGrid.vue ───── 被 template 模块使用（只读图片预览网格）
DocletWorkingPet.vue ───── 被处理中状态页面使用（动画宠物）
reorderableItems.js ────── 被 ReorderableImageGrid 和 ImagePaddlerView 使用（数组移动）
UndoRedoButtons.vue ────── 被 template 模块使用（撤销/重做）
usePointerReorder.js ───── 被 FileQueuePanel 和 ReorderableImageGrid 使用（拖拽排序）
useWindowFileDrop.js ───── 被 VideoExtractView 和 ImagePaddlerView 使用（窗口文件拖放）
```

### 代码重复问题

**`ReorderableImageGrid.vue` 与 `ImagePreviewGrid.vue`** 存在大量重复：
- 图片加载逻辑（preloadVisibleImages + 3 worker 并发）
- 分页逻辑（page/pageSize/pageCount/pageStart/pageEnd/pagedItems）
- 缩放逻辑（zoom/cardSize/thumbSize/gridStyle/cardStyle/thumbWrapStyle）
- 预览对话框逻辑
- 工具函数（itemPath/itemName/itemMeta/itemKey/imageSrc/adjustZoom）

**建议**：提取 `useImageGrid(props)` composable，两个组件共享核心逻辑。

### 设计模式

1. **Props + Slots 模式**：ToolWorkspaceShell 和 FileQueuePanel 使用命名插槽提供灵活的内容注入。
2. **Resolver 模式**：ReorderableImageGrid 和 ImagePreviewGrid 通过 `nameResolver`/`metaResolver`/`pathResolver` props 支持不同数据格式。
3. **Composable 模式**：`usePointerReorder` 和 `useWindowFileDrop` 封装可复用的交互逻辑。
4. **emit 模式**：子组件通过 emit 通知父组件，不直接修改 props。

---

## 5. Bug 与问题汇总

### 🔴 Bug（影响功能）

| # | 文件 | 行号 | 严重性 | 描述 |
|---|------|------|--------|------|
| 1 | `image_paddler.rs` | 522 | **中** | `margin_mm` 默认值 15.0 与前端推荐值 12.0 不一致。用户不设置 margin 时前后端行为不同。 |
| 2 | `image_paddler.rs` | 525 | **低** | `filename_without_ext` 默认 `false`，前端默认 `true`。仅影响不设置此参数的调用。 |
| 3 | `ImagePaddlerView.vue` | 440 | **中** | `openGeneratedOutput` 只打开 `output_path`（第一个），`per_folder` 模式生成多个文件时用户只能打开第一个。 |
| 4 | `detect.rs` | 4-31 | **低** | `list_system_fonts` 不递归扫描子目录，Linux 上可能遗漏大部分字体。 |
| 5 | `managed.rs` | 212-229 | **中** | macOS/Linux 嵌入式配置 `sha256` 为空，`has_sha256` 检查失败，嵌入式配置永远不生效。必须依赖远程 manifest。 |

### 🟡 代码质量问题

| # | 文件 | 行号 | 描述 |
|---|------|------|------|
| 6 | `UndoRedoButtons.vue` | 7-8 | `compact ? 'small' : 'small'` 三元表达式两个分支相同，冗余代码。 |
| 7 | `ffmpeg.rs` | 80-88 | `binary_name` 函数 match 两个分支返回相同值，冗余代码。 |
| 8 | `system.rs` | 87 | `let _ = mime;` 变量声明后未使用，死代码。 |
| 9 | `module_registry.rs` | 3 | `#[allow(dead_code)]` + 函数未被调用，死代码。 |
| 10 | `managed.rs` | 265-389 | `download_package` 和 `download_package_to_temp_file` 高度重复，应提取公共逻辑。 |
| 11 | `image_paddler.rs` | 913 | `generate_pdf` 17 个参数，建议使用配置结构体。 |
| 12 | `ImagePreviewGrid.vue` | 全文 | 与 `ReorderableImageGrid.vue` 大量重复代码，应提取 composable。 |

### 🟢 安全相关

| # | 文件 | 行号 | 描述 |
|---|------|------|------|
| 13 | `managed.rs` | 469-502 | ✅ Zip Slip 防护（`enclosed_name()`） |
| 14 | `managed.rs` | 391-412 | ✅ HTTPS-only 下载校验 |
| 15 | `managed.rs` | 447-467 | ✅ SHA256 校验 |
| 16 | `system.rs` | 12-14 | ✅ URL scheme 白名单（仅 HTTPS） |

### 📝 设计建议

1. **提取 `useImageGrid` composable**：统一 ReorderableImageGrid 和 ImagePreviewGrid 的图片加载/分页/缩放逻辑。
2. **统一版本号来源**：AboutView 应从后端 `get_diagnostic_info` 获取版本，而非仅依赖 `import.meta.env`。
3. **统一 URL 打开方式**：AboutView 的 `openGitHub` 应使用 `openExternalUrl` 而非直接调用 `@tauri-apps/plugin-shell` 的 `open`。
4. **工具配置结构体**：`image_paddler.rs` 的 `generate_pdf`/`generate_docx` 应使用配置结构体减少参数数量。
5. **添加工具版本管理**：当前安装后无版本比较和自动更新机制，建议在 `.docsy-tool.json` 中记录版本并在设置页显示。
