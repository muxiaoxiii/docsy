# PDF 原生改造复审结果

日期：2026-10-19。结论：**本轮不通过合并/发布验收。确认 6 项需修复问题，其中 3 项 P1、3 项 P2。** 本轮只写审查与合成证据，没有修改生产源码。依据当前未提交工作区；基线 HEAD 为 `8746760240ce801156620d8d840d7bf825d9d2f8`。

审查中工作区分派器有更新，最终核验 `select_backend(None)` 已默认 PdfCraft，默认路由缺失不列入最终问题。由于缺陷现已进入默认路径，不能沿用“全部落实并通过验证”的结论。

## 1 P1 过期表单外观检查在真实合并路径中被绕过

位置：`src-tauri/src/pdf/backend/pdfcraft_backend.rs:343-366`，关联 `src-tauri/vendor/pdfcraft/crates/organize/src/import.rs:808-833`。

新增 `flatten_document` 会检查 `/AcroForm /NeedAppearances`，但调用发生在 `Combiner::finish_with_renames()` 之后。Combiner 重建 AcroForm 时只注册 Fields，没有迁移原文档的 NeedAppearances。结果合并后的检查标志已丢失，过期 AP 被当作正常外观固化。

复现 `stale_widget.pdf`：字段值为合成的 `NEW`，NeedAppearances=true，AP 为与该值不符的旧色块。直接调用原文档 flatten 会拒绝；**实际 `PdfCraftBackend::merge(Print)` 返回成功**。PDF.js 对原文档显示 NEW，原生输出则只剩旧色块，Widget 被清除；没有触发 qpdf 回退。

修复要求：在每份输入进入 Combiner 前检查需重建的外观/表单状态，或可靠迁移该状态；覆盖 `dispatch_merge -> PdfCraftBackend::merge -> Combiner -> flatten` 整条路径，而不是只测试 flatten 单函数拒绝。原有 qpdf 回退并不自动保证重建成功，需检查输出后置条件。

## 2 P1 没有 AP 的可见高亮批注被静默删除

位置：`src-tauri/src/pdf/backend/flatten.rs:154-171`，尤其非 Widget 分支 167-171。

该分支只对缺少 AP 的 Widget 返回 Unsupported，对其他可见注释直接移除。常见 Highlight 可由 QuadPoints/C 等参数描述外观而无预生成 AP，阅读器仍可正确绘制。

复现 `highlight_no_ap.pdf`：原文档在 PDF.js/Poppler 显示黄色高亮，qpdf 对照保留可见高亮；**实际原生后端成功返回，但输出已没有高亮，也没有注释对象**。因此“没有绘制到外观”不能直接当作“该注释不需要显示”。

修复要求：为支持的注释生成外观并正确固化；未实现的可见注释返回 Unsupported 走受控回退/报错。不能把移除注释壳作为成功的唯一判据。

## 3 P1 UserUnit 换算改变 raw 坐标，现有 A4 消费方发生位置回归

位置：`src-tauri/src/pdf/page_info.rs:428-434`；消费方 `src-tauri/src/pdf/normalize.rs:156-172`、`341-372`。

本次把 `raw_width_pt/raw_height_pt/box_x0/box_y0` 一并乘上 UserUnit，但 normalize 仍直接将这些值当作原内容流坐标构造 cm。物理 pt 与 PDF 默认用户空间混用，非零 CropBox 的平移量和居中计算随之失真。测试只核对 get_page_infos 的宽高，没有验证真实转换后的内容位置。

复现 `unit_leaf.pdf`（UserUnit=2，CropBox=[20,30,180,190]）：以相同 normalize 参数对比 HEAD 的 page_info 与当前 page_info，蓝色内容在 PDF.js 渲染中的包围盒从 `[436,682,754,1001]` 变为 `[236,902,554,1221]`，出现明显左下偏移。归一化输出还保留 UserUnit=2，实际显示尺寸为 1190.56×1683.78 pt；保留该 UserUnit 的纸张尺寸问题在旧路径也存在，不把它全部归为本次新增回归。

修复要求：分别表达物理尺寸与默认用户空间坐标，迁移 normalize/blank/overlay 等消费方；在规范化时统一处理 UserUnit 和 CTM。验收应比较输出实际纸张尺寸与内容坐标，不能只比较 PageSize 数值。

## 4 P2 书签操作会附带移除原有加密设置

位置：`src-tauri/src/pdf/bookmarks.rs:136-138`、`289-291`。

新增原生书签增删路径遇到已认证的加密 PDF 就调用 remove_encryption，随后原位重写。用户只改书签也会失去原有权限加密设置；这与独立的“移除编辑密码”操作应分开。

复现 `bookmark_encrypted.pdf`：使用空用户密码、非空 owner 密码、AES-256、禁止修改。相同书签调用在 HEAD 的 qpdf 实现完成后仍 encrypted=true；**当前实现完成后 encrypted=false**。未加密对照两版均正常。

