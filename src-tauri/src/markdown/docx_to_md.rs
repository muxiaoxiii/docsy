//! docx → Markdown：docx-rs reader 读取，遍历文档树生成 GFM。

use anyhow::{Context, Result};
use docx_rs::{
    Docx, DocumentChild, DrawingData, Hyperlink, HyperlinkData, Paragraph, ParagraphChild, Run,
    RunChild, Table, TableCellContent, TableChild, TableRowChild, TextBoxContentChild,
};
use std::collections::HashMap;
use std::io::Read as _;
use std::path::Path;

/// 一张已导出的图片：Markdown 引用所需的名称与相对路径。
pub struct ExportedImage {
    /// 图片名（不含扩展名），同时用作 alt 文本，如 `image1`。
    pub name: String,
    /// 相对 md 文件的引用路径，如 `合同_assets/image1.png`。
    pub rel_path: String,
}

/// rid → 导出图片的引用信息。
type ExportedImages = HashMap<String, ExportedImage>;

/// 行内格式标记，用于相邻 Run 合并。
#[derive(Default, Clone, Copy, PartialEq, Eq)]
struct Fmt {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    /// 下划线：GFM 无原生语法，用内嵌 HTML `<u>` 表达
    underline: bool,
}

/// 转换上下文：样式 / 超链接 / 图片 / 编号定义的只读引用 + 列表计数器。
struct Ctx<'a> {
    heading_map: HashMap<String, usize>,
    hyperlinks: HashMap<&'a str, &'a str>,
    images: HashMap<&'a str, &'a str>,
    /// 已导出到 assets 目录的图片（rid → 引用信息）
    exported: ExportedImages,
    /// numId → abstractNumId
    num_to_abstract: HashMap<usize, usize>,
    /// abstractNumId → 每级编号格式（bullet / decimal / ...）
    abstract_levels: HashMap<usize, Vec<(usize, String)>>,
    /// 有序列表计数器：(numId, level) → 当前序号
    list_counters: HashMap<(usize, usize), usize>,
    /// 上一个段落所属的列表 numId，用于检测列表中断后重置计数
    last_list_num: Option<usize>,
}

impl<'a> Ctx<'a> {
    fn new(docx: &'a Docx) -> Self {
        let heading_map = docx.styles.create_heading_style_map();
        let hyperlinks = docx
            .hyperlinks
            .iter()
            .map(|(rid, target, _)| (rid.as_str(), target.as_str()))
            .collect();
        let images = docx
            .images
            .iter()
            .map(|(rid, path, _, _)| (rid.as_str(), path.as_str()))
            .collect();
        let num_to_abstract = docx
            .numberings
            .numberings
            .iter()
            .map(|n| (n.id, n.abstract_num_id))
            .collect();
        let abstract_levels = docx
            .numberings
            .abstract_nums
            .iter()
            .map(|a| {
                (
                    a.id,
                    a.levels
                        .iter()
                        .map(|l| (l.level, l.format.val.clone()))
                        .collect(),
                )
            })
            .collect();
        Self {
            heading_map,
            hyperlinks,
            images,
            exported: HashMap::new(),
            num_to_abstract,
            abstract_levels,
            list_counters: HashMap::new(),
            last_list_num: None,
        }
    }

    fn with_exported(mut self, exported: ExportedImages) -> Self {
        self.exported = exported;
        self
    }

    /// 段落样式 → 标题级别（1-6）。
    fn heading_level(&self, p: &Paragraph) -> Option<usize> {
        let style = p.property.style.as_ref()?;
        if let Some(level) = self.heading_map.get(&style.val) {
            return Some(*level);
        }
        // 兜底：直接解析 "Heading1" ~ "Heading6" 样式 id
        style
            .val
            .strip_prefix("Heading")
            .and_then(|n| n.parse::<usize>().ok())
            .filter(|n| (1..=6).contains(n))
    }

    /// 列表段落的编号格式。
    fn list_format(&self, num_id: usize, level: usize) -> String {
        self.num_to_abstract
            .get(&num_id)
            .and_then(|ab| self.abstract_levels.get(ab))
            .and_then(|levels| {
                levels
                    .iter()
                    .find(|(l, _)| *l == level)
                    .map(|(_, f)| f.clone())
            })
            .unwrap_or_else(|| "bullet".to_string())
    }

    /// 取有序列表下一序号；列表中断或层级回退时重置计数。
    fn next_ordered_index(&mut self, num_id: usize, level: usize) -> usize {
        if self.last_list_num != Some(num_id) {
            self.list_counters.retain(|(id, _), _| *id != num_id);
        }
        // 回退到更浅层级时，清掉更深层的计数
        self.list_counters
            .retain(|(id, l), _| *id != num_id || *l <= level);
        let counter = self.list_counters.entry((num_id, level)).or_insert(0);
        *counter += 1;
        *counter
    }
}

