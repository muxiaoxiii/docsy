# Docsy 0.7.0 未提交变更代码评审报告

评审日期:2026-07-20 · 评审对象:工作区全部未提交变更(54 个修改文件 + 22 个新增,+5525/-2632 行)· 评审方式:只读评审,未修改任何源代码

## 0. 评审范围与方法

**范围**:相对 HEAD 的全部 diff + untracked 新文件,包括:模板系统(Rust `docx_template.rs`/`template_history.rs` + 前端 `src/modules/template/`)、PDF 管线重写(`header_footer.rs`/`detection.rs`/`qpdf.rs`/`evidence.rs`)、EvidencePdfWorkbench 重构(4 个新 composable)、外部工具链(`managed.rs`/ffmpeg/word/wps)、Doclet 工作动画(`DocletWorkingPet.vue` + `App.vue` + `tauriBridge.js`)。

**方法**:7 路并行评审(R1–R7)→ 全部 Critical/Major 发现由独立核验 agent 重读代码复核(14 条送核:9 条确认、5 条降级、0 条驳回)。

**基线**:`cargo check` ✓;Rust 测试 117 通过;vitest 55 通过(8 文件)。已有测试覆盖且通过的行为未重复报告。

**严重度定义**:Critical=数据丢失/任意写/崩溃/安全;Major=错误结果、竞态→坏状态、契约破坏;Minor=边界缺失、静默失败、误导性错误;Suggestion=可维护性。

**动画专项(用户提问)**:见 §3.7,含 56 个命令的动画覆盖三列表格。

## 1. 结论摘要

| 严重度 | 数量 | 说明 |
|---|---|---|
| Critical | 0 | 原 1 条(zip bomb)经核验降为 Major(桌面单用户场景,仅应用级 DoS) |
| Major | 11 | 其中 2 条继承自 HEAD(renderTruePreview 竞态、拆分重入守卫),本次重构为修复窗口 |
| Minor | 35 | 以静默失败、边界缺失为主 |
| Suggestion | 18 | 见附录 6.2 |

**Top 3 风险**:

1. **模板引擎的正则 XML 手术族问题(M-02/M-03/M-05/M-06,同根因)**:`docx_template.rs` 用非贪婪正则解析 OOXML,遇嵌套结构(文本框 txbxContent、既有内容控件 sdt、嵌套表格)即静默产出错位或损坏的法律文书。4 条 Major 同一根因,建议按"族"整体治理(统一 run 枚举、quick-xml 迁移)。
2. **WPS 转换无差别强杀进程(M-08)**:转换期间用户自己打开的 WPS(含未保存文档)会被 `Stop-Process -Force` 杀掉——真实的用户数据丢失路径。
3. **zip 解压无大小上限(M-01)**:5 处 `read_to_end`/`read_to_string` 无上限,恶意 docx/.docsytpl 可 OOM;仓库 `managed.rs` 已有成熟 size-cap 范式未复用。

**整体评价**:基础设施改造质量高(run_blocking 统一、TempPathGuard RAII、字体子集化、HTTPS/大小上限、动画机制设计精巧);主要风险集中在模板引擎的 XML 正则处理与若干"静默失败"类边界。继承自 HEAD 的 2 条 Major 建议借本次重构窗口一并修复。

## 2. 严重度总表

| 编号 | 级别 | 位置 | 标题 | 核验 |
|---|---|---|---|---|
| M-01 | Major | docx_template.rs:736 等 5 处 | zip 解压读取无大小上限(zip bomb) | confirmed(由 Critical 降级) |
| M-02 | Major | docx_template.rs:13,745,852 | txbxContent 嵌套段落致 scan/wrap run_index 错位 | confirmed |
| M-03 | Major | docx_template.rs:28,2151 | 既有 sdt 嵌套被 SDT_RE 截断,输出语义损坏/malformed | confirmed |
| M-04 | Major | docx_template.rs:23,855 | 保存模板时非黄色高亮被静默清除 | confirmed |
| M-05 | Major | docx_template.rs:1290-1348 | 同行两个 party_list 字段复制产生笛卡尔积 | confirmed |
| M-06 | Major | docx_template.rs:15,1336 | TABLE_ROW_RE 遇嵌套表格复制残缺 XML,docx 损坏 | confirmed |
| M-07 | Major | header_footer.rs:379,783 | 删除重建 overlay 与主 header 同页同位叠加 | confirmed |
| M-08 | Major | evidence.rs:443 | WPS finally 无差别强杀新 wps 进程,可致用户数据丢失 | confirmed |
| M-09 | Major | TemplateView.vue:1690 | normalizeFieldRows 静默删除用户手动连接符前缀行 | confirmed |
| M-10 | Major | TemplateView.vue:2942 | 拆分对话框对合并行生成错误 markRefs,字段静默缺失 | confirmed |
| M-11 | Major | useEvidencePdfPreview.js:249 | renderTruePreview 无序号守卫,旧响应覆盖新预览(继承 HEAD) | confirmed |
| m-01…m-35 | Minor | 见 §3 各节 | — | — |
| s-01…s-18 | Suggestion | 见附录 6.2 | — | — |

## 3. 分项发现

### 3.1 R1/R2 — 模板引擎(`docx_template.rs`/`template_history.rs`/`commands/template.rs`)

#### M-01 zip 解压读取无大小上限,恶意 docx/.docsytpl 可致 OOM【核验:confirmed,Critical→Major】
- **位置**:`docx_template.rs:707-708、735-736、2208-2209、2280-2282、2290-2296`
- **证据**:
  ```rust
  730:  let mut file = archive.by_index(idx)?;
  735:  let mut xml = String::new();
  736:  file.read_to_string(&mut xml)?;   // 无上限,另 4 处同
  ```