修复要求：书签写回保留安全处理器和原有权限；无法在该权限下执行时明确拒绝或采用能保留加密的回退。仅由用户明确选择解锁功能时执行 remove_encryption。

## 5 P2 原生压平忽略 NoRotate，旋转页面上的印章方向改变

位置：`src-tauri/src/pdf/backend/flatten.rs:135-142`、`184-195`、`301-346`。

代码读取 F 后只处理 Invisible/Hidden，外观放置只计算 BBox/Matrix→Rect，没有处理 NoRotate（0x10）与页面 Rotate 的关系。把注释转成普通内容后，原本固定方向的外观会跟随页面旋转。

复现 `stamp_norotate.pdf`：页面 Rotate=90，注释 F=20（Print+NoRotate）。**Poppler 渲染原文与 qpdf 对照均为横向印章，实际原生输出变为纵向**。PDF.js 的本次单 Canvas 渲染对原文此标志表现与 Poppler 不同，故本项以 Poppler 原文与 qpdf 双对照为证，不将 PDF.js 原图作为正确参考。

修复要求：补页面旋转补偿，或在未支持该组合时拒绝候选路径。增加旋转页、非对称印章的视觉回归，不能只断言 Annots 已移除。

## 6 P2 将非继承属性 UserUnit 加入页树继承，会改变文件显示尺寸

位置：`src-tauri/vendor/pdfcraft/crates/organize/src/lib.rs:59`，以及 page_info 的父链读取。

UserUnit 是页面属性；当前 PDF.js 的实现读取 pageDict 自身，而非继承查找。本次把它加入 INHERITABLE，使 Pages 节点上的同名键传播到叶页；合并/提取时又会将其写成真正生效的页属性。

复现 `unit_parent.pdf`：只有父 Pages 带 UserUnit=2，叶页没有。PDF.js 原文显示 160×160 pt、userUnit=1，qpdf 对照保持相同尺寸；**Combiner 输出变为 320×320 pt、userUnit=2**。这也直接否定当前“三条路径与 PDF.js 严格一致”的笼统表述。

修复要求：从通用 INHERITABLE 移除 UserUnit，按叶页自身值和默认 1 处理；仍需继承 MediaBox/CropBox/Resources/Rotate。给父节点含同名额外属性的输入补一次输出渲染对照。

## 已确认通过与验收边界

- 当前 Rust `cargo test --lib`：**435 passed，0 failed，15 ignored**；外部阶段 3–4 契约：**22 passed，0 failed，2 ignored**；版本一致性与 `git diff --check` 通过。
- `word_convert.rs` 相对 HEAD 无修改，Word 链保持不变。没有重新运行客户 Office 文件转换；本轮只使用合成 PDF。
- 本轮未重复前端 Vitest/lint/build；本轮复现均在 Rust/PDF 路径，前端 330 项通过仍属于提交方证据，不能写成本轮重新验证。
- 435 项全量中已经包含 review_probe，不将阶段 0–2 的 10 项再次当作新增独立样本累加。
- qpdf 对 NeedAppearances=true 的既有 AP 只做 flatten 时可返回带警告的成功并保留 Widget；`status_is_success` 接受退出码 3。“回退到 qpdf 就绝无可编辑残留”的承诺需要改为可验证的后置条件或明确失败。
- 没有将上述 6 项当成单元测试失败：现有测试都是绿灯，新探针证明现有覆盖不足。没有用“无外部工具 PATH”推导安装包所有功能均开箱即用。

## 证据与复现

`results.json` 为当前生产模块的隔离运行结果；`pdfjs.json` 记录实际尺寸、UserUnit、注释状态。`inputs/outputs` 均为可分享的合成 PDF，不含客户内容。`contact.png` 展示高亮与过期表单差异；`norotate-poppler.png` 展示印章方向；`normalization-comparison.png` 展示正文位置回归。

探针引用当前生产 `flatten.rs`、`PdfCraftBackend`、`page_info.rs`、`normalize.rs`、`bookmarks.rs` 与 vendored cos/organize；只对未执行的 Word/取消/大文件分段作简单桩，Qpdf 路径固定为本机 `/opt/homebrew/bin/qpdf`。因此它是算法与输出证据，不等于 Tauri 安装包 UI 验收。精确源码摘要见 `source-snapshot.json`。

本机工作目录由 `/tmp/docsy-native-followup-review-path` 指向，执行：

```sh
cargo run --offline --manifest-path <隔离工作目录>/Cargo.toml -- <隔离工作目录>
```

`probe-src/` 保存运行源码，其中 before 模块来自基线 HEAD；拷贝到其他位置时需重定向 `#[path]` 的旧模块路径，以及生产源码仓库路径。生成脚本与 PDF.js 对照脚本也保留了该本机工作目录指针；在别处复现时替换该路径即可。所用依赖已保存 Cargo.lock。

建议先修复以上内容/几何问题，并把默认路由暂时恢复为受控候选，之后再重新验收原生默认开启；不能只新增几个“移除了 Annots”的断言后宣布全部通过。