/// Bold / Italic 等元素可能带 val="false"（样式覆盖），序列化为 bool 读取。
fn flag_enabled<T: serde::Serialize>(flag: &Option<T>) -> bool {
    match flag {
        Some(f) => serde_json::to_value(f)
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        None => false,
    }
}

/// 等宽字体识别：行内代码还原为反引号。
/// RunFonts 字段为私有，走 serde 序列化取 ascii 字体名。
fn is_mono_font(run: &Run) -> bool {
    let Some(fonts) = &run.run_property.fonts else {
        return false;
    };
    let name = serde_json::to_value(fonts)
        .ok()
        .and_then(|v| v.get("ascii")?.as_str().map(str::to_lowercase))
        .unwrap_or_default();
    name.contains("courier") || name.contains("consol") || name.contains("mono")
}

fn run_fmt(run: &Run) -> Fmt {
    Fmt {
        bold: flag_enabled(&run.run_property.bold),
        italic: flag_enabled(&run.run_property.italic),
        strike: flag_enabled(&run.run_property.strike) || flag_enabled(&run.run_property.dstrike),
        code: is_mono_font(run),
        underline: underline_enabled(run),
    }
}

/// 下划线识别：Underline 序列化为线型字符串，val="none" 视为关闭。
fn underline_enabled(run: &Run) -> bool {
    run.run_property.underline.as_ref().is_some_and(|u| {
        serde_json::to_value(u)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .is_some_and(|v| v != "none")
    })
}

/// 普通文本的 Markdown 转义（代码段内不转义）。
fn escape_md(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' | '*' | '_' | '`' | '[' | ']' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// 按格式包裹文本。
fn wrap_fmt(fmt: Fmt, text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    if fmt.code {
        // 内容含反引号时用双反引号包裹
        return if text.contains('`') {
            format!("`` {text} ``")
        } else {
            format!("`{text}`")
        };
    }
    let mut out = escape_md(text);
    if fmt.bold && fmt.italic {
        out = format!("***{out}***");
    } else if fmt.bold {
        out = format!("**{out}**");
    } else if fmt.italic {
        out = format!("*{out}*");
    }
    if fmt.strike {
        out = format!("~~{out}~~");
    }
    if fmt.underline {
        // GFM 无下划线语法，按惯例用内嵌 HTML
        out = format!("<u>{out}</u>");
    }
    out
}

/// 行内容渲染器：合并相邻同格式 Run，避免 `**a****b**`。
struct InlineRenderer<'a, 'b> {
    ctx: &'a mut Ctx<'b>,
    out: String,
    cur_fmt: Fmt,
    cur_text: String,
    /// 超链接内部：忽略下划线（链接本身已是可点击样式，不输出 <u>）
    in_link: bool,
}

impl<'a, 'b> InlineRenderer<'a, 'b> {
    fn new(ctx: &'a mut Ctx<'b>) -> Self {
        Self {
            ctx,
            out: String::new(),
            cur_fmt: Fmt::default(),
            cur_text: String::new(),
            in_link: false,
        }
    }

    fn flush_segment(&mut self) {
        if !self.cur_text.is_empty() {
            self.out
                .push_str(&wrap_fmt(self.cur_fmt, &std::mem::take(&mut self.cur_text)));
        }
    }

    fn push_text(&mut self, fmt: Fmt, text: &str) {
        if fmt == self.cur_fmt {
            self.cur_text.push_str(text);
        } else {
            self.flush_segment();
            self.cur_fmt = fmt;
            self.cur_text.push_str(text);
        }
    }

    /// 遇到链接 / 图片等块级行内元素时先冲刷当前段。
    fn push_raw(&mut self, raw: String) {
        self.flush_segment();
        self.out.push_str(&raw);
    }

