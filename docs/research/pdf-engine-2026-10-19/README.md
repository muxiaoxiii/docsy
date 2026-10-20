# PDF 引擎研究证据

本目录是 2026-10-19 研究的支持材料，不是已接入 Docsy 的实现或测试套件。

- evidence.json：固定提交、依赖/工具、源码指纹、运行命令及测试摘要。
- probe-results.json：合成样例的结构/文字/像素比较结果。
- contact-sheet.png：已目视检查的 11 页渲染总览，全部为合成内容。
- make_fixtures.py：重建两份测试输入，需要独立研究环境中的 reportlab、pypdf。
- docsy_probe.rs：实际运行的临时 example，依赖固定版本 PdfCraft 的 cos/organize API。

## 重现

在独立临时目录检出 PdfCraft 提交 4db6ac4ae2448df6389d8bb0bc54db6ba175721c。
完整检出包含 assets/fonts；若使用 sparse checkout，至少取 crates、vendor、assets/fonts、工作区成员与根配置。

1. 在独立研究 Python 环境安装 reportlab、pypdf；执行 make_fixtures.py --out <临时目录>/probe-data。
2. 将 docsy_probe.rs 复制到检出的 crates/organize/examples/docsy_probe.rs。
3. 在上游仓库执行：

~~~sh
cargo test --locked -p pdfcraft-cos -p pdfcraft-organize -p pdfcraft-edit -p pdfcraft-optimize --lib
cargo test --locked -p pdfcraft-optimize --test optimize
cargo run --locked -p pdfcraft-organize --example docsy_probe -- <临时目录>/probe-data
~~~

4. 在 probe-data 中执行：

~~~sh
qpdf --empty --pages input-A.pdf input-B.pdf -- qpdf-merged.pdf
qpdf --check candidate-merged.pdf
qpdf --check candidate-extracted.pdf
pdftoppm -r 72 -png input-A.pdf input-A
pdftoppm -r 72 -png input-B.pdf input-B
pdftoppm -r 72 -png candidate-merged.pdf candidate-merged
pdftoppm -r 72 -png candidate-extracted.pdf candidate-extracted
pdftoppm -r 72 -png qpdf-merged.pdf qpdf-merged
~~~

5. 按 probe-results.json 的 pixel_comparisons 对应关系比较尺寸和 RGB 像素，并用 pypdf 读回字段、附件、书签、几何与自定义 Info。

本轮使用 qpdf 12.4.1、Poppler 26.09.0。字节哈希可随工具/依赖版本、生成时间与序列化变化；重跑不要求 PDF 字节相同，须核对实际结构与页面内容。不将单次哈希或截图比较代替真实签章、大文件与跨平台验收。

此探针是结构合并，不压平表单/注释，也没有测试 Docsy 打印合并的完整语义。合成附件仅为 A.txt/B.txt；不包含用户真实文书。
