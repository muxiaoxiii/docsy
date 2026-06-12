# PDF 证据整理工作流设计

更新时间：2026-06-08

## 目标

证据整理工具面向一个母文件夹：

1. 扫描母文件夹下的子文件夹，读取 PDF、DOC、DOCX，生成可勾选的树状列表。
2. 按子文件夹分别合成 PDF，输出文件名与子文件夹名称一致。
3. 将第一步生成的 PDF 和母文件夹原有 PDF 放入待处理列表，支持排序、增删和身份命名。
4. 对待处理 PDF 添加页眉、页脚、全局页码。
5. 最后按需要合并全部 PDF。

每一步必须可以独立运行；后一步可以从前一步带入结果，但不能强制依赖前一步。

## 当前实现

已新增：

- 后端模块：`src-tauri/src/pdf/evidence.rs`
- PDF overlay 引擎：`src-tauri/src/pdf/overlay.rs`
- PDF 合并后端：`qpdf::merge`、`qpdf::overlay`
- Tauri 命令：
  - `scan_evidence_folder`
  - `build_evidence_group_pdfs`
  - `merge_evidence_pdfs`
  - `overlay_pdf_text`
  - `batch_overlay_pdf_text`
  - `get_pdf_page_count`
  - `check_pdf_pages`
- 前端页面：`src/views/PdfEvidenceView.vue`
- PDF 工具菜单入口：`证据整理`

### 第一步：扫描 + 按子文件夹合成 ✅

1. 选择母文件夹。
2. 扫描子文件夹内 PDF、DOC、DOCX。
3. 默认勾选所有支持文件，支持全选、反选、不选。
4. 按子文件夹生成 PDF。
5. DOC/DOCX 会尝试使用系统 LibreOffice/soffice 转 PDF；未安装时仅该文件失败，不阻断其它 PDF。
6. 母文件夹下已有 PDF 可以带入第二步。
7. 第一步输出 PDF 可以带入第二步。

### 第二步：整理 PDF 身份 ✅

8. 支持拖拽排序（vuedraggable）。
9. 支持手动添加 PDF 文件。
10. 支持移除 PDF。
11. 批量命名模式：
    - 不改名
    - 前缀
    - 前缀+序号
    - 序号
    - 序号+后缀
    - 后缀
    - 日期+序号
12. 序号起始值可配置。

### 第三步：页眉页脚 ✅

13. 页眉来源：文件名 / 自定义文本 / 序号（证据1）/ 中文序号（证据一）/ 固定前缀+序号。
14. 页脚页码：当前页/总页数，基于合并后全局页数计算。
15. 各子文档页数范围预览（如 1-12, 13-20）。
16. 页面尺寸检查：检测横向/非 A4 页面并警告。
17. overlay 另存副本，不修改原始 PDF。
18. 页眉页脚引擎：printpdf 生成透明文字层 + qpdf --overlay 合成。

### 第四步：合并 PDF ✅

19. 合并全部 PDF（qpdf --pages）。
20. 合并时优先使用 overlay 后的副本。

## 设计原则

1. 原文件不覆盖。
2. 每一步都有明确输出目录。
3. 上一步失败项不阻断其它成功项。
4. 页眉页脚采用另存副本，保证可重复操作。
5. 全局页码必须基于最终待合并列表计算。

## 代码质量改进

- `resolve_soffice()` 找不到 LibreOffice 时返回 `None`（之前错误地返回 `Some("soffice")`）
- `natural_key()` 支持自然数排序（"证据2" 排在 "证据10" 前面）
- 8 个新增单元测试覆盖排序、文件名清理、路径去重、文件类型判断

## 尚未实现

### 页面规范化

- 自动把非 A4 页面规范化到 A4
- 自动旋转横向页面

### DOC/DOCX 转 PDF 增强

- 在设置页显示转换器检测状态
- 允许用户配置 LibreOffice 路径
- Windows/macOS 分别给出安装提示