    fn render_children(&mut self, children: &[ParagraphChild]) {
        for child in children {
            match child {
                ParagraphChild::Run(run) => self.render_run(run),
                ParagraphChild::Hyperlink(link) => {
                    let raw = self.render_hyperlink(link);
                    self.push_raw(raw);
                }
                ParagraphChild::Insert(ins) => {
                    // 修订插入内容按普通文本处理
                    for c in &ins.children {
                        if let docx_rs::InsertChild::Run(run) = c {
                            self.render_run(run);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn render_run(&mut self, run: &Run) {
        let mut fmt = run_fmt(run);
        if self.in_link {
            fmt.underline = false;
        }
        for child in &run.children {
            match child {
                RunChild::Text(t) => self.push_text(fmt, &t.text),
                RunChild::DeleteText(_) => {}
                RunChild::Tab(_) => self.push_text(Fmt::default(), "    "),
                RunChild::Break(_) => {
                    // Markdown 硬换行：行尾两个空格
                    self.push_raw("  \n".to_string());
                }
                RunChild::Drawing(drawing) => {
                    match &drawing.data {
                        Some(DrawingData::Pic(pic)) => {
                            // 优先引用已导出的 assets 文件，否则退化为包内路径/占位
                            let raw = if let Some(img) = self.ctx.exported.get(pic.id.as_str()) {
                                format!("![{}]({})", img.name, img.rel_path)
                            } else {
                                match self.ctx.images.get(pic.id.as_str()) {
                                    Some(path) => format!("![image]({path})"),
                                    None => "![image]".to_string(),
                                }
                            };
                            self.push_raw(raw);
                        }
                        Some(DrawingData::TextBox(text_box)) => {
                            // 文本框/形状内文本（txbxContent）：按普通段落输出，
                            // 多段之间以空格衔接，内嵌表格渲染为 GFM 表格
                            for child in &text_box.children {
                                match child {
                                    TextBoxContentChild::Paragraph(p) => {
                                        self.render_children(&p.children);
                                        self.push_text(Fmt::default(), " ");
                                    }
                                    TextBoxContentChild::Table(t) => {
                                        let md = render_table(t, self.ctx);
                                        if !md.is_empty() {
                                            self.push_raw(md);
                                        }
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn render_hyperlink(&mut self, link: &Hyperlink) -> String {
        // 链接文本不继承外部格式状态，独立渲染；内部忽略下划线
        let mut inner = InlineRenderer::new(&mut *self.ctx);
        inner.in_link = true;
        inner.render_children(&link.children);
        inner.flush_segment();
        let text = inner.out;
        let url = match &link.link {
            HyperlinkData::External { rid, .. } => {
                self.ctx.hyperlinks.get(rid.as_str()).copied().unwrap_or("")
            }
            HyperlinkData::Anchor { anchor } => {
                // 文档内锚点没有外部 URL，输出纯文本
                let _ = anchor;
                ""
            }
        };
        if url.is_empty() {
            text
        } else {
            format!("[{text}]({url})")
        }
    }

    fn finish(mut self) -> String {
        self.flush_segment();
        self.out
    }
}

/// 水平线识别：无文本但带下边框的段落（md → docx 时由我们生成）。
fn has_bottom_border(p: &Paragraph) -> bool {
    p.property.borders.as_ref().is_some_and(|b| {
        serde_json::to_value(b)
            .ok()
            .and_then(|v| v.get("bottom").cloned())
            .is_some_and(|v| !v.is_null())
    })
}

fn render_paragraph(p: &Paragraph, ctx: &mut Ctx) -> Option<String> {
    let mut renderer = InlineRenderer::new(ctx);
    renderer.render_children(&p.children);
    let text = renderer.finish();
    if text.trim().is_empty() {
        // 带下边框的空段落还原为水平线
        if has_bottom_border(p) {
            ctx.last_list_num = None;
            return Some("---".to_string());
        }
        return None;
    }

    // 标题
    if let Some(level) = ctx.heading_level(p) {
        ctx.last_list_num = None;
        return Some(format!("{} {}", "#".repeat(level), text.trim()));
    }

    // 列表
    if let Some(num_pr) = &p.property.numbering_property {
        if let Some(num_id) = num_pr.id.as_ref().map(|id| id.id) {
            let level = num_pr.level.as_ref().map(|l| l.val).unwrap_or(0);
            let format = ctx.list_format(num_id, level);
            let marker = if format == "decimal" {
                format!("{}.", ctx.next_ordered_index(num_id, level))
            } else {
                ctx.last_list_num = Some(num_id);
                "-".to_string()
            };
            ctx.last_list_num = Some(num_id);
            let indent = "  ".repeat(level);
            return Some(format!("{indent}{marker} {}", text.trim()));
        }
    }

    ctx.last_list_num = None;
    Some(text.trim_end().to_string())
}

/// 单元格文本：多段合并为一行，换行转空格，竖线转义。
fn render_cell(cell: &docx_rs::TableCell, ctx: &mut Ctx) -> String {
    let mut parts = Vec::new();
    for content in &cell.children {
        match content {
            TableCellContent::Paragraph(p) => {
                let mut renderer = InlineRenderer::new(ctx);
                renderer.render_children(&p.children);
                let text = renderer.finish();
                let text = text.trim();
                if !text.is_empty() {
                    parts.push(text.replace('\n', " "));
                }
            }
            TableCellContent::Table(t) => {
                parts.push(render_table(t, ctx).replace('\n', " "));
            }
            _ => {}
        }
    }
    parts.join(" ").replace('|', "\\|")
}

/// 表头单元格若整体被一对 `**` 包裹则去掉（GFM 表头天然加粗，无需重复标记）。
fn strip_full_bold(s: &str) -> String {
    if s.len() > 4 && s.starts_with("**") && s.ends_with("**") && !s[2..s.len() - 2].contains("**")
    {
        return s[2..s.len() - 2].to_string();
    }
    s.to_string()
}

/// 单元格合并属性：(gridSpan 列数, vMerge 类型)。
/// TableCellProperty 字段私有，走 serde 序列化读取（camelCase）。
fn cell_merge(cell: &docx_rs::TableCell) -> (usize, Option<String>) {
    let v = serde_json::to_value(&cell.property).unwrap_or_default();
    let span = v
        .get("gridSpan")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1)
        .max(1) as usize;
    let vmerge = v
        .get("verticalMerge")
        .and_then(|m| m.as_str())
        .map(str::to_string);
    (span, vmerge)
}

fn render_table(table: &Table, ctx: &mut Ctx) -> String {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for child in &table.rows {
        let TableChild::TableRow(row) = child;
        let mut cells = Vec::new();
        for c in &row.cells {
            let TableRowChild::TableCell(cell) = c;
            let (span, vmerge) = cell_merge(cell);
            // 纵向合并续格（vMerge=continue）：GFM 表格没有跨行概念，续格留空
            let text = if vmerge.as_deref() == Some("continue") {
                String::new()
            } else {
                render_cell(cell, ctx)
            };
            cells.push(text);
            // 横向合并（gridSpan）：内容只写首格，跨出的列补空格
            for _ in 1..span {
                cells.push(String::new());
            }
        }
        rows.push(cells);
    }
    if rows.is_empty() {
        return String::new();
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut lines = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let mut cells: Vec<String> = row.clone();
        cells.resize(cols, String::new());
        if i == 0 {
            // 首行是表头
            cells = cells.iter().map(|c| strip_full_bold(c)).collect();
        }
        lines.push(format!("| {} |", cells.join(" | ")));
        if i == 0 {
            lines.push(format!("|{}|", vec![" --- "; cols].join("|")));
        }
    }
    lines.join("\n")
}

/// 代码块段落识别：md → docx 时代码行带 F5F5F5 底纹。
fn is_code_paragraph(p: &Paragraph) -> bool {
    p.property.shading.as_ref().is_some_and(|s| {
        serde_json::to_value(s)
            .ok()
            .and_then(|v| v.get("fill")?.as_str().map(str::to_uppercase))
            .is_some_and(|f| f == "F5F5F5")
    })
}

/// 提取段落纯文本（不带任何格式标记），用于代码块。
fn plain_text(p: &Paragraph) -> String {
    let mut out = String::new();
    for child in &p.children {
        if let ParagraphChild::Run(run) = child {
            for rc in &run.children {
                match rc {
                    RunChild::Text(t) => out.push_str(&t.text),
                    RunChild::Tab(_) => out.push_str("    "),
                    RunChild::Break(_) => out.push('\n'),
                    _ => {}
                }
            }
        }
    }
    out
}

/// docx 字节 → Markdown 字符串（不导出图片，仅测试与纯文本场景使用）。
#[cfg(test)]
pub fn docx_bytes_to_md(bytes: &[u8]) -> Result<String> {
    let docx = docx_rs::read_docx(bytes).context("无法解析 docx 文件")?;
    Ok(render_document(&docx, HashMap::new()))
}

/// 渲染整篇文档为 Markdown 字符串。
fn render_document(docx: &Docx, exported: ExportedImages) -> String {
    let mut ctx = Ctx::new(docx).with_exported(exported);
    let mut blocks: Vec<String> = Vec::new();
    let children = &docx.document.children;
    let mut i = 0;
    while i < children.len() {
        match &children[i] {
            // 连续的代码段落合并为一个围栏代码块
            DocumentChild::Paragraph(p) if is_code_paragraph(p) => {
                let mut lines = vec![plain_text(p)];
                i += 1;
                while i < children.len() {
                    match &children[i] {
                        DocumentChild::Paragraph(p) if is_code_paragraph(p) => {
                            lines.push(plain_text(p));
                            i += 1;
                        }
                        _ => break,
                    }
                }
                ctx.last_list_num = None;
                blocks.push(format!("```\n{}\n```", lines.join("\n")));
            }
            DocumentChild::Paragraph(p) => {
                if let Some(block) = render_paragraph(p, &mut ctx) {
                    blocks.push(block);
                }
                i += 1;
            }
            DocumentChild::Table(t) => {
                let table_md = render_table(t, &mut ctx);
                if !table_md.is_empty() {
                    blocks.push(table_md);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    let mut out = blocks.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// 按文档出现顺序收集图片 rid（去重，同一 rid 多处引用只记录首次）。
fn collect_image_rids(docx: &Docx) -> Vec<String> {
    let mut out = Vec::new();
    for child in &docx.document.children {
        match child {
            DocumentChild::Paragraph(p) => collect_rids_from_children(&p.children, &mut out),
            DocumentChild::Table(t) => collect_rids_from_table(t, &mut out),
            _ => {}
        }
    }
    out
}

fn collect_rids_from_table(table: &Table, out: &mut Vec<String>) {
    for child in &table.rows {
        let TableChild::TableRow(row) = child;
        for cell_child in &row.cells {
            let TableRowChild::TableCell(cell) = cell_child;
            for content in &cell.children {
                match content {
                    TableCellContent::Paragraph(p) => {
                        collect_rids_from_children(&p.children, out)
                    }
                    TableCellContent::Table(t) => collect_rids_from_table(t, out),
                    _ => {}
                }
            }
        }
    }
}

fn collect_rids_from_children(children: &[ParagraphChild], out: &mut Vec<String>) {
    for child in children {
        match child {
            ParagraphChild::Run(run) => collect_rids_from_run(run, out),
            ParagraphChild::Hyperlink(link) => collect_rids_from_children(&link.children, out),
            ParagraphChild::Insert(ins) => {
                for c in &ins.children {
                    if let docx_rs::InsertChild::Run(run) = c {
                        collect_rids_from_run(run, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_rids_from_run(run: &Run, out: &mut Vec<String>) {
    for child in &run.children {
        if let RunChild::Drawing(d) = child {
            if let Some(DrawingData::Pic(pic)) = &d.data {
                if !pic.id.is_empty() && !out.contains(&pic.id) {
                    out.push(pic.id.clone());
                }
            }
        }
    }
}

/// 解析 document.xml.rels，返回图片 rid → 包内 target（如 `media/image1.png`）。
fn parse_image_rels(xml: &str) -> HashMap<String, String> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut map = HashMap::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(e)) | Ok(quick_xml::events::Event::Start(e)) => {
                if e.name().as_ref() == b"Relationship" {
                    let mut id = None;
                    let mut target = None;
                    let mut is_image = false;
                    for attr in e.attributes().flatten() {
                        let value = attr
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap_or_default();
                        match attr.key.as_ref() {
                            b"Id" => id = Some(value.to_string()),
                            b"Target" => target = Some(value.to_string()),
                            b"Type" => is_image = value.ends_with("/image"),
                            _ => {}
                        }
                    }
                    if is_image {
                        if let (Some(id), Some(target)) = (id, target) {
                            map.insert(id, target);
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

/// rels target → zip 内路径：`media/x.png` → `word/media/x.png`；`/word/...` 原样。
fn media_zip_path(target: &str) -> String {
    let t = target.trim_start_matches('/');
    if t.starts_with("word/") {
        t.to_string()
    } else {
        format!("word/{t}")
    }
}

/// 转换失败时清理由我们创建且仍为空的 assets 目录。
fn cleanup_empty_assets_dir(dir: &Path, created_by_us: bool) {
    if created_by_us && dir.read_dir().is_ok_and(|mut d| d.next().is_none()) {
        let _ = std::fs::remove_dir(dir);
    }
}

/// 把文档内嵌图片导出到 `<输出目录>/<stem>_assets/`，返回 rid → 引用信息。
///
/// 图片字节直接从 zip 包读取（docx-rs reader 在无 image feature 时只保留
/// PNG 签名图片，不可靠）；rid 顺序按文档中首次出现排序，同一 rid 复用
/// 只导出一份。没有任何可导出图片时不创建 assets 目录。
fn export_images(bytes: &[u8], docx: &Docx, output: &Path) -> Result<ExportedImages> {
    let rids = collect_image_rids(docx);
    if rids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("读取 docx 包失败")?;
    let rels_xml = match archive.by_name("word/_rels/document.xml.rels") {
        Ok(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s).context("读取 document.xml.rels 失败")?;
            s
        }
        // 没有 rels 就没有可导出的图片
        Err(_) => return Ok(HashMap::new()),
    };
    let rels = parse_image_rels(&rels_xml);

    // 先解析出所有可导出的图片（rid、扩展名、字节），避免先建目录才发现没图
    let mut plan: Vec<(String, String, Vec<u8>)> = Vec::new();
    for rid in rids {
        let Some(target) = rels.get(&rid) else { continue };
        let zip_path = media_zip_path(target);
        let Ok(mut entry) = archive.by_name(&zip_path) else {
            continue;
        };
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data)
            .with_context(|| format!("读取图片失败: {zip_path}"))?;
        // 扩展名取包内条目的原始格式
        let ext = Path::new(target)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "png".to_string());
        plan.push((rid, ext, data));
    }
    if plan.is_empty() {
        return Ok(HashMap::new());
    }

    let stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .context("无法解析输出文件名")?;
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let assets_dir = parent.join(format!("{stem}_assets"));
    // 目录已存在时不清空，覆盖同名文件即可（编号是确定性的）
    let created_dir = !assets_dir.exists();
    std::fs::create_dir_all(&assets_dir)
        .with_context(|| format!("创建图片目录失败: {}", assets_dir.display()))?;

    let mut exported = HashMap::new();
    for (i, (rid, ext, data)) in plan.iter().enumerate() {
        let name = format!("image{}", i + 1);
        let file_path = assets_dir.join(format!("{name}.{ext}"));
        if let Err(err) = std::fs::write(&file_path, data)
            .with_context(|| format!("导出图片失败: {}", file_path.display()))
        {
            cleanup_empty_assets_dir(&assets_dir, created_dir);
            return Err(err);
        }
        exported.insert(
            rid.clone(),
            ExportedImage {
                rel_path: format!("{stem}_assets/{name}.{ext}"),
                name,
            },
        );
    }
    Ok(exported)
}

pub fn convert(input: &Path, output: &Path) -> Result<()> {
    let bytes = std::fs::read(input)
        .with_context(|| format!("无法读取 docx 文件: {}", input.display()))?;
    let docx = docx_rs::read_docx(&bytes).context("无法解析 docx 文件")?;
    let exported = export_images(&bytes, &docx, output)?;
    let md = render_document(&docx, exported);
    std::fs::write(output, md)
        .with_context(|| format!("无法写入 Markdown 文件: {}", output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{
        AbstractNumbering, Docx, IndentLevel, Level, LevelJc, LevelText, NumberFormat, Numbering,
        NumberingId, Paragraph, Run, RunFonts, Start, Style, StyleType, TableCell, TableRow,
    };
    use std::io::Cursor;

    fn pack(docx: Docx) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        docx.build().pack(&mut buf).expect("打包 docx 失败");
        buf.into_inner()
    }

    fn para(text: &str) -> Paragraph {
        Paragraph::new().add_run(Run::new().add_text(text))
    }

    #[test]
    fn extracts_heading_and_inline_marks() {
        let docx = Docx::new()
            .add_style(
                Style::new("Heading1", StyleType::Paragraph)
                    .name("heading 1")
                    .bold(),
            )
            .add_style(
                Style::new("Heading2", StyleType::Paragraph)
                    .name("heading 2")
                    .bold(),
            )
            .add_paragraph(
                Paragraph::new()
                    .style("Heading1")
                    .add_run(Run::new().add_text("证据目录")),
            )
            .add_paragraph(
                Paragraph::new()
                    .style("Heading2")
                    .add_run(Run::new().add_text("第二节")),
            )
            // 相邻同格式 run 应合并输出，而不是 **a****b**
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("普通"))
                    .add_run(Run::new().add_text("加粗").bold())
                    .add_run(Run::new().add_text("再加粗").bold()),
            )
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("斜体").italic())
                    .add_run(Run::new().add_text("删除").strike())
                    .add_run(
                        Run::new()
                            .add_text("代码")
                            .fonts(RunFonts::new().ascii("Consolas")),
                    ),
            )
            // 全段加粗但不是标题样式：保持原样输出，不猜标题
            .add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("整段加粗").bold()),
            );
        let md = docx_bytes_to_md(&pack(docx)).unwrap();
        assert!(md.contains("# 证据目录"), "md:\n{md}");
        assert!(md.contains("## 第二节"), "md:\n{md}");
        assert!(md.contains("普通**加粗再加粗**"), "md:\n{md}");
        assert!(!md.contains("****"), "md:\n{md}");
        assert!(md.contains("*斜体*"), "md:\n{md}");
        assert!(md.contains("~~删除~~"), "md:\n{md}");
        assert!(md.contains("`代码`"), "md:\n{md}");
        assert!(md.contains("**整段加粗**"), "md:\n{md}");
        assert!(!md.contains("# **整段加粗**"), "md:\n{md}");
    }

    #[test]
    fn extracts_lists_with_counters() {
        let bullet = AbstractNumbering::new(1).add_level(Level::new(
            0,
            Start::new(1),
            NumberFormat::new("bullet"),
            LevelText::new("•"),
            LevelJc::new("left"),
        ));
        let ordered = AbstractNumbering::new(2).add_level(Level::new(
            0,
            Start::new(1),
            NumberFormat::new("decimal"),
            LevelText::new("%1."),
            LevelJc::new("left"),
        ));
        let docx = Docx::new()
            .add_abstract_numbering(bullet)
            .add_numbering(Numbering::new(1, 1))
            .add_abstract_numbering(ordered)
            .add_numbering(Numbering::new(2, 2))
            .add_paragraph(
                para("无序一").numbering(NumberingId::new(1), IndentLevel::new(0)),
            )
            .add_paragraph(
                para("无序二").numbering(NumberingId::new(1), IndentLevel::new(0)),
            )
            .add_paragraph(para("中间普通段落"))
            .add_paragraph(
                para("有序一").numbering(NumberingId::new(2), IndentLevel::new(0)),
            )
            .add_paragraph(
                para("有序二").numbering(NumberingId::new(2), IndentLevel::new(0)),
            );
        let md = docx_bytes_to_md(&pack(docx)).unwrap();
        assert!(md.contains("- 无序一"), "md:\n{md}");
        assert!(md.contains("- 无序二"), "md:\n{md}");
        assert!(md.contains("1. 有序一"), "md:\n{md}");
        assert!(md.contains("2. 有序二"), "md:\n{md}");
    }

    #[test]
    fn extracts_table_and_merges_cell_lines() {
        let header = TableRow::new(vec![
            TableCell::new().add_paragraph(para("列一")),
            TableCell::new().add_paragraph(para("列二")),
        ]);
        // 单元格内多个段落合并为一行
        let body = TableRow::new(vec![
            TableCell::new()
                .add_paragraph(para("第一行"))
                .add_paragraph(para("第二行")),
            TableCell::new().add_paragraph(para("含|竖线")),
        ]);
        let docx = Docx::new().add_table(Table::new(vec![header, body]));
        let md = docx_bytes_to_md(&pack(docx)).unwrap();
        assert!(md.contains("| 列一 | 列二 |"), "md:\n{md}");
        assert!(md.contains("| --- | --- |"), "md:\n{md}");
        assert!(md.contains("| 第一行 第二行 | 含\\|竖线 |"), "md:\n{md}");
    }

    #[test]
    fn extracts_underline_as_inline_html() {
        let docx = Docx::new()
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("普通"))
                    .add_run(Run::new().add_text("下划线").underline("single"))
                    .add_run(Run::new().add_text("再加下划线").underline("single")),
            )
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text("粗下划").bold().underline("single")),
            );
        let md = docx_bytes_to_md(&pack(docx)).unwrap();
        // 相邻同格式 run 合并为一个 <u> 段
        assert!(md.contains("普通<u>下划线再加下划线</u>"), "md:\n{md}");
        assert!(md.contains("<u>**粗下划**</u>"), "md:\n{md}");
    }

    #[test]
    fn extracts_merged_cells() {
        use docx_rs::VMergeType;
        // 第一行：跨两列的合并单元格 + 普通格
        let row1 = TableRow::new(vec![
            TableCell::new()
                .add_paragraph(para("横向合并"))
                .grid_span(2),
            TableCell::new().add_paragraph(para("列三")),
        ]);
        // 第二行：纵向合并起点 + 普通格
        let row2 = TableRow::new(vec![
            TableCell::new()
                .add_paragraph(para("纵向起点"))
                .vertical_merge(VMergeType::Restart),
            TableCell::new().add_paragraph(para("占位")),
            TableCell::new().add_paragraph(para("数据一")),
        ]);
        // 第三行：纵向合并续格，内容应留空
        let row3 = TableRow::new(vec![
            TableCell::new()
                .add_paragraph(para("不应出现"))
                .vertical_merge(VMergeType::Continue),
            TableCell::new().add_paragraph(para("占位")),
            TableCell::new().add_paragraph(para("数据二")),
        ]);
        let docx = Docx::new().add_table(Table::new(vec![row1, row2, row3]));
        let md = docx_bytes_to_md(&pack(docx)).unwrap();
        // gridSpan=2 → 首格写内容，跨出的列补空
        assert!(md.contains("| 横向合并 |  | 列三 |"), "md:\n{md}");
        // vMerge=continue 的格子内容留空
        assert!(md.contains("|  | 占位 | 数据二 |"), "md:\n{md}");
        assert!(!md.contains("不应出现"), "md:\n{md}");
        assert!(md.contains("| 纵向起点 | 占位 | 数据一 |"), "md:\n{md}");
    }

    #[test]
    fn extracts_textbox_content_as_paragraph_text() {
        // docx-rs 写端不支持 TextBox 序列化（unimplemented），
        // 直接在内存里构造 Paragraph 走 render_paragraph 验证渲染逻辑
        let mut text_box = docx_rs::TextBox::new();
        text_box
            .children
            .push(TextBoxContentChild::Paragraph(Box::new(para("框内文字"))));
        text_box
            .children
            .push(TextBoxContentChild::Paragraph(Box::new(
                Paragraph::new().add_run(Run::new().add_text("第二段").bold()),
            )));
        let mut run = Run::new();
        run.children
            .push(RunChild::Drawing(Box::new(docx_rs::Drawing::new().text_box(text_box))));
        let p = Paragraph::new().add_run(run);
        let docx = Docx::new();
        let mut ctx = Ctx::new(&docx);
        let md = render_paragraph(&p, &mut ctx).unwrap();
        assert!(md.contains("框内文字"), "md:\n{md}");
        assert!(md.contains("**第二段**"), "md:\n{md}");
    }

    #[test]
    fn rejects_invalid_bytes() {
        assert!(docx_bytes_to_md(b"not a docx").is_err());
    }
}

#[cfg(test)]
mod image_export_tests {
    use super::*;
    use docx_rs::{Docx, Paragraph, Pic, Run};

    /// 生成不同内容的 PNG 字节（不同尺寸保证字节不同，
    /// 避免 docx-rs 写端按内容去重把两张图合并）。
    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = image::DynamicImage::new_rgba8(w, h);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    fn pack_docx(docx: Docx) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        docx.build().pack(&mut buf).expect("打包 docx 失败");
        buf.into_inner()
    }

    #[test]
    fn exports_images_with_dedup_and_relative_refs() {
        let dir = std::env::temp_dir().join(format!("docsy-img-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("证据.docx");
        let output = dir.join("证据.md");

        // 两张不同的图，第一张在文档中引用两次
        let pic_a = Pic::new_with_dimensions(png_bytes(1, 1), 1, 1);
        let pic_b = Pic::new_with_dimensions(png_bytes(2, 1), 2, 1);
        let docx = Docx::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic_a.clone())))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic_b)))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic_a)));
        std::fs::write(&input, pack_docx(docx)).unwrap();

        convert(&input, &output).unwrap();

        // assets 目录有且仅有 2 个文件（重复引用只导出一份）
        let assets = dir.join("证据_assets");
        assert!(assets.is_dir(), "应创建 assets 目录");
        let mut entries: Vec<String> = std::fs::read_dir(&assets)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        entries.sort();
        assert_eq!(entries, vec!["image1.png", "image2.png"]);

        // md 引用相对路径，重复引用指向同一文件
        let md = std::fs::read_to_string(&output).unwrap();
        let ref1 = "![image1](证据_assets/image1.png)";
        let ref2 = "![image2](证据_assets/image2.png)";
        assert_eq!(md.matches(ref1).count(), 2, "md:\n{md}");
        assert_eq!(md.matches(ref2).count(), 1, "md:\n{md}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_assets_dir_when_document_has_no_images() {
        let dir = std::env::temp_dir().join(format!("docsy-img-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("纯文本.docx");
        let output = dir.join("纯文本.md");

        let docx = Docx::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text("没有图片")));
        std::fs::write(&input, pack_docx(docx)).unwrap();

        convert(&input, &output).unwrap();
        assert!(output.is_file());
        assert!(!dir.join("纯文本_assets").exists(), "无图文档不应创建 assets 目录");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_assets_dir_is_reused_and_overwritten() {
        let dir = std::env::temp_dir().join(format!("docsy-img-reuse-{}", std::process::id()));
        let assets = dir.join("复用_assets");
        std::fs::create_dir_all(&assets).unwrap();
        // 预先放一个会被覆盖的同名文件和一个无关文件
        std::fs::write(assets.join("image1.png"), b"stale").unwrap();
        std::fs::write(assets.join("keep.txt"), b"keep").unwrap();
        let input = dir.join("复用.docx");
        let output = dir.join("复用.md");

        let pic = Pic::new_with_dimensions(png_bytes(1, 1), 1, 1);
        let docx = Docx::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)));
        std::fs::write(&input, pack_docx(docx)).unwrap();

        convert(&input, &output).unwrap();
        // 同名文件被覆盖（不再是 stale），无关文件保留
        assert_ne!(std::fs::read(assets.join("image1.png")).unwrap(), b"stale");
        assert_eq!(std::fs::read(assets.join("keep.txt")).unwrap(), b"keep");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