- **触发与影响**:打开/渲染构造的 zip bomb(压缩几十 MB、解压数 GB)→ 内存耗尽崩溃。.docsytpl 是可分享交换格式,接收他人模板打开即可触发。降级理由:桌面单用户、需用户主动打开恶意文件、无代码执行。
- **建议**:复用 `external/managed.rs:346-350` 范式——读取前校验 `file.size()`(单 entry 如 128MB + 累计上限),或 `file.take(limit+1)` 截断读取超限报错;`archive.len()` 设条目数上限;`std::fs::read`(L2187)加文件大小预检。

#### M-02 txbxContent 嵌套段落导致 scan 与 wrap 的 run_index 错位【核验:confirmed】
- **位置**:`docx_template.rs:13-14(PARAGRAPH_RE)、745-773(scan)、852-865(wrap)`
- **证据**:
  ```rust
  14:  Regex::new(r#"(?s)<w:p\b[^>]*>.*?</w:p>"#)   // 非贪婪
  745: for (paragraph_index, paragraph) in PARAGRAPH_RE.find_iter(xml).enumerate() {
  748:     for run in RUN_RE.find_iter(paragraph.as_str()) {   // scan:只数段落内
  852: for mat in RUN_RE.find_iter(xml) {                      // wrap:数全文
  ```
- **触发与影响**:docx 段落含文本框(`<w:txbxContent><w:p>…`,法律文书封面案号框常见)且文本框后同段落还有 run → 外层 `<w:p>` 匹配在内层 `</w:p>` 提前结束,其后的 run 不被 scan 计数但被 wrap 计数 → 该点之后 run_index 全部偏移 → **sdt 控件绑到错误文字上、且 UI 看不到这些标黄**,全程无报错。`validate_mark_specs` 无法发现(只查 id 存在性)。
- **建议**:scan 与 wrap 共用同一 run 枚举器(对全文 RUN_RE 枚举,段落归属另行计算),消除"两套计数必须一致"的隐式不变量;短期先检测 txbxContent 场景并拒绝保存 + 补回归测试。

#### M-03 源文档既有内容控件(sdt)嵌套导致输出语义损坏/malformed【核验:confirmed,措辞修正】
- **位置**:`docx_template.rs:28-29(SDT_RE)、852-866、2151-2162、1846-1866`
- **触发与影响**:源 docx 自带 Word/WPS 内容控件且标黄 run 落在其内 → wrap 二次包裹成嵌套 sdt(无检测)→ 渲染时外层 SDT_RE 在内层 `</w:sdt>` 截断。核验修正:普通替换路径下输出碰巧良构但**既有 content control 被静默销毁、内容迁移进错误 sdt**;`remove_optional_sdt`(字段设 optionalRule 且值为空)路径下输出**真正不闭合的 XML**,Word 打不开。
- **建议**:保存时检测 mark 是否已位于既有 sdt 内并明确报错(或剥离);渲染前断言 sdt 不嵌套;长期迁移 quick-xml。

#### M-04 保存模板时非黄色高亮被静默清除【核验:confirmed】
- **位置**:`docx_template.rs:23-25(HIGHLIGHT_RE)、352-353、855-860、954-956`
- **证据**:
  ```rust
  24:  Regex::new(r#"(?s)<w:highlight\b[^>]*/>|<w:highlight\b[^>]*>.*?</w:highlight>"#)  // 任意颜色
  859:     out.push_str(&strip_highlight(run_xml));   // 未绑字段的 run 一律剥除
  ```
- **触发与影响**:源 docx 含绿色/红色审阅高亮 → 保存模板时 marks 由全部非空 run 构成(L352-353 `text_runs_as_marks` 无过滤),未绑字段 run 走 `strip_highlight` 剥除任意颜色 → 用户原有高亮格式在模板及所有渲染产物中静默丢失。`has_yellow_highlight`(L2306)只服务 inspect UI,保存路径不用。
- **建议**:`strip_highlight` 只剥 `w:val="yellow"`(含单引号变体);或保存流程把处理集限定为黄色 run,其余原样输出。补非黄色保留测试。

#### M-05 同一表格行含两个 party_list 字段时行复制产生笛卡尔积【核验:confirmed】
- **位置**:`docx_template.rs:1290-1317、1336-1348`
- **触发与影响**:模板行 `| 原告{A} | 被告{B} |`(A 2 项、B 3 项)→ 逐字段串行复制:先为 A 复制 2 行(B 的 sdt 因 replace_sdt_values 原样回写而存活),再为 B 对每行各复制 3 次 → 输出 6 行交叉组合,当事人张冠李戴。全文无"一行多 party_list"检测。
- **建议**:复制前检测行内其他 party_list tag,存在则报错提示作者拆行;或改为"行内多字段逐项 zip 对齐"语义一次展开。

#### M-06 TABLE_ROW_RE 遇嵌套表格复制残缺 XML,docx 损坏【核验:confirmed】
- **位置**:`docx_template.rs:15-16、1336-1348`
- **触发与影响**:party_list 行单元格内含嵌套 `<w:tbl>` → `.*?</w:tr>` 在内层表第一行提前结束 → 残缺块(缺外层 `</w:tc></w:tr>`)被按 items 复制 → document.xml 结构非法,Word 报"无法读取的内容"。触发条件:嵌套表出现在 tag 之前/同段。
- **建议**:复制前校验块内 `<w:tbl>`/`</w:tbl>` 计数平衡,不平衡则回退单值替换并告警;中期 quick-xml 按标签配对定位。

#### m-01 .docsytpl 打开校验缺口【Minor】
`docx_template.rs:2276-2298`:缺 manifest.json/template.docx 时抛英文底层 zip 错误;`format_version` 不校验(未来版本被静默按 v1 解析);`field_type` 无白名单(未知类型静默按 text)。建议:中文上下文错误 + 版本/类型显式校验。

#### m-02 同一 mark 被多字段重复引用时后续 spec 静默丢弃【Minor】
`docx_template.rs:574-592、902-925`:`validate_mark_specs` 只查缺失不查重复占用,重叠区间 `start < cursor` 被跳过 → 第二个字段的 tag 永不写入,值静默缺失。建议:validate 增加跨字段重复 mark_id 检测(字段内拆分合法,需分组判断)。

