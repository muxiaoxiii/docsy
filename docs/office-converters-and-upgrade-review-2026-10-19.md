# Office 原生转换器与依赖升级评审

日期：2026-10-19。基于 Docsy 当前工作区、上游源码、注册表版本，以及实际转换与逐页渲染核对。工作区包含尚未提交的 1.1.1/PdfCraft 改动，本次没有改动生产代码或项目依赖。

## 1. 决策

**office2pdf 可以作为可接入的独立 Rust 转换候选，MiniPdf 也确实已有 Rust 实现；两者目前都不能直接替换这批带表单域、复杂页眉的文件所需的保真转换链。** 有 PDF 输出、退出码为零，与显示正确是不同验收项。

依赖应做完整盘点并分批升级。先处理 PDF.js 等已落入公告影响版本范围的包，再同步 Tauri 组件；Office/PDF 引擎和前端大版本迁移分别验收。不得把升级操作与当前 Gemini 后端修复混成一个无法定位回归的提交。

## 2. 两个项目实际提供什么

| 项目 | 已确认能力 | 接入边界 |
| --- | --- | --- |
| office2pdf v0.9.1 | Rust 库/CLI；DOCX、XLSX、PPTX 经自身 IR/Typst 排版导出 PDF；无需 Office/LO 运行时；API 返回 PDF bytes 与结构化 warnings | 当前直接传 `.doc` 实测拒绝，必须先中转；不是 Word 原生排版。crate MSRV 1.92，Apache-2.0；完整 Docsy 构建与各平台安装包仍需验证。 |
| MiniPdf Rust 0.7.0 | 独立 Rust OOXML 解析/布局/PDF writer，可返回 PDF bytes；没有 .NET/Office 运行时要求 | Rust 子项目自称 experimental，并非 .NET 版能力对等。MSRV 1.82，Apache-2.0；本次页眉/页脚、分节、表单存在实际缺失，不适合当前保真默认路径。 |

office2pdf 运行的是官方 v0.9.1 macOS ARM64 二进制，发布 commit `7893fe96341a11504abba6428cf2614b3821c733`。压缩包 30,051,665 字节，解压执行文件 77,418,224 字节；SHA-256 与官方资产 digest 一致：`d9a5277b28929775e188e57d32d2bc3caa286b141681e81bf621eb1313484c4d`。这只是一个平台的分发体积，不是 Docsy 的最终增量体积。

源码另检查 main `70ebaa0a6d8c06e9fbf68408fe7db4cc1f0c78b2`；它已包含发布版之后的页眉空段落高度等变动，不能把 main 的代码改进算成 v0.9.1 已修复，也不能把 v0.9.1 的实际缺陷不加复测地断言在未来版本不变。MiniPdf 源码固定在 `b34f3a41c86c1127d6504df0629fd62c1449e144`。

已在隔离 manifest 中将 `office2pdf = "=0.9.1"` 与 Docsy 当前依赖一起解析成功，包括当前 `image = "=0.25.5"`，但这只证明解析成功；没有完成全应用编译、链接、安装包验收。官方发布资产未提供 Windows x86，而 Docsy 有 x86 分发目标，需单独确认支持策略。

