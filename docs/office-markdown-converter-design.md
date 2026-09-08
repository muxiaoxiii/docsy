# MD 转换模块：Office 与 Markdown 互转

## 目标

模块名称保持为“MD 转换”，但能力是本地优先的 Office 与 Markdown 双向转换：Markdown 可生成多种可编辑 Office 文件，常见 Office 文件也可整理为可读 Markdown。它处理的是文档结构、正文与常见文字样式；Markdown 本身无法承载 Office 的精确页面版式、宏、图表动画、批注或修订记录，因此这些信息不会被伪装成“无损保留”。

## 支持矩阵

| 输入 | 输出 | 引擎 | 说明 |
| --- | --- | --- | --- |
| `.md` / `.markdown` / 常见 MD 扩展 | `.docx` | Docsy Markdown 渲染器 | 保留标题、列表、表格、代码、链接、图片；提供 Word 版式预设。 |
| `.md` / `.markdown` / 常见 MD 扩展 | `.xlsx` / `.pptx` | `office_oxide` 文档模型 | 将 Markdown 结构转换为可编辑工作簿或演示文稿。 |
| `.docx` / `.docm` | `.md` | Docsy DOCX 读取器，失败时 AnyDoc | 保留标题、列表、表格、加粗、斜体、删除线、链接、图片引用等语义。 |
| `.doc` / `.xls` / `.xlsx` / `.xlsm` / `.ppt` / `.pptx` / `.pptm` | `.md` | Docsy 内置的 AnyDoc fork | 统一提取标题、列表、表格、文字样式、批注/讲者备注等可表达内容。 |
| `.odt` / `.ods` / `.odp` / `.rtf` / `.csv` | `.md` | Docsy 内置的 AnyDoc fork | 扩展办公文档与文本表格的导入范围。 |

Markdown → Excel / PowerPoint 仅创建结构化可编辑内容，不等同于人工排版的工作簿或演示文稿。图表、嵌入对象、SmartArt、复杂公式和精确页面布局不会凭空生成。

## 架构

```text
输入识别
  ├─ Markdown（`.md`、`.markdown`、`.mdown`、`.mkdn`、`.mdwn`、`.mdtxt`）→ 目标 Office 格式
  │    ├─ DOCX: md_to_docx（Docsy 样式预设）
  │    └─ XLSX / PPTX: office_oxide DocumentIR
  └─ Office → Markdown
       ├─ DOCX / DOCM: docx_to_md（图片导出与结构恢复）
       ├─ 旧 DOC: 可选 Word/WPS 中转（高保真）或 AnyDoc 直接提取
       └─ Excel / PowerPoint / OpenDocument / RTF / CSV: `vendor/docsy-anydoc` 统一文档模型
```

`vendor/docsy-anydoc` 是 Docsy 维护的 AnyDoc fork，而不是运行时调用外部程序或直接依赖的黑盒。它保留各 Office 格式解析器和统一文档模型，移除了 PDF 通路；Docsy 在 `docsy.rs` 中定义自己的文件读取、格式判定、Markdown 规范化、嵌入资源导出与元数据边界。扩展格式内的图片会写入与 Markdown 同级的 `<文件名>_assets` 目录并使用相对路径引用。转换边界、输出路径、结果大小和警告仍统一由 `markdown/mod.rs` 负责；前端只选择输入、Markdown 输出格式与版式预设，旧 `.doc` 才额外询问是否用本机 Word/WPS 中转。

## Office 版式预设

| 预设 | 用途 | 关键规则 |
| --- | --- | --- |
| `professional` | 默认通用文档 | Word 使用宋体正文、黑体标题、Times New Roman 西文、A4版心与规范1.5倍行距；表格保留可读边框和表头层次。 |
| `legal` | 法律文书、证据说明 | Word 使用仿宋正文、黑体标题、Times New Roman 西文、首行缩进两字符与装订线加宽。 |
| `compact` | 工作笔记、交付草稿 | Word 使用宋体正文、黑体标题、Times New Roman 西文、紧凑字号和间距，适合信息密集内容。 |

预设只影响 Markdown 新生成的 DOCX；Excel 和 PowerPoint 会使用统一的清晰结构模板，不会改写导入的 Office 文件。

## 验收

1. 常见 Markdown 扩展名可加入队列并按选择的目标格式导出。
2. Markdown 可选择生成 `.docx/.xlsx/.pptx`，输出自动避让重名文件。
3. Word 文档支持三种版式预设，生成包可由 `docx-rs` 读回；Excel 和 PowerPoint 生成包可由 Office 文档读取器回读。
4. Word、Excel、PowerPoint、OpenDocument、RTF 与 CSV 可转换为 Markdown；宏永不执行。
5. 不能表达的精确版式、宏、动画、公式与修订记录以警告呈现，不静默伪装为无损。
6. 队列、粘贴转换和拖放使用同一套格式判断与后端参数。
