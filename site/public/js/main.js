/* Docsy 下载站交互：数据渲染、平台识别、校验值复制、入场动效 */
(function () {
  'use strict';

  /* 无 JS 或数据加载失败时的回退数据（与发布产物保持一致） */
  var FALLBACK = {
    "latest": "1.1.4",
    "githubUrl": "https://github.com/muxiaoxiii/docsy/releases",
    "releases": [
      {
        "version": "1.1.4",
        "tag": "v1.1.4",
        "published_at": "2026-11-03T03:56:54Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.4_aarch64.dmg",
            "size": 57722039,
            "sha256": "60b8f1712c3e7573b5b9f7d1d4b63c3f89929ec4553661f4d657f1c3fe362d71",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.4_x64-setup.exe",
            "size": 53551533,
            "sha256": "509342cbc37c7f3827ea1961457450485a213e52b6d85ce27b61a76189bab6e8",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.4_x86-setup.exe",
            "size": 52365157,
            "sha256": "f83cbaaef39984576702d1275d657b7632d92de24a56509314caa36e706a41d9",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.4/Docsy_1.1.4_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.3",
        "tag": "v1.1.3",
        "published_at": "2026-11-02T03:47:50Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.3_aarch64.dmg",
            "size": 55937224,
            "sha256": "0ae6a11ae7dddea52fc7cb714c9934c88f75a62a38954669e50e287e327691cb",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.3_x64-setup.exe",
            "size": 52337043,
            "sha256": "5b1a58b82cc3f57b0283811ff5f826fd08ab1f25b9bd6a1f131a282c09ea9925",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.3_x86-setup.exe",
            "size": 51172141,
            "sha256": "585836e6672430ecd67318a455e5119c13aa7625ca9984abacfdded02ce9e410",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.3/Docsy_1.1.3_x86-setup.exe"
          }
        ]
      },
      {
        "version": "1.1.2",
        "tag": "v1.1.2",
        "published_at": "2026-10-23T01:22:43Z",
        "assets": [
          {
            "platform": "macos",
            "name": "Docsy_1.1.2_aarch64.dmg",
            "size": 55914400,
            "sha256": "24ecc3a02d986fc85faf67ded7a5eb307eb9d4744fdf1d9e47174287d7e27341",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_aarch64.dmg"
          },
          {
            "platform": "windows",
            "name": "Docsy_1.1.2_x64-setup.exe",
            "size": 52293013,
            "sha256": "1ce308b1c56316000ffe99ca741d9d8e8c44ddf9acfff26dda9dbde245e73c98",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x64-setup.exe"
          },
          {
            "platform": "windows-x86",
            "name": "Docsy_1.1.2_x86-setup.exe",
            "size": 51138434,
            "sha256": "c4637c3cae58de03d95dcc87256f8d105af7fe32183badc4de602f0793c4a390",
            "url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe",
            "github_url": "https://github.com/muxiaoxiii/docsy/releases/download/v1.1.2/Docsy_1.1.2_x86-setup.exe"
          }
        ]
      }
    ]
  };

  var FALLBACK_CHANGELOG = [
    {
      "version": "1.1.4",
      "date": "2026-11-02",
      "sections": [
        {
          "title": "新增功能与体验优化",
          "items": [
            "新增「长图处理」功能：支持网页长截图、手机长截屏图片（PNG/JPG/WebP/BMP）以及单页或多页 PDF 逐页智能切片，合并输出标准 A4 打印文件；多页预览标明来源页码。",
            "智能切刀防截断：通过视觉空白间隙与色差方差检测，自动寻找自然段落留白处下刀，避免文字行被截断或腰斩。",
            "多维度排版适配：支持 A4 纵向/横向切换、四周边距调节、内容宽度模式（满版 / 手机适中 55% / 自定义比例）与水平对齐设置。",
            "页眉与页脚定制：支持添加来源网址或标题、打印日期（可自定义是否显示“打印日期：”前缀）及标准多样式页脚页码。",
            "实时 A4 打印预览：提供 A4 纸张模拟翻页预览，所见即所得；支持拖拽批量处理，并融入 Doclet 动效与完成庆祝反馈。",
            "依赖与兼容性：升级 PdfCraft v0.18.0、Mermaid 12 及 Tauri/Rust 安全依赖，适配 quick-xml 新版实体引用解析。"
          ]
        }
      ]
    },
    {
      "version": "1.1.3",
      "date": "2026-11-02",
      "sections": [
        {
          "title": "改进与修复",
          "items": [
            "优化证据拆分：支持批量分段、精确跳转首尾页和预览缓存，改进页眉与页码相邻时的识别。",
            "统一证据合并的页眉、页脚和页码配置、列表及预览；支持逐文件编辑、文件名选字和全局修改冲突提示与撤销。",
            "改进 PDF 页面裁切、旋转和页面尺寸处理，修复部分文件预览及原页眉页脚定位。",
            "改进 PDF 合并输出目录和文件名设置；同名输出自动加序号，不覆盖原文件。",
            "调整证据设置及文件列表布局，改善按钮对齐和窄窗口下的显示。"
          ]
        }
      ]
    },
    {
      "version": "1.1.2",
      "date": "2026-10-23",
      "sections": [
        {
          "title": "体验与稳定性",
          "items": [
            "2026-10-26 补充修复：部分 PDF 在 A4 规范化后正文消失、裁切内容重新露出；预览遵守裁切范围，证据编号起始与导出同步。",
            "修复 Windows 下 Doclet 鼠标跟随偶发黑底，保留现有转头与动作效果。",
            "Doclet 使用新旧素材补齐八方向，停留后快速转向与回正，修正左上衔接闪帧；点击随机播放动作。",
            "统一按钮、输入框、数字加减、下拉框与提示的尺寸、圆角和对齐；加入按压回弹、侧栏与标签页滑动高亮、弹窗过渡，配置说明收进对应气泡。",
            "重整证据合并设置，页码格式改为下拉并扩充样式，格式预览独立展示；浏览器预览明确提示桌面功能限制。",
            "修正图片排版网格示意、N 字顺序及末页重排，统一手机照片方向标记在预览和导出中的表现。",
            "PDF／证据合并支持 Word 和静态图片；图片默认一图一页，可拖拽合页、排序、旋转、缩放及手动裁剪，调整行列保留手动缩放，保留原始像素导出；修复 Mac Word 自动转换，失败可补充手动 PDF 继续。",
            "证据扫描包含母文件夹文件，支持按文件夹分别输出或全部合并；文件夹内、文件夹间双面补白独立设置，分隔页带识别标记，空白页删除仍由用户勾选并保守检查。",
            "修复启动时的分包依赖问题，公式和图表按需加载，缩减跟随图集与安装资源占用。",
            "修复 Windows 测试启动与工具路径识别，恢复 x64、x86 完整后端回归检查；清理废弃代码与编译警告。"
          ]
        }
      ]
    },
    {
      "version": "1.1.1",
      "date": "2026-10-20",
      "sections": [
        {
          "title": "体验优化",
          "items": [
            "修正 Doclet 右侧转向上方时经过正面的问题，避免鼠标跟随停在闭眼帧。",
            "Doclet 接入透明视频动作，支持缓慢鼠标跟随与点击随机动作，并在不可见时暂停动态。",
            "收紧各模块配置布局，统一按钮与提示圆角，优化工具栏分组和小窗口显示。",
            "MD 转换的“公式与图表”入口移至标题旁，按输出格式说明可编辑公式与图片回退。",
            "校正功能提示：PDF 仅支持已有文本层提取为 Markdown，MD 页面暂不导出 PDF；清理超出实际能力的描述。"
          ]
        },
        {
          "title": "新增",
          "items": [
            "引入纯 Rust 页面组织后端（基于 PdfCraft 固定提交精简 vendored crates：`cos`, `crypt`, `filters`, `content`, `organize`），支持无外部依赖的页面提取、合并与注释/签章打印压平。",
            "引入后端统一门面与分派器（`dispatch_merge`、`dispatch_extract`），**默认优先使用纯 Rust `PdfCraftBackend`（Native-First）**，遇到需重算外观（`NeedAppearances=true`）、防复制备份合并或复杂加密时自动受控回退至 `QpdfBackend`（亦可通过 `DOCSY_PDF_BACKEND=qpdf` 强制指定外置 `qpdf`）。",
            "**PDF 原生优先（Native-First）基础能力落地**：",
            "显式定义合并语义策略 `MergePolicy`（`Print` 打印压平模式 vs `Structural` 结构保留模式）与细粒度 `OutputPolicy`。",
            "阶段 3 标记隔离系统落地：支持 0°/90°/180°/270° 四向旋转与非零 CropBox 视觉矩阵映射（`view_matrix_for`），支持独立标记流的无损更新与按批次/类别物理清理。",
            "阶段 4 结构化提取 PoC：独立提取页面文本与边界框，生成保留 1-based 页码的 Markdown，并对空白与纯图像扫描页给出明确提示。",
            "**Markdown 复制为多格式可粘贴内容**：Markdown 转换页新增“复制”功能，支持从粘贴区（富文本/纯文本下拉）或文件队列一键复制，同时向剪贴板写入 HTML 与规范化纯文本。富文本与纯文本可分别复制；粘贴格式及样式保留取决于目标软件。",
            "**GitHub 加速镜像备选扩充**：设置页公共镜像候选由 4 个扩充至 11 个优质社区节点，支持一键测速并按可用性与延迟排序；下载失败时由 Docsy 自动回退至官方源。",
            "**多模块响应式可调节分栏与现代化空态**：",
            "**首屏体积与性能优化**：通过 Vite `manualChunks` 将 Element Plus、Mermaid、MathJax、Cytoscape 等第三方库独立分包，首页入口 `index.js` 体积从 1.07 MB 降至 71.39 kB（降幅 93.3%），Mermaid 与 MathJax 完全按需动态引入。",
            "**保持 Word (`.docx` / `.doc`) 转 PDF 链路不变**：经实测对照，现有纯 Rust 独立排版引擎（`office2pdf`、`MiniPdf`）在复杂法律文书上仍存在新增页眉重叠、复选框与表单文字丢失及分页偏移问题，因此 `word_convert.rs` 保持系统 Word / WPS / LibreOffice 保真转换管线不变。"
          ]
        },
        {
          "title": "修复",
          "items": [
            "**文书模板与工作区 SQLite 锁死修复 (P0)**：修复点击「文书模板」时弹出的 `database is locked: Error code 5: The database file is locked`。重构数据库初始化机制，使用进程级互斥保证建表、索引与迁移单例一次性完成；常规连接不再重复执行 DDL 与 UPDATE；写事务统一升级为 `TransactionBehavior::Immediate` 独占模式杜绝死锁；前端调整为按需懒加载 Tab 数据。",
            "**设置页单层滚动条与全路由溢出优化**：修复 `ToolWorkspaceShell` flex 属性导致的卡片内部多滚动条回归，卡片恢复自然高度并由外层统一平滑滚动；完成 1440 与 1100 窗口下全路由溢出审计。",
            "**qpdf 回退元数据与防复制恢复链保留 (P0-2 & P2-3)**：修复 qpdf 回退丢弃 trailer `/Info` 和 `DocsyAntiCopyBackup` 的缺陷；通过深递归拓扑复制工具完整回写元数据，并改由同目录临时文件与 `replace_file` 实现排他性原子发布，杜绝并发覆盖并确保恢复链 100% 完整。",
            "**Print 模式附件剥离策略执行 (P1-1)**：修复 Print 模式合并可能外泄内嵌附件的隐患；合并时严格执行附件剥离策略，彻底清除 Catalog `/Names` 中的 `/EmbeddedFiles` 节点。",
            "**预检 Warning/Fallback 全量透传与电子签名深入扫描 (P1-2 & P2-1)**：在 `dispatch_merge` / `dispatch_extract` 中将预检产生的 `PreflightSummary.warnings` 完整合并进最终结果；签名检测由单一 `SigFlags` 扩展至全对象字典（`/Type /Sig`、`/FT /Sig`、Widget `/V`）深度扫描，qpdf 后端同步补齐签名/书签/备份感知。",
            "**证据批量处理故障隔离与取消支持 (P1-3)**：`build_group_pdfs` 分组处理由中断整批的 `?` 改为单组隔离 `match`，收集 `failed_groups` 并保留成功产物；`apply_overlay_batch` 保留成功输出；`build_evidence_group_pdfs`、`merge_evidence_pdfs`、`split_merged_evidence_pdf` 全面接入 `run_managed`，长任务可即时响应前端停止请求。",
            "**PDF.js 依赖安全升级与 XFA 攻击面收敛 (P1-4)**：升级 `pdfjs-dist` 至 `^6.4.299`；`PdfJsPreview.vue` 默认关闭 `enableXfa: false`，彻底消除 GHSA-hq66-cqwq-w95j 潜在安全风险。",
            "**开源合规与第三方声明补齐 (P0-1 & P1-5)**：`vendor/pdfcraft` 源码及 LICENSE/NOTICE 完整纳入 Git 跟踪并补齐 `VENDOR.md` 补丁记录；`THIRD-PARTY-NOTICES.md` 补齐 PdfCraft 双重许可证（MIT/Apache-2.0）声明。",
            "**原生书签写入重编号门禁 (P2-2)**：`bookmarks.rs` 增加 `validate_recovery_backup` 门禁，遇到依赖瞬态对象编号的 v1 防复制备份时主动拦截报错，杜绝重编号破坏文字恢复映射。",
            "**Element Plus 组件规范与 UI 细节**：全面将 `el-radio-button` / `el-radio` 的 `label` 迁移至 `value` 规范，消除控制台废弃警告；修复未注册图标 `<i-ep-arrow-right>` 为 `ArrowRight`；统一字体权重、路径断行与文本对比度。",
            "**防删除与防覆盖发布安全**：合并与提取统一使用排他性原子发布（`copy_unique_with_stem`），杜绝覆盖源文件、已有用户目标或并发结果；失败与回退前绝不触碰已有目标文件，中间产物全程由目录和临时路径守卫管理释放。",
            "**防复制恢复链保留**：修复页面提取忽略防复制备份的缺陷；提取产物完整保留 Trailer `Info`，保留 v2 字体级 `DocsyOriginalCMap` 并对 v1 备份自动重映射字体编号，确保提取后的 PDF 依然 100% 可逆恢复。",
            "**结构保留语义完整性**：Structural 合并完整保留首份输入的 Trailer `Info`（含自定义 `CaseRef` 等）与 Catalog `/Metadata` XMP 流；禁止将 Structural 请求降级为破坏 AcroForm 表单的打印合并。",
            "**无 `Do` 算子页面未使用 XObject 清理修复**：修复 `pdfcraft-organize/src/prune.rs` 在页面不含任何 `Do` 算子时未注册 `XObject` 清理键导致未使用 Form/Image XObject 残留的问题，同时保护 `/Pattern`、`/ExtGState`、`/Shading` 与 `/Annots` 间接资源。",
            "**`UserUnit` 预览与后端尺寸一致性修复**：在 `page_info.rs` 全路径（`pdfcraft-cos`、`lopdf`、`qpdf` JSON）及 `pdfcraft-organize` 页面树继承属性中统一接入 `/UserUnit`（PDF 1.6+，默认 `1.0`）缩放，使后端 `PageSize` 与前端 PDF.js `getViewport({ scale: 1 })` 几何尺寸严格一致。",
            "**流关键字行尾空白容错修复**：修复 `pdfcraft-cos` 流解析器对 `stream` 关键字后紧跟水平空格/制表符再换行（`stream \\n`）样本报语法错误的问题。",
            "**包装流所有权与正文保护加固**：`marks::is_wrapper_stream` 要求同时满足 `is_docsy_stream == true`、`/DocsyWrapper` 为 `/Push` 或 `/Pop` 且解码内容精确匹配 `q\\n` / `\\nQ\\n`，防止误删带有同名键的普通正文流。",
            "**取消上下文与前端通知**：`merge_pdfs` 与 `extract_pdf_pages` 入口全面接入 `OperationManager` 与线程取消 token，可即时响应前端任务取消；通过 `pdf-backend-notice` 事件与 `PdfOutputResult` 扩展字段传回 warnings 与 fallback_reason。",
            "**页面标记所有权与正文保护 (P1)**：修复 `/Docsy false` 或缺少受管标记字段的正文流被误判为标记物理删除的问题；仅当 `/Docsy` 为真值或同时具备 `DocsyId` 与 `DocsyVersion` 时才认领为 Docsy 标记流。",
            "**页面几何属性 Parent 树继承 (P1)**：修复 `page_geometry` 仅读取当前页字典、忽略父级 `Pages` 树节点属性的缺陷；沿 `Parent` 链严格向上解析 `Rotate`、`CropBox`、`MediaBox` 与 `UserUnit` 继承属性，保证旋转与非零边界框计算准确无误。",
            "**多用途共享大图防过度降采样 (P1)**：修复 Form XObject 深度拓宽后，共享大图因嵌套的小尺寸用途分析导致被过度降采样的回归缺陷；以多用途中最小 DPI（即视觉最大用途的最大像素需求）为下限保护，禁止大用途因嵌套缩略图而被毁坏画质。",
            "**提取 PoC 真实坐标与字号计算 (P2)**：修复文本提取 span 固定假常量（`[0, 0, width, 12.0]` / `12.0`）的问题；基于内容流算子解码真实解析 `Tf`（字号）、`Tm`/`Td`/`TD`（文本位置与变换矩阵）并结合 `ctm` 计算实际像素边界框。",
            "**提取 PoC 异常真实上报 (P2)**：修复内容流语法错误或损坏时将文本解析失败静默伪装为空白扫描页的缺陷，如实将底层解析错误向外抛出为 `BackendError`。",
            "**标记标识符 PDF 字符转义 (P2)**：修复 `DocsyId` 字符串字面量中反斜杠 `\\`、括号等特殊字符未转义导致破坏 PDF 标记内容流语法的缺陷。",
            "**标记图形状态严格隔离 (P2)**：在页面已有基础正文流首尾分别插入 `DocsyWrapper` 的 `q` / `Q` 状态包装流，确保基础正文内未闭合的 `cm` 变换矩阵 100% 不污染后续标记渲染，且在清空标记时自动回收包装流。",
            "**页面树环形死循环防护 (P2)**：页面收集统一接入带 `visited: HashSet<ObjRef>` 的 `collect_leaf_pages`，彻底消除 `Kids` 循环引用导致的栈溢出与死循环。",
            "**Form XObject 循环引用阻断 (P2)**：Form XObject 递归遍历引入 `ancestor_forms` 调用栈检测，杜绝自引用与互相嵌套引起的指数级遍历爆炸。",
            "**未定义字体资源校验 (P2)**：`apply_mark_to_page` 在添加标记前严格检查标记算子使用的字体是否已在页面或父级 `/Resources /Font` 中定义，杜绝静默写入未定义字体。",
            "**依赖与编译修复**：修复 `pdfcraft-crypt` 单元测试缺失 `proptest` 特性导致的离线编译报错。"
          ]
        },
        {
          "title": "验证数据",
          "items": [
            "mimo 阶段 3–4 验收契约套件：**22 项通过、0 项失败、2 项忽略**（覆盖 10 项缺陷全部转绿）。",
            "Reviewer 阶段 0–2 复现套件：**10 项全部通过 (10/10)**。",
            "Rust 全量单元测试：**448 项通过、0 项失败、15 项忽略**（含 `PATH=/nonexistent` 无外部工具环境下的原生优先测试与并发 DB 锁死回归测试）。",
            "前端测试：**40 个文件，334 项测试全部通过 (334/334)**；ESLint **0 错误 0 警告**；构建测试 **通过（index 71 kB / gzip 27 kB）**；版本一致性检查 `check-release-version.mjs` **1.1.1 通过**。",
            "代码差异检查：`git diff --check` **通过**。"
          ]
        }
      ]
    },
    {
      "version": "1.0.10",
      "date": "2026-10-19",
      "sections": [
        {
          "title": "新增",
          "items": [
            "PDF 合并支持 PDF 与 Word（.docx/.doc/.docm）混排、排序与拖入；单份 Word 可直接转为 PDF。",
            "分项证据合并可导入 Word，转换后使用 PDF 预览和原有处理流程。",
            "共用 Word→PDF 转换模块，优先调用本机 Microsoft Word、Windows WPS，再尝试 LibreOffice；旧版 .doc 直接转换失败后可通过 doc2x 转为临时 .docx 再重试。"
          ]
        },
        {
          "title": "修复",
          "items": [
            "默认转换输出通过排他创建避开已有或并发生成的同名文件，保留原有 PDF。",
            "指定目标的保存采用同目录临时文件与安全替换，复制或替换失败时清理临时文件。",
            "转换使用独立临时目录，结束或失败后释放中间文件。",
            "证据导入期间锁定重复导入和执行入口，避免异步转换覆盖后续导入结果。",
            "修复 Windows 首次写入不存在目标文件时的替换处理。",
            "保存目标若为目录则拒绝替换，避免误将目录当作文件。"
          ]
        },
        {
          "title": "使用说明",
          "items": [
            "Word 转 PDF 依赖本机可用办公引擎；WPS 自动转换当前仅在 Windows 支持。不承诺不同引擎的分页完全一致。",
            "PDF 结构处理继续使用现有 qpdf 路径，本版不包含 PdfCraft 新后端。"
          ]
        },
        {
          "title": "本机验证",
          "items": [
            "前端 330 项通过；Rust 单元测试 410 项通过、15 项忽略，doc2x 探针另 1 项通过；规范、生产构建和版本一致性检查通过。",
            "Microsoft Word 实际导出样例 PDF 并完成渲染检查；输出有 qpdf 可兼容处理的交叉引用警告。Windows Office 自动转换及安装验收仍需实机核对。"
          ]
        }
      ]
    }
  ];

  function formatSize(bytes) {
    if (!bytes) return '';
    var mb = bytes / 1024 / 1024;
    return (mb >= 100 ? mb.toFixed(0) : mb.toFixed(1)) + ' MB';
  }

  function formatDate(iso) {
    if (!iso) return '';
    var date = new Date(iso);
    if (isNaN(date.getTime())) return iso;
    return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
  }

  /* 渲染 **加粗** 标记为 <strong> */
  function renderBold(text) {
    var parts = String(text).split(/\*\*(.+?)\*\*/g);
    var html = '';
    for (var i = 0; i < parts.length; i++) {
      if (i % 2 === 1) {
        html += '<strong>' + escapeHtml(parts[i]) + '</strong>';
      } else {
        html += escapeHtml(parts[i]);
      }
    }
    return html;
  }

  function escapeHtml(text) {
    return String(text)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function platform() {
    var ua = navigator.userAgent || '';
    if (/Mac/i.test(ua)) return 'macos';
    if (/Windows|Win/i.test(ua)) return 'windows';
    return '';
  }

  function assetFor(release, platformName) {
    var list = (release && release.assets) || [];
    for (var i = 0; i < list.length; i++) {
      if (list[i].platform === platformName) return list[i];
    }
    return null;
  }

  /* ---------- 下载区渲染 ---------- */

  function renderDownloads(releases) {
    if (!releases || !releases.length) return;
    var latest = releases[0];
    var current = platform();

    var mac = assetFor(latest, 'macos');
    var win = assetFor(latest, 'windows');

    var versionEl = document.getElementById('dl-version-static');
    if (versionEl) versionEl.textContent = 'v' + latest.version;

    var macDownload = (mac && mac.github_url) || (mac && mac.url) || FALLBACK.githubUrl;
    var winDownload = (win && win.github_url) || (win && win.url) || FALLBACK.githubUrl;

    if (mac) {
      setText('dl-mac-name', mac.name);
      setText('dl-mac-size', formatSize(mac.size));
      setText('dl-mac-sha', mac.sha256 || '见 GitHub Releases 发布页');
      setAttr('dl-mac-link', 'href', macDownload || '#');
    }
    if (win) {
      setText('dl-win-name', win.name);
      setText('dl-win-size', formatSize(win.size));
      setText('dl-win-sha', win.sha256 || '见 GitHub Releases 发布页');
      setAttr('dl-win-link', 'href', winDownload || '#');
    }

    /* Hero CTA 也指向最新版 */
    if (mac) setAttr('cta-macos', 'href', macDownload || '#');
    if (win) setAttr('cta-windows', 'href', winDownload || '#');
    var x86 = assetFor(latest, 'windows-x86');
    if (x86) setAttr('dl-win-x86', 'href', x86.github_url || x86.url);

    /* 标记当前系统 */
    if (current) {
      var card = document.querySelector('[data-platform-card="' + current + '"]');
      if (card) card.classList.add('is-current');
      var tag = document.querySelector('[data-os-tag="' + current + '"]');
      if (tag) tag.hidden = false;
    }

    setAttr('dl-release-page', 'href', FALLBACK.githubUrl + '/tag/' + (latest.tag || 'v' + latest.version));
  }


  /* ---------- 更新日志渲染 ---------- */

  var changelogEntries = [];
  var changelogExpanded = false;

  function renderChangelog(entries) {
    if (entries && entries.length) changelogEntries = entries;
    var listEl = document.getElementById('changelog-list');
    if (!listEl) return;
    listEl.innerHTML = '';
    var list = changelogExpanded ? changelogEntries : changelogEntries.slice(0, 5);
    if (!list.length) {
      listEl.innerHTML = '<li class="timeline-empty">暂无更新记录。</li>';
      return;
    }
    list.forEach(function (entry) {
      var li = document.createElement('li');
      var head = '<div class="tl-head"><span class="tl-version">v' + escapeHtml(entry.version) + '</span>' +
        '<span class="tl-date">' + escapeHtml(entry.date || '') + '</span></div>';
      var sections = '';
      (entry.sections || []).forEach(function (section) {
        if (!section.items || !section.items.length) return;
        var items = section.items.map(function (item) {
          return '<li>' + renderBold(item) + '</li>';
        }).join('');
        sections +=
          '<div class="tl-section"><div class="tl-section-title">' +
          escapeHtml(section.title) + '</div><ul>' + items + '</ul></div>';
      });
      li.innerHTML = head + '<div class="tl-sections">' + sections + '</div>';
      listEl.appendChild(li);
    });

    var toggleBtn = document.getElementById('changelog-toggle-btn');
    if (toggleBtn) {
      if (changelogEntries.length > 5) {
        toggleBtn.hidden = false;
        toggleBtn.textContent = changelogExpanded ? '收起历史版本' : '展开更多历史版本 (' + changelogEntries.length + ')';
      } else {
        toggleBtn.hidden = true;
      }
    }
  }

  function initChangelogToggle() {
    var toggleBtn = document.getElementById('changelog-toggle-btn');
    if (!toggleBtn) return;
    toggleBtn.addEventListener('click', function () {
      changelogExpanded = !changelogExpanded;
      renderChangelog();
    });
  }

  function setText(id, value) {
    var el = document.getElementById(id);
    if (el) el.textContent = value;
  }

  function setAttr(id, attr, value) {
    var el = document.getElementById(id);
    if (el && value) el.setAttribute(attr, value);
  }

  /* ---------- 复制校验值 ---------- */

  function initCopyButtons() {
    document.querySelectorAll('.copy-btn').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var selector = btn.getAttribute('data-copy');
        var target = selector ? document.querySelector(selector) : null;
        var text = target ? target.textContent : '';
        if (!text) return;
        var done = function () {
          btn.classList.add('is-copied');
          var original = btn.getAttribute('aria-label') || '';
          btn.setAttribute('aria-label', '已复制');
          setTimeout(function () {
            btn.classList.remove('is-copied');
            btn.setAttribute('aria-label', original);
          }, 1600);
        };
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(done).catch(function () {
            legacyCopy(text);
            done();
          });
        } else {
          legacyCopy(text);
          done();
        }
      });
    });
  }

  function legacyCopy(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand('copy');
    } catch (e) {
      /* ignore */
    }
    document.body.removeChild(area);
  }

  /* ---------- 入场动效与导航 ---------- */

  function initReveal() {
    var elements = document.querySelectorAll('.reveal');
    if (!('IntersectionObserver' in window)) {
      elements.forEach(function (el) {
        el.classList.add('is-visible');
      });
      return;
    }
    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: '0px 0px -8% 0px' }
    );
    elements.forEach(function (el) {
      observer.observe(el);
    });
  }

  function initNav() {
    var nav = document.getElementById('nav');
    if (!nav) return;
    var update = function () {
      nav.classList.toggle('is-scrolled', window.scrollY > 8);
    };
    window.addEventListener('scroll', update, { passive: true });
    update();
  }

  /* ---------- 软件截图 ---------- */

  function renderScreenshots(items) {
    var section = document.getElementById('screenshots');
    var grid = document.getElementById('shot-grid');
    if (!section || !grid || !items || !items.length) return;
    var html = '';
    items.forEach(function (item, index) {
      var src = typeof item === 'string' ? item : item.src;
      var alt = typeof item === 'string' ? '' : item.alt || item.title || '';
      var title = typeof item === 'string' ? '' : item.title || item.alt || '';
      var desc = typeof item === 'string' ? '' : item.description || '';
      if (!src) return;
      var isFeatured = index === 0;
      html +=
        '<div class="shot-item' +
        (isFeatured ? ' shot-featured' : '') +
        ' reveal is-visible">' +
        '<div class="shot-img">' +
        '<img src="' +
        escapeHtml(src) +
        '" alt="' +
        escapeHtml(alt) +
        '" loading="lazy" />' +
        '</div>' +
        '<div class="shot-caption">' +
        '<h4>' +
        escapeHtml(title) +
        '</h4>' +
        (desc ? '<p>' + escapeHtml(desc) + '</p>' : '') +
        '</div>' +
        '</div>';
    });
    if (!html) return;
    grid.innerHTML = html;
    section.hidden = false;
  }

  /* ---------- 数据加载 ---------- */

  function loadJson(url) {
    return fetch(url, { cache: 'no-cache' }).then(function (response) {
      if (!response.ok) throw new Error('HTTP ' + response.status);
      return response.json();
    });
  }

  function init() {
    initCopyButtons();
    initReveal();
    initNav();
    initChangelogToggle();

    /* 先渲染回退数据，保证首屏可用，再异步刷新 */
    renderDownloads(FALLBACK.releases);
    renderChangelog(FALLBACK_CHANGELOG);

    // Each feed refreshes independently: one missing file must not freeze all data.
    loadJson('data/releases.json').then(function (data) {
      renderDownloads(data.releases || []);
    }).catch(function () {});
    loadJson('data/changelog.json').then(function (data) {
      renderChangelog(data || []);
    }).catch(function () {});
    loadJson('data/screenshots.json').then(function (data) {
      if (data && data.items) renderScreenshots(data.items);
    }).catch(function () {});
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