#### m-03 date_value 不校验月日数字【Minor】
`docx_template.rs:1560-1578`:`"2026-13-45"` → "2026年13月45日";ISO datetime → "23T10:00:00日"。建议:month/day 解析 u8 并校验 1..=12/1..=31。

#### m-04 inspect 返回的 converted_path 是悬空路径【Minor】
`docx_template.rs:257-267、321-325`:函数返回时 guard 已删文件,返回必失效路径;且 inspect 转一次、save 又转一次(冗余)。建议:移除该字段或缓存复用,写清契约。

#### m-05 .doc 转换临时路径可预测 + 竞态【Minor,由 Major 降级】
`docx_template.rs:295-309`:fnv1a(源路径) 无 PID/随机,两实例转换同一 .doc 时 exists→save_as TOCTOU 窗口,TempPathGuard 互删。降级理由:桌面单用户、最坏转换报错可重试、无数据丢失。建议:`tempfile::Builder` 随机名或加入 PID。

#### m-06 record_generation 失败时输出文件已落盘【Minor】
`docx_template.rs:500-511`:历史写入失败 → 前端收"生成失败"但文件已生成(孤儿),重试还产 `-1` 副本。建议:历史失败降级为日志警告不阻断 Ok;或失败时清理已写文件。

#### m-07 rusqlite 未设 busy_timeout【Minor】
`template_history.rs:300-306`:并发命令(生成+刷新历史)偶发 "database is locked"(并放大 m-06)。建议:`conn.busy_timeout(5s)`。

#### m-08 semanticSuggestions 不排除当前模板,建议列表重复【Minor】
`template_history.rs:439-455` + `TemplateView.vue:3867-3868`:同模板同值同时出现在 fieldSuggestions 与 semanticSuggestions,前端直接拼接。建议:语义查询加 `AND template_id != ?3`。

#### m-09 optionalRule 前后缀移除要求落在单个 w:t 内,Word 拆 run 时静默失败【Minor】
`docx_template.rs:2093-2119`:前后缀被 Word 拆成多 run(拼写检查边界常见)→ ends_with/starts_with 匹配失败静默返回 → 空值字段残留",第三人"类文字。建议:扩展匹配到连续多 w:t 逆序截断,或保存时校验并提示。

---

### 3.2 R3 — PDF 管线(`header_footer.rs`/`detection.rs`/`qpdf.rs`/`commands/pdf.rs`)

#### M-07 删除重建 overlay 与主 header/footer 同页同位叠加,页眉文字重叠【核验:confirmed】
- **位置**:`header_footer.rs:379-389、783-795、595-639`;前端 `useEvidencePdfSession.js:209-212、369-381`
- **证据**:
  ```rust
  387:     &BTreeSet::new(),   // skip_header_pages 恒空
  388:     &BTreeSet::new(),   // skip_footer_pages 恒空
  783: if let Some(config) = header.filter(|config| {
  784:     !skip_header_pages.contains(&index) && overlay_applies_to_page(config, local_page)
  ```
- **触发与影响**:文件带标准 artifact 页眉且用户编辑其文本(前端生成 headerReplacement),原位编辑失败走删除分支生成重建 overlay,同时全局规则仍下发 args.header(证据标签)→ 重建 overlay 与主 header 坐标同源(同 marginMm/region/基线公式)→ **同页同位置绘制两段不同文字,输出花字**;附带 warning 未提示叠加。
- **建议**:把 removed_header_pages/removed_footer_pages 传入 skip_header_pages/skip_footer_pages(参数本就预留,当前恒空——即最小修复点);或前端互斥两种配置。

#### m-10 早退分支丢弃已累积 warnings,普通文本页眉可被静默删除【Minor】
`header_footer.rs:359-376`:plain 页眉删除 + headerReplacement + 无其他配置时,`plain_text_processing_warnings` 已 push 但返回 `warnings: Vec::new()`,且该场景删而不建。建议:早退返回 `std::mem::take(&mut warnings)`,并确认删而不建是否预期。

#### m-11 content stream 字号采样对 Identity-H CID 字体(中文 PDF 主流)静默失效【Minor】
`detection.rs:721-735`:无 BOM 的 2 字节 CID 文本 from_utf8 失败 → PDFDocEncoding 逐字节乱码 → 采样永不命中,font_size 恒 None(回退 bbox 推断,精度差)。建议:读 ToUnicode CMap 映射,或注释声明限制。

#### m-12 qpdf --is-encrypted 退出码 2 + stderr 非空一律误报【Minor】
`qpdf.rs:37-47`:可自动修复的受损 PDF(WARNING + 退出码 2)从旧实现的"正常返回"回归为整体报错。建议:退出码 2 视为未加密,仅 stderr 含 error/invalid 时才报错。

#### m-13 CFF 子集字体以完整 sfnt 嵌入 FontFile3/CIDFontType0C【Minor】
`header_footer.rs:1105-1116`:仅 Linux NotoSansCJK 路径命中;Acrobat 容忍但 veraPDF 等合规校验器可能拒绝(法律文书偶有此需求)。建议:CFF 分支提取裸 CFF 表,或注释说明。

---

### 3.3 R4 — 命令层/外部工具(`evidence.rs`/`managed.rs`/`external/`/ffmpeg 等)

#### M-08 WPS 转换 finally 无差别强杀所有"新出现"的 wps 进程【核验:confirmed】
- **位置**:`evidence.rs:423-447`
- **证据**:
  ```powershell
  425: $before=@(Get-Process wps ... | ForEach-Object { $_.Id });
  443: Get-Process wps ... | Where-Object { $before -notcontains $_.Id }
       | ForEach-Object { Stop-Process -Id $_.Id -Force ... }
  ```
