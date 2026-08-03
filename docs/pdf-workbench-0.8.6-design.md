# Docsy 0.8.6 PDF 工作台重整设计

更新时间：2026-08-03

## 目标

0.8.6 不再把页眉、页脚文字和页码视为同一个“页眉页脚”设置。三者从检测、确认、编辑、预览、插入到输出都使用独立状态，并由同一份证据文件清单驱动。

本次同时修复 Microsoft Word 跨平台探测和全局文件列表排序，保证设置页显示的能力与实际转换、合并、拆分和图片排版顺序一致。

## 使用流程

1. 导入 PDF 后后台检测现有页眉、页脚文字和页码。
2. 顶部“原有内容”区域只显示非零统计项：原页眉、原页脚、原页码、待删除、待编辑。
3. 点击统计项打开确认窗口。窗口按文件列出识别文字、页段、来源和处理决定。
4. 用户可以逐项或批量选择删除、忽略、编辑；支持全选和反选。
5. 文件主列表只展示输出所需信息，不再重复展示三列原内容和无业务意义的状态列。
6. 新页眉、页脚文字、页码分别设置。普通预览即时叠加，真实预览手动生成。
7. 执行处理时只处理已确认的决定；“忽略”不会在其他入口重新变成待删除或待编辑。

## 统一对象模型

```ts
type ExistingPdfElementKind = "header" | "footerText" | "pageNumber"
type ExistingPdfElementDecision = "keep" | "delete" | "edit" | "ignore"

type ExistingPdfElement = {
  kind: ExistingPdfElementKind
  detectedText: string
  editedText: string
  normalizedText: string
  source: "artifact" | "content-text" | "manual"
  bbox: PdfBBox | null
  pageStart: number
  pageEnd: number
  confidence: number
  decision: ExistingPdfElementDecision
}

type PageNumberRule = {
  enabled: boolean
  sequence: "continuous" | "per-file"
  style: "arabic" | "chinese" | "roman-upper" | "roman-lower" |
    "circled" | "dingbat"
  template: string
  position: TextPositionRule
  overrides: PageNumberOverride[]
}

type PageNumberOverride = {
  id: string
  scope: "global" | "file"
  start: number
  end: number
  action: "exclude" | "override"
  style?: PageNumberRule["style"]
  template?: string
  position?: Partial<TextPositionRule>
}
```

旧字段在本次迁移中仍可作为处理负载的适配层，但 UI 和业务判断只读取统一对象；不能再分别维护“删除、转换、编辑、忽略”多组互相冲突的布尔值。

## 原有内容确认窗口

### 顶部入口

- 原页眉：只统计检测到的页眉文字。
- 原页脚：只统计非页码的页脚文字。
- 原页码：统计位于顶部或底部的页码候选。
- 待删除：统计决定为 `delete` 的对象。
- 待编辑：统计决定为 `edit` 的对象；“转换”不再作为用户概念。
- 数量为 0 的入口隐藏。

### 窗口内容

- 列：选择、文件名、类型、检测文字、页段、来源、处理、编辑后文字。
- 批量操作：全选、反选、标记删除、忽略、恢复保留。
- 编辑操作同时显示“编辑前”和“编辑后”，可取消编辑恢复检测值。
- 页脚候选冲突也进入同一窗口，不再只藏在预览区域。
- 预览中的决定按钮与窗口操作调用同一 action，不维护第二份状态。

## 检测规则

### 区域与来源

- 检测优先级固定为：标准 Pagination Artifact → Docsy 扩展元数据 → 跨页正文文本启发式；低层结果不得覆盖高层结果。
- Artifact 从实际页面内容流和 Form XObject 遍历，不使用整份 QDF 正则扫描。
- 标准 Artifact 必须以逐项对象进入检测结果，至少包含页、Header/Footer 区域、可提取文字和稳定标识，不能只返回文档级数量。
- 空白或只有装饰线的 Header/Footer Artifact 不作为文字候选。
- 文本扫描区域与删除区域分离，默认至少扫描顶部/底部 25 mm。
- 页码可以位于页眉区域或页脚区域，分类依据文本模式和序列，不依据区域强制归类。

### 页码候选评分

评分至少考虑：

- 页码格式是否匹配。
- 不同页面上的实际出现页数，而不是文字行数量。
- 数字、罗马数字或中文数字是否形成连续序列。
- `{total}` 是否与文件页数一致。
- 候选纵向位置是否稳定。
- 多套页码并存时，优先连续、覆盖页数更多、位置更靠外侧的候选；仍冲突则交给用户确认。

普通百分比、功率、专利号等数字不得仅凭靠近页边就自动成为页码。单页或低重复候选默认进入“需确认”，不直接成为删除目标。

这里不维护百分比、功率、单位或规格字符串黑名单。即使内容形似规格，只要跨页重复且位置稳定，仍应成为候选；反之，即使内容形似页码，没有跨页序列或标准 Artifact，也不能自动采用。

### 标准写入与 Docsy 扩展