参考：[office2pdf 发布](https://github.com/developer0hye/office2pdf/releases/tag/v0.9.1)、[库 API](https://github.com/developer0hye/office2pdf/blob/70ebaa0a6d8c06e9fbf68408fe7db4cc1f0c78b2/crates/office2pdf/src/lib.rs)、[MiniPdf Rust 说明](https://github.com/mini-software/MiniPdf/blob/b34f3a41c86c1127d6504df0629fd62c1449e144/minipdf-rs/README.md)。

## 3. 合成样本与真实文件结果

### 3.1 合成样本

两种引擎各完成 4 个输入：双页中英文/粗斜体/页眉页脚、含合并单元格表格、纵横混合分节、现有 doc2x 将公开 `.doc` 测试件中转所得 DOCX。输出全部完成渲染核对。

- office2pdf：合成双页页眉/页脚和表格线可见；混合分节保留两页与各自纸张尺寸。
- MiniPdf：合成双页页眉/页脚消失、表格线缺失；纵向/横向两节合在一张横向页，两个节的正文都在同页。
- 合成 MiniPdf 使用调试构建，不能与 office2pdf 发布构建计时做性能比较。后续真实文件已改用 MiniPdf 发布构建。

### 3.2 四份带域文件与新增页眉副本

仅使用匿名别名：A/B 为最初 DOCX/DOC，C/D 为随后新增页眉的 DOCX/DOC。客户文件、字段内容、文件名和渲染图未存入仓库。4 份原件 SHA-256 前后相同。转换均在本机进行。

| 输入 | 本机 Word 对照 | office2pdf v0.9.1 | MiniPdf Rust 发布构建 |
| --- | --- | --- | --- |
| A 原 DOCX | 5 页 | 6 页 | 8 页 |
| B 原 DOC | 10 页 | doc2x 后 10 页 | doc2x 后 14 页 |
| C 新页眉 DOCX | 5 页 | 6 页 | 8 页 |
| D 新页眉 DOC | 10 页 | doc2x 后 10 页 | doc2x 后 14 页 |

B/D 中间 DOCX 另交给 Word 导出，均为 10 页。所有实际原生输出均返回成功且完成逐页渲染；这不代表通过内容/版式验收。MiniPdf 发布构建四份约 1–4 秒完成；此前调试构建两份 80 秒超时只记录为调试过程，不据此判定发布性能。

#### 阻塞默认接入的缺陷

1. **传统复选框丢失。** A/C 包含 17 个 FORMCHECKBOX（其中 2 个显式 checked）、1 个 FORMTEXT 和 2 个 PAGE；B/D 的中间 DOCX 包含 18 个 FORMCHECKBOX、7 个 FORMTEXT 和 2 个 PAGE。Word 能显示复选框，两种原生输出第一页均未显示对应框形，选中状态也不可见。PDF 文本检索不能替代框形可视检查。
2. **部分表单文字丢失。** B/D 的中间 DOCX 中，5 个规范化后长度至少 4 的非空 FORMTEXT 默认值，Word 输出检出 5/5，两种原生输出只检出 1/5；另外 4 项对应可视空缺。旧格式中转的字段结果存在 `instrText` 编码，不能只遍历 `w:t`，也不能依赖 cached-result-only 统计宣称字段都为空。该核对仅针对这几个明确值，不是全篇文本一致性证明。
3. **office2pdf 首页规则与分页失真。** A/C 在第 2 页出现无正文的附加页，导致 5→6 页；A/B 的原首页专用页眉在输出后续页重复。B 虽仍为 10 页，行距、换页与 Word 对照并不相同。
4. **新增页眉与正文重叠。** office2pdf 的 C/D 页眉内容可见，但表格行高、边框、位置与 Word 不同，多页与正文首行重叠。D 还给出 even-page header/footer 降级为默认页眉的 warning。成功退出并未自动阻止失真输出。
5. **MiniPdf 页眉页脚缺失并重排。** C/D 新增页眉不显示，原页脚 PAGE 页码也未显示；A/C 5→8 页、B/D 10→14 页，A/C 有空白页。D 相对 B 的新增页眉在该引擎结果中没有体现。
6. **doc2x 中转也有边界。** Word 对 B/D 中间 DOCX 仍能显示上述表单值与复选框，但 D 页眉表格外观已与直接导出原 DOC 有差异；原生渲染的进一步问题不能全部归咎于中转，也不能把中转当作全保真承诺。

office2pdf 的页脚 PAGE 在这四份输出中会按其自身输出页数递增；这证明这些 PAGE 数字不是一律使用同一缓存值，不能推及任意 REF/TOC/IF/MERGEFIELD 域。此组实际字段类型主要是传统表单与 PAGE，不能当成交叉引用或目录域的完整验收。

## 4. 接入应怎样实施

当前 `src-tauri/src/pdf/word_convert.rs:227` 的平台 Word/WPS/LibreOffice 链未接入这两库；本次测试是隔离 CLI 和本机 Word 自动化，不是 Docsy 已发布安装包的完整 UI 验收。

office2pdf 可放入独立候选后端，先复用现有临时目录、取消/超时和原子发布（同文件 98/142 行的包装），再接 API `convert_with_options/convert_bytes`，收集 warnings。若先采用 CLI，必须随包携带并在支持的平台检查摘要/签名、隔离子进程、限制资源；后续再评估静态库的包体/内存/构建成本。

第一轮实现应包含：

- 输入预检传统 `ffData`/FORMCHECKBOX/FORMTEXT、首页不同/奇偶页不同、页眉表格、多节、复杂对象；本次暴露的组合优先转系统 Word/WPS/LO，或明确报告不支持，不能仅因候选返回 PDF 就静默认定成功。预检只是已知风险筛选，不能保证普通输入零差异。
- `.doc` 保留直用系统引擎的路线；确需免 Office 时单独走 doc2x 中转候选，并记录两层诊断。不要让一次可恢复格式中转改变用户原件。
- 对现有普通输入提供可见的输出预览；复杂域必须按能力区分“保留显示值”和“重新计算”，不得宣称支持所有 Word 域。
- 引擎不支持/超时/取消/低磁盘空间/已有目标冲突时，保留原文件与既有输出。MiniPdf 的直接 `fs::write` 导出 API 不能绕过 Docsy 原有安全发布封装。
- 复现夹具用合成字段、首页/奇偶页、表格页眉和段落控制重建，不能把本次客户材料提交到 GitHub。
- 干净字体环境、中文/符号、macOS Intel/ARM、Windows x64/x86、Linux、安装包 WebView 实测后，按真实通过的输入范围开放默认路径。

源码依据：MiniPdf `minipdf-rs/crates/minipdf/src/docx.rs:352` 从 body 的单个 sectPr 读取整份布局，对应混合分节实测；检查 header/footer 路径不足以覆盖其 .NET 主项目。office2pdf 的 DOCX 解析、header/footer 与 Typst 页面生成横跨 `parser/docx.rs`、`parser/docx_sections.rs`、`render/typst_gen.rs`，应围绕本次输出问题定位修复，不能只改 Docsy 后端选择开关。

## 5. npm 依赖盘点

当前直接依赖有 **17 项不是最新稳定版**，其中 14 项可在声明范围内更新，5 项存在更高 major；两组有交集（Vite/Vitest）。以下为当前注册表快照，wanted 是 npm 根据现有声明给出的候选，不等于已验证兼容。

| 包 | 当前安装/锁定 | wanted | latest |
| --- | --- | --- | --- |
| `@tauri-apps/api` | 2.11.0 | 2.12.4 | 2.12.4 |
| `@tauri-apps/cli` | 2.11.2 | 2.12.3 | 2.12.3 |
| `@tauri-apps/plugin-dialog` | 2.7.1 | 2.8.3 | 2.8.3 |
| `@tauri-apps/plugin-fs` | 2.5.1 | 2.7.1 | 2.7.1 |
| `@tauri-apps/plugin-shell` | 2.3.5 | 2.5.0 | 2.5.0 |
| `@vitejs/plugin-vue` | 6.0.8 | 6.0.10 | 6.0.10 |
| `element-plus` | 2.14.1 | 2.14.7 | 2.14.7 |
| `eslint` | 10.7.0 | 10.13.0 | 10.13.0 |
| `eslint-plugin-vue` | 10.9.2 | 10.11.1 | 10.11.1 |
| `mermaid` | 11.17.2 | 11.17.2 | 12.2.0 |
| `pdfjs-dist` | 6.1.200 | 6.4.299 | 6.4.299 |
| `pinia` | 2.3.1 | 2.3.1 | 4.1.0 |
| `prettier` | 3.9.5 | 3.9.10 | 3.9.10 |
| `vite` | 7.3.6 | 7.3.7 | 8.3.4 |
| `vitest` | 4.1.8 | 4.1.12 | 5.0.5 |
| `vue` | 3.5.35 | 3.5.44 | 3.5.44 |
| `vue-router` | 4.6.4 | 4.6.4 | 5.4.0 |

`npm audit` 记录 13 个受影响包条目：7 high、3 moderate、3 low、0 critical。这是包条目计数，不是 13 个独立漏洞，也不证明当前桌面应用可被利用。

优先级：

- **PDF.js 6.1.200 → 至少 6.2.108，建议验证 6.4.299。** 当前版本落入 GHSA-hq66-cqwq-w95j 公告范围；公告触发条件涉及 scripting/CSP，Docsy 当前 Canvas 显示路径不能据此直接认定可利用，但应升级并回归 worker/离线字体/CMap/阅读器资源。
- **Vitest 4.1.8 → 4.1.12**，覆盖公告修复边界 4.1.11；先留在当前 major，Vitest 5 随另一批迁移。
- **Vue 3.5.35 → 3.5.44**，连带核对 server-renderer 等传递项。高危传递项的实际运行可达性需结合本应用，不把开发/服务端依赖问题自动等同于桌面风险。
- 重新锁定 postcss/source-map-js/brace-expansion/nanoid 等传递项；dompurify/katex/mermaid 的修复需要结合 Mermaid 12 迁移成本单列，避免 `audit fix --force` 一次破坏多条功能。

来源：[PDF.js 公告](https://github.com/advisories/GHSA-hq66-cqwq-w95j)、[Vitest 公告](https://github.com/advisories/GHSA-82fw-gwwq-j7x9)、配套 `npm-audit.json`。

## 6. Rust 直接依赖盘点

37 个 registry 直接/构建依赖中，29 个有较新稳定版；另有 3 个 path 依赖。按 Cargo.lock 根包依赖实际解析版本统计，没有把并存的传递依赖版本当作根包版本。crates.io API 本机返回 403，改从官方 sparse index 完成，查询无遗留失败。

| 包 | 当前解析 | 最新稳定/来源 | 备注 |
| --- | --- | --- | --- |
| `tauri-build` | 2.6.3 | 2.7.3 |  |
| `cc` | 1.3.0 | 1.8.0 |  |
| `tauri` | 2.11.5 | 2.12.4 |  |
| `tauri-plugin-dialog` | 2.7.2 | 2.8.3 |  |
| `tauri-plugin-fs` | 2.5.1 | 2.7.1 |  |
| `tauri-plugin-shell` | 2.3.5 | 2.5.0 |  |
| `serde` | 1.0.229 | 1.0.229 |  |
| `serde_json` | 1.0.151 | 1.0.154 |  |
| `anyhow` | 1.0.104 | 1.0.104 |  |
| `thiserror` | 2.0.19 | 2.0.21 |  |
| `zip` | 0.6.6 | 9.0.5 |  |
| `sevenz-rust` | 0.6.1 | 0.6.1 |  |
| `regex` | 1.13.1 | 1.13.1 |  |
| `include_dir` | 0.7.4 | 0.7.4 |  |
| `unicode-normalization` | 0.1.25 | 0.1.26 |  |
| `dirs` | 5.0.1 | 7.0.0 |  |
| `reqwest` | 0.12.28 | 0.13.5 |  |
| `sha2` | 0.10.9 | 0.11.1 |  |
| `chrono` | 0.4.45 | 0.4.45 |  |
| `image` | 0.25.5 | 0.25.10 | 精确锁定 =0.25.5，单独评估 |
| `printpdf` | 0.9.1 | 0.12.8 |  |
| `docx-rs` | 0.4.21 | 0.4.22 |  |
| `pulldown-cmark` | 0.13.4 | 0.13.4 |  |
| `lopdf` | 0.39.0 | 0.45.0 |  |
| `ttf-parser` | 0.25.1 | 0.25.1 |  |
| `base64` | 0.22.1 | 0.23.1 |  |
| `encoding_rs` | 0.8.35 | 0.8.42 |  |
| `open` | 5.4.0 | 5.4.4 |  |
| `log` | 0.4.33 | 0.4.34 |  |
| `allsorts` | 0.16.1 | 0.17.0 |  |
| `rusqlite` | 0.32.1 | 0.40.2 |  |
| `tokio-util` | 0.7.18 | 0.7.20 |  |
| `office_oxide` | 0.1.12 | 0.1.15 |  |
| `docsy-anydoc` | 0.2.4 | path 固定副本 | path，不参与 registry 自动升级 |
| `quick-xml` | 0.37.5 | 0.42.0 |  |
| `calamine` | 0.26.1 | 0.37.0 |  |
| `rust_xlsxwriter` | 0.80.0 | 0.100.0 |  |
| `htmd` | 0.5.5 | 0.5.6 |  |
| `pdfcraft-cos` | 0.9.0 | path 固定副本 | path，不参与 registry 自动升级 |
| `pdfcraft-organize` | 0.9.0 | path 固定副本 | path，不参与 registry 自动升级 |

`0.x` 的 minor 上调也可能不兼容，尤其 lopdf、printpdf、quick-xml、calamine、rust_xlsxwriter、office_oxide、allsorts。zip 0.6→9、reqwest 0.12→0.13、rusqlite 0.32→0.40、dirs 5→7、sha2 0.10→0.11、base64 0.22→0.23 必须检查 API/特性/许可证和回归范围，不能以同名库作机械替换。

本机 rustc 1.99.0 满足本次候选的已声明 MSRV；仓库 CI 和各平台实际工具链必须同步核对。image 采用精确版本是现有约束；不能为追新直接移除 pin。此前独立 PDF Oxide 探针曾按上游锁定 office_oxide 0.1.9，而 Docsy 根包声明另有范围，独立探针通过不能证明整应用的任意新版本组合可编译。

## 7. vendored 源码、外置工具和版本文件

- PdfCraft 当前是 0.9.0 vendored 源码。固定副本与新上游 `43550c448db3ba91753f638e3843c0da036bbd60` 比较，当前使用模块相关变化有 parser 对 `stream` 后空格/制表符再换行的容忍及测试，可考虑定向导入；版本号没有变化，不能只看 Cargo version。它不自动修复之前发现的压平/清理语义问题。
- docsy-anydoc 是本地维护 fork/path 依赖，需保留本地补丁并逐项对比上游，不能靠 `cargo update` 自动完成。
- doc2x 的平台包 npm latest 仍为 0.1.0，当前 pin 同版本；本次 `.doc` 实测走的正是现有 sidecar。无需为了“全面升级”改成不存在的新版本。
- managed fallback 配置中的 qpdf 非 Windows 12.2.0、Windows64 12.3.2；上游最新 12.4.2。Poppler 非 Windows 25.12.0、Windows64 26.02.0-0；所查 Windows 分发最新 26.09.0-0。分发仓库、架构与平台不能混用，摘要、下载地址、许可证通知和回退路径需一起验收。
- 现配置 `toolchain-v1/tools-manifest.json` 下载地址本次 HTTP 404。单纯升级 Cargo/npm 无法修复扩展工具清单发布问题，需要确认资产发布与下载契约。
- 当前 package/Cargo/Tauri 版本为 1.1.1，而 package-lock 根包仍为 1.0.10，版本一致性检查失败。这是本轮工作区状态，应在发版前同步并重跑检查，不代表已提交的 1.0.10 发布自身出错。

## 8. 给实现者的升级顺序与验收

| 批次 | 修改范围 | 最低验收 |
| --- | --- | --- |
| 1 | PDF.js/Vue/Vitest 当前兼容范围和相关传递安全修复，版本文件同步 | 锁文件可复现安装；安全报告差异；Vitest/ESLint/build；实际 PDF 多页/中文/缩放/字体/离线 worker；版本一致性。 |
| 2 | Tauri Rust/JS API、CLI、build 和 dialog/fs/shell 插件协调升级 | Rust 相关测试；文件权限/对话框/子进程/取消；三平台安装包冒烟与现有工具链检测。 |
| 3 | office2pdf 候选适配；PDF/Office Rust 库分别迁移；vendored 定向补丁 | 字段/页眉/分节/中文字体合成契约、真实私有文件人工对照；安全输出/取消/内存；现有 PDF 合并/提取/压平/书签/压缩/文本检测回归。 |
| 4 | Vite 8、Vitest 5、Pinia 4、Vue Router 5、Mermaid 12 | 按迁移指南分别变更与回归，确认构建产物、路由/状态持久化、图表、安全修复均覆盖。 |

每批记录精确锁文件、升级前后差异、已运行检查和未覆盖平台。需要保真的复杂 Word 文件，在上述缺陷修复并复测前仍应走系统排版器。不要因“原生优先”目标而把本次已见的内容缺失当成可接受的静默降级。

## 9. 可复核证据

目录：[research/office-converter-upgrade-2026-10-19](research/office-converter-upgrade-2026-10-19/)。包含合成探针/结果、匿名真实文件数量与字段测量、npm 更新与审计、Rust sparse-index 盘点、外置工具版本和发布版本检查。客户原文、PDF、图片不在该目录。

本次修改只有研究文档、可分享探针/元数据和相应文档忽略例外；没有安装依赖升级、改业务路由、提交、推送或替换现有 Gemini 代码。

PDF 结构辅助检查：14 份不同输入/引擎组合的输出中，8 份原生引擎输出通过 qpdf 检查（退出码 0）；6 份 Word 对照返回退出码 3（可恢复的结构警告），全部页面已成功渲染。结构检查不能代替域、页眉和版式核对。