- **触发与影响**:转换期间(finally 每次必执行)用户自己打开 WPS 编辑**未保存**文档 → 该 PID 不在快照 → 被 -Force 强杀,**未保存内容丢失且无提示**;第二个 Docsy 实例并发转换互杀。单实例内 A/B 文件互杀不成立(顺序循环)。仅 Windows+WPS 路径。
- **建议**:只针对本次 COM 激活产生的 PID(New-Object 前后差集)先 CloseMainWindow 等待、超时再 Force;或只 Quit COM 对象,残留交下次 $before 豁免,有可见错误时提示用户手动关 WPS。

#### m-14 macOS Word 转换脚本:save as 参数类型存疑、Word 不退出、转换链无超时【Minor】
`evidence.rs:473-520`:`save as ... file name` 传 POSIX file 对象(部分 Word 版本 coercion -1700 失败);无 `quit` 批量后 Word 常驻;三条转换子进程全用无超时 `.status()`,模态对话框可永久阻塞线程(新引入的 idle-timeout 设施未用于转换链)。建议:file name 改文本、脚本加 `quit saving no`、统一接入 `command_output_with_idle_timeout`。

#### m-15 HTTPS-only 校验不覆盖重定向,https→http 降级被静默跟随【Minor】
`managed.rs:241-280`:reqwest 默认跟随重定向不限 scheme。建议:`redirect::Policy::none()` + 逐跳 validate,或自定义 Policy 拒绝非 https 跳转。

#### m-16 解压大小上限基于 zip 声明尺寸,构造条目可绕过【Minor】
`managed.rs:346-355`:`file.size()` 是声明值,deflate 实际输出由流决定。建议:包装 Write 计数实际字节,超限中止删部分文件。

#### m-17 ffprobe `-v quiet` 下 45s "idle" 超时实为总时长上限,可误杀大文件 probe【Minor】
`ffmpeg/probe.rs:5,22-32`:静默命令全程无输出,last_activity 不刷新。建议:静默命令用总超时语义(或去 quiet),错误文案区分"总超时/无输出超时"。

#### m-18 ffmpeg 无效时间字符串静默忽略;start 超时长时 0 帧静默成功【Minor】
`ffmpeg/extract.rs:122-146`:`"abc"` → 视为未提供 → 全片抽帧;超时长 → count:0 成功。建议:字段存在但解析失败时 bail;0 帧时返回 warning。

#### m-19 read_image_data_url 为同步命令,最坏主线程读 100MB + base64【Minor】
`commands/system.rs:37-63`:上限逻辑本次刚加但未异步化。建议:改 `run_blocking`,或收紧上限至 20MB。

---

### 3.4 R5 — template 前端模块(`TemplateView.vue` 4867 行 + `publicRules.js`)

#### M-09 normalizeFieldRows 静默删除用户手动设置的连接符前缀行【核验:confirmed】
- **位置**:`TemplateView.vue:1690-1696、2847、2507`
- **证据**:
  ```js
  1695: return rowUsage(row) === 'prefix' && (String(row.rowId || '').includes(':auto-connector') || row.label === '连接符')
  2847: row.label = usage === 'prefix' ? (isConnectorRow(row) ? '连接符' : '前缀') : '后缀'
  ```
- **触发与影响**:用户把纯连接符标黄片段(如 ",")手动改为"前缀" → label 被设为 '连接符' → 之后任一 normalizeFieldRows 触发(从预览新增标记 L2507、重扫 L1559)→ 该行被 `dropGeneratedConnectorRows` **无提示删除** → 保存模板丢失"字段为空删除连接符"规则,生成文书残留孤立连接符(病句)。核验确认:`':auto-connector'` 在全仓**无任何创建点**(死分支),唯一生效判别式与用户行正面冲突。
- **建议**:自动生成的连接符行打 rowId 标记(`:auto-connector` 真正创建),删掉 `label === '连接符'` 判别;或仅未改过 usage 的行允许 drop。

#### M-10 拆分对话框对多 mark 合并行生成错误的 markRefs【核验:confirmed】
- **位置**:`TemplateView.vue:2912-2957`;对照正确实现 `refsForRowTextRange:1940`
- **证据**:
  ```js
  2942: markRefs: [{ markId: row.markId,                        // 永远第一个 mark
  2945:           start: [...row.text.slice(0, start)].length,  // 相对拼接文本的偏移
  ```
- **触发与影响**:autoMergeMarks 合并的行(如日期两个 run)被"拆分",片段落在第二个 mark 范围 → markRef 指向错误 mark 且偏移越界 → 后端 validate 只查 id 存在(通过)→ 渲染期越界被钳成 start==end 后**静默丢弃**(wrap_run_with_specs L891-893)→ 保存成功假象 + 生成文书静默缺字段。附带:L2934 UTF-16 偏移与 L2945 code point 偏移混用,surrogate pair 文本算错。
- **建议**:applySplitDialog 改用 `refsForRowTextRange(row, start, end)`;统一偏移单位;后端 validate 补 start/end 边界校验。

#### m-20 applyHistoryRun 在打开模板失败后仍继续填值并报成功【Minor,由 Major 降级】
`TemplateView.vue:3335-3341、3374-3387`:openTemplatePackage 先写 templatePath 再 await,失败不回滚;applyHistoryRun 不检查成败 → "错误+成功"双 toast,历史值填进另一模板表单。场景现实:模板移入回收站后历史记录仍可点"填入"(list_generation_runs 不过滤存活性)。建议:成功后才写 templatePath 并返回 boolean,失败中止后续。

#### m-21 openPath 与历史上下文共 3 处静默吞掉失败【Minor】
`TemplateView.vue:791、3578、3434`:文件已删时"打开文档"毫无反应;生成成功提示已弹但文档没打开。建议:openPath 统一 `if (!r.ok) ElMessage.warning(...)`。