- 所有新页眉、页脚文字和页码首先写成 `/Artifact`、`/Type /Pagination`、`/Subtype /Header|/Footer`、`/Attached /Top|/Bottom` 的标准 marked content。
- Docsy 扩展只附加在标准结构上，记录 `ActualText`、版本、稳定 ID 和 `HeaderText|FooterText|PageNumber` 类型；没有扩展字段时，标准结构仍可检测、删除和编辑。
- Docsy 扩展不能取代标准结构，也不能作为其他 PDF 软件识别页眉页脚的必要条件。
- 删除和编辑先处理标准 Artifact；只有不存在可定位的标准对象时，才允许用户确认后处理正文文本。

## 新页眉和页脚文字

页眉和页脚文字使用相同格式模型：

- 启用开关。
- 文件名、证据列表名称或固定文本。
- 前缀、后缀和序号占位符。
- 字体、字号、颜色。
- 左中右对齐、距顶/距底、水平偏移。
- 按文件编辑最终文字。

页脚文字不负责页码。即使页码关闭，页脚文字仍可独立插入。

## 新页码

### 样式

- 阿拉伯数字：`1`。
- 中文数字：`一`。
- 大写/小写罗马数字：`I`、`i`。
- 带圈数字：`①`，仅在完整字符范围内提供。
- 键帽数字依赖彩色 Emoji 与组合字形，PDF 字体嵌入兼容性不足，本版不作为可选样式暴露。
- 实心圈数字：`❶`，仅在完整字符范围内提供。

模板负责前后文字，例如 `{page}`、`-{page}-`、`{page}/{total}`、`第{page}页`。样式只改变 `{page}` 和 `{total}` 的数字系统。

### 编号方式

- 全部文件连续：按最终合并顺序编号。
- 每个文件连续：每个 PDF 从 1 开始。
- 分段/例外：在弹窗中逐条添加，不预置固定段数。

### 分段与例外

- `exclude`：指定页或页段不插入页码。
- `override`：指定页段覆盖样式、模板或位置。
- 未命中规则的页面使用全局页码规则。
- 规则按列表顺序应用，后面的规则覆盖前面的规则。
- 页面范围可按最终合并后的全局页码或单个文件内页码设置。

## 文件列表

- 顺序列前增加拖拽柄。
- PDF 合并、证据工作台、证据扫描分组、拆分页段、图片排版和视频抽帧结果共用 Pointer Events 排序，不使用与 Tauri 文件拖入冲突的 HTML5 DnD。
- 拖拽结束后重新计算文件全局页段和连续页码。
- 保持当前预览文件，不因数组下标变化跳到别的文件。
- 主列表移除：原页眉、原页脚、原页码、现有处理、状态。
- 主列表保留：顺序、文件、新页眉、新页脚文字、新页码样例、页数、全局页段、来源页段、操作。

## Word 探测和转换

### Windows

探测顺序：

1. `HKLM/HKCU App Paths/Winword.exe`，同时查询 32/64 位视图。
2. Word COM 注册 `Word.Application`。
3. Office 常见安装目录和 `where winword`。

路径解析必须支持含空格的未加引号路径。转换仍使用 Word COM；WPS 使用 `KWPS.Application`；两者失败后使用 LibreOffice。

### macOS

探测顺序：

1. `/Applications/Microsoft Word.app` 和 `~/Applications/Microsoft Word.app`。
2. Launch Services/AppleScript 解析 `com.microsoft.Word`。
3. `mdfind` 兜底。

转换使用 AppleScript，保留用户授权所需的原文件父目录访问方式，不把文件复制到另一个临时目录规避授权。

### 状态定义

设置页的“可用”表示实际自动化入口可调用，不仅表示找到一个路径。探测结果应包含来源和失败原因，便于区分“未安装”“COM 未注册”“AppleScript 未授权”。

## 性能和安全

- 默认检测页数受限，不能为检测 Artifact 生成整份 QDF。
- 2000 页文件不在前端创建逐页对象或逐页 canvas。
- 规则只生成必要的连续页段配置，不为每页创建一个前端 overlay 对象。
- 删除普通文本必须匹配文字、页段和 bbox；检测不到时保留原文，不允许白条遮盖。
- 任何低置信度数字候选都默认保留，等待用户确认。

## 0.8.6 验收

1. macOS 和 Windows 安装 Word 后设置页能识别；转换失败显示具体阶段。
2. `Test.zip` 中空白 Word Header Artifact 不显示为原页眉，`1/14` 页码可识别。
3. 同一 PDF 顶部章节页码和底部总页码同时进入确认窗口，用户可分别忽略或删除。
4. `640W 23.7% 1% 0.4%` 不自动成为页码删除目标。
5. 原页眉、原页脚、原页码可逐文件确认，批量全选/反选有效。
6. 忽略决定在确认窗口、预览和最终处理负载中保持一致。
7. 页脚文字与页码可独立启用、独立定位。
8. 连续、单文件、分段、排除页码规则的普通预览和真实输出一致。
9. PDF 合并和证据列表在 Windows/macOS 均可拖拽排序，输出顺序一致。
10. Rust、前端测试、Lint、格式检查和 Windows/macOS 构建全部通过。