#### m-22 推断命名依赖全局 fieldRows 长度,重复扫描结果漂移【Minor】
`TemplateView.vue:1719、1559、2524`:同一文档二次扫描同一片段生成名从 '字段1' 变 '字段{N+1}'。建议:改用管线内局部 rows 长度。

#### m-23 用户手动合并的行会被后续 normalize 静默重拆/丢连接符【Minor】
`TemplateView.vue:2507、1713-1741`:splitLeadingConnectorRow 漏检 userSelectedType(其他 split 函数已检)。建议:合并行打标记,管线各 split 放行。

#### m-24 inferPartyRole 忽略 targetType,住所地/法定代表人被误判为 party_list【Minor】
`publicRules.js:206-248`:PUBLIC_ROLE_PREFIX_RULES 中 targetType:'text' 的规则被强制 buildInference('party_list',...)。建议:保留 targetType 并补测试。

#### m-25 openTemplatePackage/loadHistoryContext 无并发守卫,快速切换模板时建议数据错位【Minor】
`TemplateView.vue:3374-3447`:A 的响应后返回覆盖 B 的 historyContext。建议:请求序号或 templatePath 快照比对。

---

### 3.5 R6 — evidence-pdf 重构 + 前端核心

#### M-11 renderTruePreview 无请求序号守卫,旧响应覆盖新预览【核验:confirmed,继承自 HEAD】
- **位置**:`useEvidencePdfPreview.js:249-277`
- **触发与影响**:确切时序 t0 发起 A 渲染 → t1 切到 B(selectPreviewRow 只清 truePreview,不取消在途)→ t2 A 响应写回 `truePreview`/`previewData` → 面板显示 A 的图,且 **previewData 被旧页尺寸覆盖导致标记坐标换算错误**(终态错位,用户无感知)。与 HEAD 逐行一致(继承问题);同仓 `PdfJsPreview.vue:73,171` 已有 requestId 正确范式未复用。
- **建议**:`const reqId = ++truePreviewReqSeq`,await 后校验序号再写回;selectPreviewRow/movePreviewPage 中递增序号使在途失效。**借本次重构窗口修复**。

#### m-26 detectAllHeaderFooter 无 try/finally,中途抛错按钮永久 loading【Minor,继承 HEAD】
`useEvidencePdfDetection.js:34-62`:后端返回 ok 但 data 畸形 → L76 抛 TypeError → L61 复位永不执行,且 L36 守卫使后续全部静默 return。建议:try/finally + 异常路径 ElMessage。

#### m-27 importMergedPdfAsEvidence 守卫在 await open() 之后才置位【Minor】
`useEvidencePdfMergedImport.js:67-83`:文件对话框停留期间(无 modal 缓解)可重复触发,计划状态被两次赋值。建议:守卫通过后立即置位,取消时复位。

#### m-28 executeMergedImportPlan 重入守卫在 await confirm 之后【Minor,由 Major 降级,继承 HEAD】
`useEvidencePdfMergedImport.js:190-219`:检查-置位窗口存在,但 ElMessageBox modal 遮罩+焦点囚禁使 UI 层第二次触发实际不可达,仅理论程序化穿透。建议:仍应进入函数即置位(防御式)。

#### m-29 操作指示器(docsy-operation-start/finish)并发时提前隐藏、label 互相覆盖【Minor,新引入】
`App.vue:87-98` + `tauriBridge.js:17-27`:无计数管理,先结束者的 finish 隐藏面板。建议:维护 pending 命令集合,非空才显示。

#### m-30 image-paddler 拖拽过滤把"名称含点的文件夹"误判丢弃【Minor】
`ImagePaddlerView.vue:697-702`:"2024.05 扫描件" 伪扩展名不在白名单 → 整个文件夹被静默丢弃。建议:仅已知非图片类型(.pdf/.docx 等)才过滤,其余放行交后端。

#### m-31 tauriCallSafe 返回值抽查:2 处静默【Minor】
`SettingsView.vue:458-462`(open_log_dir/open_log_file)、`TemplateView.vue:3578`(见 m-21)。另确认:`void logError(...)` fire-and-forget **安全**(appLogger.send 内 try/catch 不会 reject,无 unhandled rejection)。

---

### 3.6 R7 — Doclet 工作动画专项(回答"有没有全面覆盖")

**结论:触发机制覆盖 = 100% 全面;文案(label)覆盖 = 不全面。**

- **机制**:所有前端命令调用均经 `tauriCall`/`tauriCallSafe` 统一派发 start/finish 事件(唯一绕过点 `write_frontend_log` 是防止日志递归的**有意设计**,正确)→ 任何命令 ≥350ms 都会触发动画,无"该出不出的死路径"。start/finish 用 try/finally 配对,异常路径无泄漏。
- **动画本体质量高**(逐帧核验):spritesheet 实测 1536×2288,CSS background-size 1152×1716(0.75×)→ 8 列×11 行、每帧 144×156 与元素尺寸**整数对齐无裁剪**;16 帧 × 6.25% = 100%,5.6s/16 = 350ms/帧,steps(1,end) 无跳帧;`prefers-reduced-motion` 已处理;纯 CSS 无 rAF 泄漏;文本插值无 XSS;350ms 防闪烁延迟 + onBeforeUnmount 清理,设计正确。

**发现的问题**:

#### m-32 extract_pdf_pages 无专属 label,回落通用文案【Minor】
`tauriBridge.js:4-15`:与 merge_pdfs/compress_pdf 同视图但体验不一致。补:`extract_pdf_pages: 'Doclet 正在提取页面…'`。

#### m-33 工具下载命令(install_external_tool×2)无 label,几十 MB 下载全程通用文案【Minor】
`tauriBridge.js:4-15`;`managed.rs:224`(ffmpeg 80+MB)。**最长耗时的操作信息最少**,易误以为卡死。建议补 label;后续可做下载进度事件复用同一面板。

#### m-34 operationLabels 含 2 个 stale 映射【Minor】
`tauriBridge.js:8,10`:`merge_evidence_pdfs`、`batch_overlay_pdf_text` 前端**无任何调用点**(Rust 侧仍注册),造成"已覆盖"的错觉——正是"不确定有没有全覆盖"的直观原因。建议删除或注释。

#### m-35 .pet-runs/(4MB QA 产物)未被 .gitignore 覆盖【Minor】
`.gitignore` 只忽略 `.codex-pet-runs/`,实际目录名是 `.pet-runs/`;`git add .` 会误提交。另:**test-pdf/(86MB)同样未被忽略**,建议一并处理。

#### s-17 861KB spritesheet 首次显示时才加载,可能首帧空白【Suggestion】
可选:onMounted `new Image().src = spritesheet` 预加载;或按 1152×1716 出图省 ~44% 体积。

#### s-18 中耗时命令(detect/preview_pdf_header_footer、scan_evidence_folder、inspect_*)均回落通用文案【Suggestion】
建议决策约定:只给 >2s 的操作配专属文案并写进注释,或按 §3.7 表格统一补齐。

### 3.7 命令动画覆盖三列表格(56 个注册命令)

**A. 耗时操作(>500ms 潜在)——重点核对区**

| 命令 | 有 label | 前端调用 |
|---|---|---|
| extract_frames / render_docx_template / apply_evidence_pdf_rules / build_evidence_group_pdfs / split_merged_evidence_pdf / merge_pdfs / compress_pdf / run_image_paddler | ✓ | ✓ |
| **extract_pdf_pages** | ✗→通用 | ✓ PdfToolsView:363 |
| **detect_pdf_header_footer** | ✗→通用 | ✓ useEvidencePdfDetection:65 |
| **preview_pdf_header_footer** | ✗→通用 | ✓ useEvidencePdfPreview:259 |
| **scan_evidence_folder** | ✗→通用 | ✓ EvidencePdfView:73 |
| **install_external_tool(+_from_package)**(下载数十 MB) | ✗→通用 | ✓ SettingsView:423/443 |
| inspect_docx_template / inspect_merged_evidence_pdf / inspect_docsytpl / analyze_image_paddler_folder / render_pdf_preview / probe_video / unlock_pdf(中耗时) | ✗ | ✓ |
| merge_evidence_pdfs / batch_overlay_pdf_text | ✓(**stale**) | 无调用 |

**B. 快操作(<500ms,350ms 延迟下基本不显示)——无 label 可接受**:get/set_app_settings、check_*、get_pdf_page_count、read_image_data_url、open_path、list_*、save_docx_template_to_library、get_template_history_context、模板回收站系列等 21 个。

**C. 注册但前端无调用**:check_qpdf、inspect_pdf、split_pdf、overlay_pdf_text、delete_pdf_*、get_module_registry、list_system_fonts、save_docx_template(内部调用)、get_log_file_path。

## 4. 架构评审

### 4.1 TemplateView.vue(4867 行)拆分建议

成本证据:script ~3180 行 / ~230 个函数 / 38 个响应式状态;**跨 tab 共享 ref 仅 7 个**(activeTab、templatePath、templateManifest、renderableTemplateFields、formValues、historyRuns、templateLibrary),无 >200 行函数——拆分接口窄、成本低。问题在横向数量而非纵向长度。

| 抽离物 | 类型 | 行号区间 | 规模 | 备注 |
|---|---|---|---|---|
| rules/autoStructure.js | 纯 JS 模块 | 1565-2107 | ~540 行 | 零 Vue 依赖,可直接搬并补单测(**M-09/M-10/m-22/m-23 都藏在这里**) |
| rules/manifestBuilder.js | 纯 JS 模块 | 3049-3265 | ~216 行 | 保存契约核心,最该有测试 |
| useTemplateUndo | composable | 2179-2223 | ~45 行 | |
| useHistorySuggestions | composable | 918-920、3426-3489、3861-3969 | ~230 行 | 顺带修 m-25 |
| usePreviewSelection | composable | 2645-2840 + 949-959 | ~210 行 | |
| TemplateHistoryPanel.vue | 子组件 | 模板 748-800;脚本 3287-3341 | ~180 行 | 顺带隔离 m-20 修复 |
| TemplateFillPanel.vue | 子组件 | 模板 530-746;脚本 3397-3969 | ~700 行 | |
| FieldConfirmTable.vue | 子组件 | 模板 23-418;脚本 969-1315、2842-2904 | ~800 行 | 依赖 autoStructure 模块 |
| TemplatePreviewPanel.vue | 子组件 | 模板 420-525;脚本 2225-2520 | ~650 行 | 依赖 usePreviewSelection |
| rules/partyText.js | 纯函数 | 2875-2904、3740-3841 | ~130 行 | 两侧共用 |

建议顺序:纯模块 1→2→3→4(纯搬运、立即获得可测性、直接修复 4 条发现),再组件 6→7→8→9。

### 4.2 设计文档(`template-system-design.md`)偏差表

| 设计承诺 | 实现现状 | 判定 |
|---|---|---|
| 类型表含 `select`,且"当前实现范围"声称已实现 7 种类型 | 无 select;案由以 text+autocomplete 兜底 | 代码偏离(或文档过时) |
| 类型表无 `reference` | 实现有完整 reference 类型 | 文档过时(实现超出设计) |
| 仅提"日期 run 前端可先自动合并" | autoMergeMarks 对任意相邻 run 启发式合并 | 文档过时(需回写合并规则) |
| 字段模型示例 snake_case | 全链路 camelCase(serde rename 契约一致) | 文档示例过时 |
| 历史与四类建议 | lastValues/field/semantic/association 齐全 | 一致 ✓ |
| 多模板材料包 | 未实现 | 后续范围 |

### 4.3 Rust 侧架构观察

1. **正则 XML 引擎是系统性风险源**(M-02/M-03/M-05/M-06 同根因):PARAGRAPH_RE/RUN_RE/SDT_RE/TABLE_ROW_RE 全部简单非贪婪匹配,任何嵌套结构即破。中期建议 quick-xml 流式解析;短期把"scan/wrap 统一枚举 + 复制前配对计数"作为防御。
2. `docx_template.rs` 3409 行混合 5 类职责(zip IO/正则引擎/manifest 模型/模板库 FS/多 pass 渲染管线),渲染 pass 顺序(L996-1045)与 scan/wrap 枚举一致性两个正确性前提无显式保障。建议拆 scan/wrap/render/package/library 子模块。
3. `docx_template` ⇄ `template_history` 双向依赖(渲染层调历史记录,历史层引用 manifest 类型):建议历史层自定义最小 DTO 解耦。
4. 双字体栈:printpdf 残留仅作字体度量/cmap helper,与新增 allsorts 能力重叠,建议收敛;lopdf 裸操作已散布 5 个模块,建议抽 `pdf/content_stream` 与 `pdf/font_embed` 共享模块。
5. idle-timeout 基础设施投放错位:ffmpeg/ffprobe 接入了,但 hang 风险最高的 Word/WPS/LibreOffice 转换链仍无超时 .status()(见 m-14)。
6. 数据目录 fallback 链在 `managed.rs` 与 `services/history.rs` 有两份拷贝,应提取共享。
7. 前端 composable 边界总体清晰(无模块级共享可变状态,依赖参数注入);但函数签名过宽普遍:useEvidencePdfPreview 36 入参、HeaderFooterRuleFields 19 props、MergedImport 8 个死参数——建议按职责聚合对象参数。
8. 异步守卫模式不统一:renderTruePreview 无序号(PdfJsPreview 有范式)、execute/import 守卫置位晚、detectAll 无 finally——建议在 core 抽"最新请求 wins"小工具统一承载。
9. 历史库无 schema 版本机制(PRAGMA user_version + 迁移数组),下次变更前需补齐。
10. 动画机制(事件总线 + 350ms 防闪烁 + 纯 CSS 动画)是良好设计,可作为后续"进度反馈"的承载面板。

## 5. 风险标记核验表

| # | 假设(评审前标记) | verdict | 结论摘要 |
|---|---|---|---|
| 1 | docx zip 读取无上限(zip bomb) | confirmed | 5 处无上限;Critical→Major(M-01) |
| 2 | L301 临时路径可预测 | confirmed | 竞态+TOCTOU;Major→Minor(m-05) |
| 3 | OOXML 正则手术脆弱 | confirmed | 转义正确、嵌套表格 OK;txbxContent/嵌套 sdt 击穿(M-02/M-03) |
| 4 | docsytpl 校验/zip-slip | partial | zip-slip 不成立(不落地解压);校验缺口 m-01 |
| 5 | 高亮匹配误报(w:shd/整篇黄底) | dismissed | 只认 w:val=yellow;反向发现过度清除(M-04) |
| 6 | 模板引擎测试覆盖 | partial | 34 个测试真实有效;缺畸形输入/txbxContent/重复 mark 回归 |
| 7 | output_path 无 sandbox + 孤儿文件 | partial | 路径来自保存对话框(信任模型一致);孤儿文件 m-06 |
| 8 | SQL 注入/迁移/事务 | partial | 全参数化 ✓、单事务 ✓;无 user_version(s-02);semantic 重复 m-08 |
| 9 | run_blocking JoinError 挂起 | dismissed | 正常映射 Err;建议 {:#} 保留错误链(s-01) |
| 10 | rusqlite 主线程阻塞 | dismissed | 全部在 run_blocking 内;缺 busy_timeout(m-07) |
| 11 | party_list/marker/delete_text 渲染正确性 | partial | split-run/marker/delete 有测试 ✓;party_list 两条 Major(M-05/M-06) |
| 12 | .ttc face 0 嵌错面 | partial | 现硬编码字体第 0 面均为目标面;无 face 校验,合集变动会静默嵌错 |
| 13 | lopdf xref/overlay 顺序 | dismissed | 全量重写正确;端到端测试(生成→qpdf overlay→pdftotext)基线通过 |
| 14 | TempPathGuard panic 清理 | dismissed | RAII Drop 全覆盖,较旧代码严格改进 |
| 15 | detection 空页除零/中文数字误匹配 | partial | 无 panic、误匹配风险低;CID 字号采样失效 m-11 |
| 16 | qpdf 退出码/参数注入 | partial | `--` 分隔正确、页码校验 ✓;退出码 2 回归 m-12 |
| 17 | artifacts 页码跟踪错位 | dismissed | 0-based/1-based 基准一致,M-07 是绘制互斥缺失而非页码错位 |
| 18 | WPS PowerShell kill | confirmed | M-08 |
| 19 | managed.rs HTTPS/上限/zip-slip | partial | 初验+流式上限+enclosed_name ✓;重定向降级 m-15、声明尺寸绕过 m-16 |
| 20 | system.rs 大小写/100MB 时机 | dismissed | lowercase ✓、读前检查 ✓;同步命令问题 m-19 |
| 21 | image_paddler layout "2" 兼容性 | dismissed | 前后端协调修复(预览与输出不一致的旧 bug),有测试 |
| 22 | sync→async 残留阻塞/错误契约 | dismissed | 无残留、错误文案不变;run_blocking 推广不一致(s-06) |
| 23 | ffmpeg 边界/idle 误杀 | partial | 时间校验正确;ffprobe 静默命令误杀 m-17、无效输入静默 m-18 |
| 24 | natural_cmp 溢出/对等 | dismissed | 不解析整数无溢出;实为旧实现 >u64 静默丢 key 的修复 |
| 25 | .doc/.docm picker 静默失败 | dismissed | 后端 office_oxide 转换链闭环,失败有中文提示 |
| 26 | openPath 静默统计 | confirmed | 3 处(m-21/m-31),其余 8 处正确 |
| 27 | 克隆 Range 复用/监听清理 | partial | 卸载清理 ✓、stale 有守卫;高频全量扫描 s-15 |
| 28 | defaultPath 混合分隔符 | dismissed | parentDir 取两种分隔符最大值,正确 |
| 29 | undo 栈无上限 | partial | slice(-9) 有上限;覆盖面不一致(s-13) |
| 30 | publicRules 确定性/测试缺口 | partial | 纯函数确定;漂移在调用方(m-22);5 处分支无测试 |
| 31 | tab 切换 pending 回调 | partial | 无崩溃;historyRefreshTimer 未清(s-16)、建议错位 m-25 |
| 32 | TemplateView 4867 行需拆分 | confirmed | §4.1,跨 tab 耦合窄、拆分成本低 |
| 33 | renderTruePreview 竞态 | confirmed | M-11(继承 HEAD) |
| 34 | detectAll 无 try/finally | confirmed | m-26(继承 HEAD) |
| 35 | executeMergedImportPlan 重入 | confirmed→降级 | m-28(modal 物理封锁,Major→Minor) |
| 36 | overlayRows computed 副作用 | partial | 无无限循环、当前无消费者绕读;架构隐患 s-07 |
| 37 | tauriCallSafe 忽略 {ok:false} | partial | 抽 10 处 8 正确 2 静默;logError fire-and-forget 安全 |
| 38 | HeaderFooterRuleFields 19 props | confirmed | 双向绑定无遗漏;建议聚合对象(s-09) |
| 39 | `_` 前缀死参数 | confirmed | 9 个(s-08) |
| 40 | 重构行为对等 | confirmed | 3 流程抽查无回退;测试锚定的收紧均有测试 |
| 41 | pdfUtils 测试缺口 | partial | parsePageSelection 够;range 编辑函数零测试(s-10) |
| 42 | composable 状态泄漏 | dismissed | 无模块级可变状态,字段归属不重叠 |
| 43 | 动画机制覆盖全面性 | confirmed | 100% 命令经 bridge;write_frontend_log 绕过系防递归设计 |
| 44 | 动画 start/finish 配对 | confirmed | try/finally 无泄漏 |
| 45 | 动画帧数学/spritesheet 匹配 | confirmed | 帧尺寸/百分比/周期全部整数对齐 |
| 46 | 动画 350ms 防闪烁/清理 | confirmed | 短命令无闪烁、卸载清理配对 |
| 47 | 动画 label 覆盖全面性 | partial | 6 个耗时命令通用文案 + 2 stale(m-32/m-33/m-34) |
| 48 | 资产/gitignore 卫生 | partial | spritesheet 打包合理;.pet-runs/test-pdf 未忽略(m-35) |

## 6. 附录

### 6.1 排除项说明
Prettier 纯格式化 diff、命名/注释/CSS 风格、已有测试覆盖且通过的行为(除非测试本身缺陷)、第三方 crate 内部实现、已提交历史代码(只评 diff 触及行与新增文件;继承自 HEAD 的 2 条 Major/2 条 Minor 已明确标注"继承")。

### 6.2 Suggestion 汇总(18 条)

| # | 位置 | 内容 |
|---|---|---|
| s-01 | commands/mod.rs:13-16 | run_blocking 用 `format!("{e:#}")` 保留 anyhow 错误链 |
| s-02 | template_history.rs:316-353 | 引入 PRAGMA user_version + 迁移数组 |
| s-03 | docx_template.rs(全文) | 3409 行拆 scan/wrap/render/package/library 子模块,注释固定渲染 pass 顺序 |
| s-04 | header_footer.rs:1267-1278 | encode_subset_glyph_text whitespace/else 死分支合并 |
| s-05 | header_footer.rs:733,387 | skip_*_pages 死参数——修复 M-07 时接线,否则删除 |
| s-06 | commands/image_paddler.rs、video.rs | 4 处内联 spawn_blocking 改用 run_blocking;get_pdf_page_count 等同步命令异步化 |
| s-07 | EvidencePdfWorkbench.vue:833 | overlayRows computed 纯化(assignPageRanges 副本),变更点显式 updatePageRanges |
| s-08 | useEvidencePdfMergedImport/ExistingEditing | 删 9 个 `_` 死参数 |
| s-09 | HeaderFooterRuleFields.vue | 19 props 聚合为 header/footer 两对象 |
| s-10 | core/pdfUtils.js | range 编辑函数补单测;removeRangeAt 加 index<0 防御 |
| s-11 | useEvidencePdfDetection.js:147-209 | 页码/页脚候选打分启发式(全新逻辑)补参数化单测——**最大测试缺口** |
| s-12 | useEvidencePdfMergedImport.js:448 | hasSplitWarning 子串匹配改边界正则或后端结构化引用 |
| s-13 | TemplateView.vue:2210 | undo 覆盖不一致:onRowTypeChange/onFieldNameInput 入栈,或改按钮文案 |
| s-14 | TemplateView.vue:1209-1222 | 字段表格 O(n²)~O(n³) 渲染热点改 computed Map 预计算 |
| s-15 | TemplateView.vue:949,2645 | selectionchange 加 ~100ms 节流/先判容器再全扫 |
| s-16 | TemplateView.vue:3484 | historyRefreshTimer 在 onUnmounted 清除 |
| s-17 | DocletWorkingPet.vue | spritesheet 预加载/按渲染尺寸出图 |
| s-18 | tauriBridge.js | 明确"只给 >2s 操作配专属文案"约定并写注释 |
| — | useEvidencePdfSession.js:191 | assignPageRanges 非变异化后 `file.outputPath = outputPath` 死写,删除 |
| — | external/word.rs:90-101 | where 多行输出取首行 + exists 校验 |
| — | EvidencePdfView.vue | conversionFailures 在 buildEvidence 入口不清空,残留上次失败列表 |

---

*报告完。评审过程未修改任何源代码文件;全部 Major 发现经独立核验 agent 二次读码确认。*
