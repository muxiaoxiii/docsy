//! Markdown → docx：pulldown-cmark 事件流转 docx-rs 文档。

use anyhow::{Context, Result};
use docx_rs::{
    AbstractNumbering, AlignmentType, BreakType, Docx, Hyperlink, HyperlinkType, IndentLevel,
    Level, LevelJc, LevelText, LineSpacing, NumberFormat, Numbering, NumberingId, PageMargin,
    Paragraph, ParagraphChild, Pic, Run, RunFonts,
    Shading, SpecialIndentType, Start, Style, StyleType, Table, TableCell, TableCellBorder,
    TableCellBorderPosition, TableCellBorders, TableCellMargins, TableLayoutType, TableRow,
    WidthType,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Markdown 生成 DOCX 的版式预设。只影响新生成文件，不会改写导入的文档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocxStylePreset {
    Professional,
    Legal,
    Compact,
}

impl DocxStylePreset {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("professional") {
            "professional" => Ok(Self::Professional),
            "legal" => Ok(Self::Legal),
            "compact" => Ok(Self::Compact),
            other => anyhow::bail!("未知的 Word 文档样式: {other}"),
        }
    }

    fn body_size(self) -> usize {
        match self {
            Self::Professional => 22,
            Self::Legal => 24,
            Self::Compact => 20,
        }
    }

    fn line_spacing(self) -> LineSpacing {
        match self {
            Self::Professional => LineSpacing::new().line(360).after(120),
            Self::Legal => LineSpacing::new().line(420).after(0),
            Self::Compact => LineSpacing::new().line(300).after(60),
        }
    }

    fn page_margin(self) -> PageMargin {
        match self {
            Self::Professional => PageMargin {
                top: 1440,
                right: 1440,
                bottom: 1440,
                left: 1440,
                header: 720,
                footer: 720,
                gutter: 0,
            },
            Self::Legal => PageMargin {
                top: 1440,
                right: 1440,
                bottom: 1440,
                left: 1800,
                header: 720,
                footer: 720,
                gutter: 0,
            },
            Self::Compact => PageMargin {
                top: 1080,
                right: 1080,
                bottom: 1080,
                left: 1080,
                header: 540,
                footer: 540,
                gutter: 0,
            },
        }
    }

    fn heading_color(self) -> &'static str {
        match self {
            Self::Professional => "1F4E79",
            Self::Legal | Self::Compact => "000000",
        }
    }

    fn table_header_color(self) -> &'static str {
        match self {
            Self::Professional => "D9EAF7",
            Self::Legal => "E7E6E6",
            Self::Compact => "EAF1E8",
        }
    }
}

/// 正文字体：西文 Times New Roman；通用版式与紧凑版式使用宋体，法律文书使用仿宋。
fn body_fonts(style: DocxStylePreset) -> RunFonts {
    let east_asia = match style {
        DocxStylePreset::Professional | DocxStylePreset::Compact => "宋体",
        DocxStylePreset::Legal => "仿宋",
    };
    RunFonts::new()
        .ascii("Times New Roman")
        .hi_ansi("Times New Roman")
        .east_asia(east_asia)
}

/// 标题字体：西文 Times New Roman；中文黑体。
fn heading_fonts(_style: DocxStylePreset) -> RunFonts {
    RunFonts::new()
        .ascii("Times New Roman")
        .hi_ansi("Times New Roman")
        .east_asia("黑体")
}

/// 等宽字体：代码块 / 行内代码用。
fn mono_fonts() -> RunFonts {
    RunFonts::new().ascii("Consolas").hi_ansi("Consolas")
}

/// 图片最大宽度（EMU），超出则等比缩小，避免撑出版心。
const MAX_IMAGE_WIDTH_EMU: u32 = 5_500_000;
const EMU_PER_PX: u32 = 9525;

/// 行内样式状态。
#[derive(Default, Clone, Copy)]
struct InlineState {
    bold: bool,
    italic: bool,
    strike: bool,
}

struct ListCtx {
    num_id: usize,
}

/// 表格构建状态。注意 pulldown-cmark 的表格单元格只产生行内事件，
/// 不产生 Paragraph 事件，所以单元格内容直接在 children 里收集。
#[derive(Default)]
struct TableState {
    rows: Vec<TableRow>,
    current_cells: Vec<TableCell>,
    in_header: bool,
}

struct ImageCtx {
    dest: String,
    alt: String,
}

struct Builder<'a> {
    base_dir: &'a Path,
    style: DocxStylePreset,
    docx: Docx,
    next_numbering_id: usize,
    /// 当前块（段落 / 标题 / 表格单元格）的行内容。
    children: Vec<ParagraphChild>,
    inline: InlineState,
    /// 当前处于的标题级别（1~6），标题内 Run 选用标题字体。
    current_heading: Option<usize>,
    /// 链接栈：进入链接时记录 children 长度和目标 URL。
    link_stack: Vec<(usize, String)>,
    image_stack: Vec<ImageCtx>,
    blockquote_depth: usize,
    list_stack: Vec<ListCtx>,
    in_code_block: bool,
    code_buf: String,
    table: Option<TableState>,
    has_primary_heading: bool,
}

fn table_borders() -> TableCellBorders {
    let mut borders = TableCellBorders::with_empty();
    for position in [
        TableCellBorderPosition::Top,
        TableCellBorderPosition::Left,
        TableCellBorderPosition::Bottom,
        TableCellBorderPosition::Right,
    ] {
        borders = borders.set(TableCellBorder::new(position).size(4).color("B7C3D0"));
    }
    borders
}

impl<'a> Builder<'a> {
    fn new(base_dir: &'a Path, style: DocxStylePreset) -> Self {
        Self {
            base_dir,
            style,
            docx: Docx::new(),
            // numId 从 2 开始：docx-rs 写出 numbering.xml 时总会内置
            // abstractNumId=1 + numId=1 的默认十进制编号，不能冲突。
            next_numbering_id: 2,
            children: Vec::new(),
            inline: InlineState::default(),
            current_heading: None,
            link_stack: Vec::new(),
            image_stack: Vec::new(),
            blockquote_depth: 0,
            list_stack: Vec::new(),
            in_code_block: false,
            code_buf: String::new(),
            table: None,
            has_primary_heading: false,
        }
    }

    /// 当前是否处于表头单元格（表头文字加粗）。
    fn header_cell_active(&self) -> bool {
        self.table.as_ref().is_some_and(|t| t.in_header)
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => {
                // 行内代码
                let mut run = Run::new().add_text(code.as_ref()).fonts(mono_fonts());
                if self.inline.bold || self.header_cell_active() {
                    run = run.bold();
                }
                self.children.push(ParagraphChild::Run(Box::new(run)));
            }
            Event::SoftBreak | Event::HardBreak => {
                let run = Run::new().add_break(BreakType::TextWrapping);
                self.children.push(ParagraphChild::Run(Box::new(run)));
            }
            Event::Rule => {
                // 分割线转换为一个空白段落（回车空行），自然分隔段落，避免 Word 中多出无用的实线或边框
                self.flush_paragraph(None);
                let para = Paragraph::new();
                self.docx = std::mem::take(&mut self.docx).add_paragraph(para);
            }
            Event::TaskListMarker(checked) => {
                let mark = if checked { "☑ " } else { "☐ " };
                let run = Run::new().add_text(mark);
                self.children.push(ParagraphChild::Run(Box::new(run)));
            }
            // HTML、脚注、数学公式等不支持的特性直接忽略
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph | Tag::Item => {}
            Tag::Heading { level, .. } => {
                let lv = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.current_heading = Some(lv);
            }
            Tag::BlockQuote(_) => {
                // 紧凑列表项内嵌套块前先闭合未决段落
                self.flush_paragraph(None);
                self.blockquote_depth += 1;
            }
            Tag::CodeBlock(_) => {
                self.flush_paragraph(None);
                self.in_code_block = true;
                self.code_buf.clear();
            }
            Tag::List(start) => {
                // 嵌套列表：先把外层列表项的文本按外层层级闭合
                self.flush_paragraph(None);
                let num_id = self.add_numbering(start);
                self.list_stack.push(ListCtx { num_id });
            }
            Tag::Emphasis => self.inline.italic = true,
            Tag::Strong => self.inline.bold = true,
            Tag::Strikethrough => self.inline.strike = true,
            Tag::Link { dest_url, .. } => {
                self.link_stack
                    .push((self.children.len(), dest_url.to_string()));
            }
            Tag::Image { dest_url, .. } => {
                self.image_stack.push(ImageCtx {
                    dest: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Tag::Table(_) => {
                self.flush_paragraph(None);
                self.table = Some(TableState::default());
            }
            Tag::TableHead => {
                if let Some(t) = self.table.as_mut() {
                    t.in_header = true;
                    t.current_cells.clear();
                }
            }
            Tag::TableRow => {
                if let Some(t) = self.table.as_mut() {
                    t.current_cells.clear();
                }
            }
            Tag::TableCell => {
                // 单元格内容从空 children 开始收集
                self.children.clear();
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush_paragraph(None),
            TagEnd::Heading(level) => {
                let lv = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.flush_paragraph(Some(lv));
                self.current_heading = None;
            }
            TagEnd::BlockQuote(_) => {
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1)
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.flush_code_block();
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            // 紧凑列表（tight list）不触发 Paragraph 事件，文本直接在 Item 里，
            // 在 Item 结束时按段落闭合；宽松列表此处 children 已空，是 no-op。
            TagEnd::Item => self.flush_paragraph(None),
            TagEnd::Emphasis => self.inline.italic = false,
            TagEnd::Strong => self.inline.bold = false,
            TagEnd::Strikethrough => self.inline.strike = false,
            TagEnd::Link => self.close_link(),
            TagEnd::Image => self.close_image(),
            TagEnd::TableHead => {
                if let Some(mut t) = self.table.take() {
                    t.in_header = false;
                    let row = TableRow::new(std::mem::take(&mut t.current_cells));
                    t.rows.push(row);
                    self.table = Some(t);
                }
            }
            TagEnd::TableRow => {
                if let Some(mut t) = self.table.take() {
                    let row = TableRow::new(std::mem::take(&mut t.current_cells));
                    t.rows.push(row);
                    self.table = Some(t);
                }
            }
            TagEnd::TableCell => {
                if let Some(mut t) = self.table.take() {
                    let mut para = Paragraph::new();
                    para.property = para
                        .property
                        .line_spacing(LineSpacing::new().line(280).after(0));
                    for child in std::mem::take(&mut self.children) {
                        para.children.push(child);
                    }
                    let mut cell = TableCell::new()
                        .add_paragraph(para)
                        .set_borders(table_borders());
                    if t.in_header {
                        cell = cell.shading(Shading::new().fill(self.style.table_header_color()));
                    }
                    t.current_cells.push(cell);
                    self.table = Some(t);
                }
            }
            TagEnd::Table => {
                if let Some(t) = self.table.take() {
                    let table = Table::new(t.rows)
                        .layout(TableLayoutType::Fixed)
                        .width(9_000, WidthType::Dxa)
                        .margins(TableCellMargins::new().margin(72, 96, 72, 96));
                    self.docx = std::mem::take(&mut self.docx).add_table(table);
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_buf.push_str(text);
            return;
        }
        if let Some(img) = self.image_stack.last_mut() {
            img.alt.push_str(text);
            return;
        }
        let fonts = if self.current_heading.is_some() {
            heading_fonts(self.style)
        } else {
            body_fonts(self.style)
        };
        let mut run = Run::new().add_text(text).fonts(fonts);
        if self.inline.bold || self.header_cell_active() {
            run = run.bold();
        }
        if self.inline.italic {
            run = run.italic();
        }
        if self.inline.strike {
            run = run.strike();
        }
        self.children.push(ParagraphChild::Run(Box::new(run)));
    }

    /// 结束段落：套用标题样式 / 列表编号 / 引用缩进。
    fn flush_paragraph(&mut self, heading: Option<usize>) {
        let children = std::mem::take(&mut self.children);
        // 空段落不输出，避免多余空行
        if children.is_empty() {
            return;
        }
        let mut para = Paragraph::new();
        if let Some(level) = heading {
            para = para.style(&format!("Heading{level}"));
            // Markdown 的首个 H1 通常就是文档标题；标题不应孤立在页尾。
            para = para.keep_next(true);
            if level == 1 && !self.has_primary_heading {
                para = para.align(AlignmentType::Center);
                self.has_primary_heading = true;
            }
        }
        if self.blockquote_depth > 0 {
            // 引用块：整段左缩进
            para = para.indent(Some(420 * self.blockquote_depth as i32), None, None, None);
        } else if heading.is_none()
            && self.list_stack.is_empty()
            && self.style == DocxStylePreset::Legal
        {
            // 法律文书正文通常使用首行缩进两字符；标题、列表和引用不套用。
            para = para.indent(None, Some(SpecialIndentType::FirstLine(480)), None, None);
        }
        if let Some(list) = self.list_stack.last() {
            let level = (self.list_stack.len() - 1).min(8);
            para = para.numbering(NumberingId::new(list.num_id), IndentLevel::new(level));
        }
        for child in children {
            para.children.push(child);
        }
        self.docx = std::mem::take(&mut self.docx).add_paragraph(para);
    }

    fn flush_code_block(&mut self) {
        let code = std::mem::take(&mut self.code_buf);
        let mut lines: Vec<&str> = code.lines().collect();
        if lines.is_empty() {
            lines.push("");
        }
        for line in lines {
            let run = Run::new().add_text(line).fonts(mono_fonts());
            let mut para = Paragraph::new().add_run(run);
            para.property = para
                .property
                .line_spacing(LineSpacing::new().line(260).after(0))
                .indent(Some(360), None, None, None)
                .shading(Shading::new().fill("F5F5F5"));
            self.docx = std::mem::take(&mut self.docx).add_paragraph(para);
        }
        self.in_code_block = false;
    }

    /// 闭合链接：把进入链接后产生的 Run 包进超链接。
    fn close_link(&mut self) {
        let Some((checkpoint, url)) = self.link_stack.pop() else {
            return;
        };
        let drained: Vec<ParagraphChild> = self.children.drain(checkpoint..).collect();
        let mut link = Hyperlink::new(&url, HyperlinkType::External);
        let mut rest = Vec::new();
        for child in drained {
            match child {
                ParagraphChild::Run(run) => {
                    let mut run = *run;
                    run.run_property = run.run_property.color("0563C1").underline("single");
                    link = link.add_run(run);
                }
                other => rest.push(other),
            }
        }
        self.children.push(ParagraphChild::Hyperlink(link));
        self.children.extend(rest);
    }

    /// 闭合图片：本地文件存在则嵌入，否则输出占位文本。
    fn close_image(&mut self) {
        let Some(ctx) = self.image_stack.pop() else {
            return;
        };
        let placeholder = || {
            let text = format!("![{}]({})", ctx.alt, ctx.dest);
            ParagraphChild::Run(Box::new(Run::new().add_text(text)))
        };
        if ctx.dest.starts_with("http://") || ctx.dest.starts_with("https://") {
            self.children.push(placeholder());
            return;
        }
        let path = {
            let p = PathBuf::from(&ctx.dest);
            if p.is_absolute() {
                p
            } else {
                self.base_dir.join(p)
            }
        };
        match build_pic(&path) {
            Some(pic) => {
                let run = Run::new().add_image(pic);
                self.children.push(ParagraphChild::Run(Box::new(run)));
            }
            None => self.children.push(placeholder()),
        }
    }

    /// 为列表分配一套编号定义，返回 numId。
    fn add_numbering(&mut self, start: Option<u64>) -> usize {
        let id = self.next_numbering_id;
        self.next_numbering_id += 1;
        let mut ab = AbstractNumbering::new(id);
        for level in 0..=8usize {
            let indent_left = 420 * (level as i32 + 1);
            let lv = match start {
                Some(n) => Level::new(
                    level,
                    Start::new(n as usize),
                    NumberFormat::new("decimal"),
                    LevelText::new(format!("%{}.", level + 1)),
                    LevelJc::new("left"),
                ),
                None => Level::new(
                    level,
                    Start::new(1),
                    NumberFormat::new("bullet"),
                    LevelText::new("•"),
                    LevelJc::new("left"),
                ),
            }
            .indent(
                Some(indent_left),
                Some(SpecialIndentType::Hanging(210)),
                None,
                None,
            );
            ab = ab.add_level(lv);
        }
        self.docx = std::mem::take(&mut self.docx).add_abstract_numbering(ab);
        self.docx = std::mem::take(&mut self.docx).add_numbering(Numbering::new(id, id));
        id
    }
}

/// 读取本地图片并转成 PNG Pic；失败返回 None。
fn build_pic(path: &Path) -> Option<Pic> {
    if !path.is_file() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let (w_px, h_px) = (img.width(), img.height());
    if w_px == 0 || h_px == 0 {
        return None;
    }
    let mut png = Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png).ok()?;
    let mut w_emu = w_px.saturating_mul(EMU_PER_PX);
    let mut h_emu = h_px.saturating_mul(EMU_PER_PX);
    if w_emu > MAX_IMAGE_WIDTH_EMU {
        let scale = f64::from(MAX_IMAGE_WIDTH_EMU) / f64::from(w_emu);
        w_emu = MAX_IMAGE_WIDTH_EMU;
        h_emu = (f64::from(h_emu) * scale) as u32;
    }
    Some(Pic::new_with_dimensions(png.into_inner(), w_px, h_px).size(w_emu, h_emu))
}

/// 内置标题样式：黑体加粗 + 西文 Times New Roman + 规范段前段后间距。
fn heading_styles(style: DocxStylePreset) -> Vec<Style> {
    // (级别, 字号 half-points, 段前 twips, 段后 twips)
    let specs = match style {
        DocxStylePreset::Professional => [
            (1, 36, 240, 120),
            (2, 30, 200, 100),
            (3, 26, 160, 80),
            (4, 24, 120, 60),
            (5, 22, 100, 50),
            (6, 22, 100, 50),
        ],
        DocxStylePreset::Legal => [
            (1, 36, 240, 120),
            (2, 32, 200, 100),
            (3, 28, 160, 80),
            (4, 24, 120, 60),
            (5, 24, 100, 50),
            (6, 24, 100, 50),
        ],
        DocxStylePreset::Compact => [
            (1, 30, 160, 80),
            (2, 26, 140, 70),
            (3, 24, 120, 60),
            (4, 22, 100, 50),
            (5, 20, 80, 40),
            (6, 20, 80, 40),
        ],
    };
    specs
        .iter()
        .map(|(level, size, before, after)| {
            Style::new(format!("Heading{level}"), StyleType::Paragraph)
                .name(format!("heading {level}"))
                .bold()
                .size(*size)
                .color(style.heading_color())
                .fonts(heading_fonts(style))
                .line_spacing(LineSpacing::new().before(*before).after(*after))
                .outline_lvl(level - 1)
        })
        .collect()
}

/// 生成 docx 字节（供写文件与测试复用）。
pub fn build_docx_bytes_with_style(
    md: &str,
    base_dir: &Path,
    style: DocxStylePreset,
) -> Result<Vec<u8>> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(md, options);
    let mut builder = Builder::new(base_dir, style);
    for event in parser {
        builder.handle_event(event);
    }
    let mut docx = builder.docx;
    docx = docx
        // A4（210 × 297 mm），避免由用户本机默认模板决定纸张大小。
        .page_size(11_906, 16_838)
        .default_fonts(body_fonts(style))
        .default_size(style.body_size())
        .default_line_spacing(style.line_spacing())
        .page_margin(style.page_margin());
    for heading_style in heading_styles(style) {
        docx = docx.add_style(heading_style);
    }
    let mut buf = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut buf)
        .map_err(|e| anyhow::anyhow!("打包 docx 失败: {e}"))?;
    Ok(buf.into_inner())
}

#[cfg(test)]
pub fn build_docx_bytes(md: &str, base_dir: &Path) -> Result<Vec<u8>> {
    build_docx_bytes_with_style(md, base_dir, DocxStylePreset::Professional)
}

pub fn convert(input: &Path, output: &Path, style: DocxStylePreset) -> Result<()> {
    let md = std::fs::read_to_string(input)
        .with_context(|| format!("无法读取 Markdown 文件: {}", input.display()))?;
    let base_dir = input.parent().unwrap_or_else(|| Path::new("."));
    let bytes = build_docx_bytes_with_style(&md, base_dir, style)?;
    std::fs::write(output, &bytes)
        .with_context(|| format!("无法写入 docx 文件: {}", output.display()))?;
    // 自验证：生成的文件必须能被 docx-rs reader 读回
    let back = docx_rs::read_docx(&bytes).context("生成的 docx 无法读回校验")?;
    if back.document.children.is_empty() && !md.trim().is_empty() {
        anyhow::bail!("生成的 docx 内容为空，转换失败");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::DocumentChild;
    use std::io::Read;

    const SAMPLE_MD: &str = r#"# 合同标题

正文 **加粗** 和 *斜体* 以及 ~~删除线~~ 与 `code`。

## 第二节

- 项目一
- 项目二
  - 子项目

1. 第一条
2. 第二条

| 姓名 | 金额 |
| --- | --- |
| 张三 | 100 |
| 李四 | 200 |

```
fn main() {}
let x = 1;
```

> 引用内容

---

[链接](https://example.com)

- [x] 已完成事项
- [ ] 未完成事项
"#;

    #[test]
    fn builds_valid_docx_with_expected_structure() {
        let bytes = build_docx_bytes(SAMPLE_MD, Path::new(".")).unwrap();
        // 自验证：能读回
        let docx = docx_rs::read_docx(&bytes).unwrap();
        let has_heading = |style: &str| {
            docx.document.children.iter().any(|c| {
                matches!(c, DocumentChild::Paragraph(p)
                    if p.property.style.as_ref().is_some_and(|s| s.val == style))
            })
        };
        assert!(has_heading("Heading1"));
        assert!(has_heading("Heading2"));
        // 表格：1 表头 + 2 数据行
        let table = docx.document.children.iter().find_map(|c| match c {
            DocumentChild::Table(t) => Some(t),
            _ => None,
        });
        assert!(table.is_some());
        assert_eq!(table.map(|t| t.rows.len()), Some(3));
        // 列表编号定义存在
        assert!(!docx.numberings.abstract_nums.is_empty());
    }

    #[test]
    fn round_trip_preserves_key_structure() {
        let bytes = build_docx_bytes(SAMPLE_MD, Path::new(".")).unwrap();
        let md = crate::markdown::docx_to_md::docx_bytes_to_md(&bytes).unwrap();
        // 标题级别
        assert!(md.contains("# 合同标题"), "md:\n{md}");
        assert!(md.contains("## 第二节"), "md:\n{md}");
        // 行内标记
        assert!(md.contains("**加粗**"), "md:\n{md}");
        assert!(md.contains("*斜体*"), "md:\n{md}");
        assert!(md.contains("~~删除线~~"), "md:\n{md}");
        assert!(md.contains("`code`"), "md:\n{md}");
        // 列表（含嵌套与有序序号）
        assert!(md.contains("- 项目一"), "md:\n{md}");
        assert!(md.contains("  - 子项目"), "md:\n{md}");
        assert!(md.contains("1. 第一条"), "md:\n{md}");
        assert!(md.contains("2. 第二条"), "md:\n{md}");
        // 表格行列
        assert!(md.contains("| 姓名 | 金额 |"), "md:\n{md}");
        assert!(md.contains("| 张三 | 100 |"), "md:\n{md}");
        assert!(md.contains("| 李四 | 200 |"), "md:\n{md}");
        // 代码块
        assert!(md.contains("```"), "md:\n{md}");
        assert!(md.contains("fn main() {}"), "md:\n{md}");
        // 链接
        assert!(md.contains("[链接](https://example.com)"), "md:\n{md}");
        // 任务列表
        assert!(
            md.contains("☑ 已完成事项") || md.contains("已完成事项"),
            "md:\n{md}"
        );
    }

    #[test]
    fn all_word_style_presets_emit_a4_and_distinct_typography() {
        for (preset, east_asia_font, heading_color) in [
            (DocxStylePreset::Professional, "宋体", "1F4E79"),
            (DocxStylePreset::Legal, "仿宋", "000000"),
            (DocxStylePreset::Compact, "宋体", "000000"),
        ] {
            let bytes =
                build_docx_bytes_with_style("# 标题\n\n正文", Path::new("."), preset).unwrap();
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut document_xml = String::new();
            archive
                .by_name("word/document.xml")
                .unwrap()
                .read_to_string(&mut document_xml)
                .unwrap();
            let mut styles_xml = String::new();
            archive
                .by_name("word/styles.xml")
                .unwrap()
                .read_to_string(&mut styles_xml)
                .unwrap();

            assert!(document_xml.contains("w:w=\"11906\""), "A4 width missing");
            assert!(document_xml.contains("w:h=\"16838\""), "A4 height missing");
            assert!(
                styles_xml.contains(east_asia_font),
                "font missing: {east_asia_font}"
            );
            assert!(
                styles_xml.contains(heading_color),
                "heading color missing: {heading_color}"
            );
            assert!(
                styles_xml.contains("黑体"),
                "heading font 黑体 missing in styles"
            );
        }
    }

    #[test]
    fn missing_image_falls_back_to_placeholder() {
        let md = "![示意图](no-such-image-xyz.png)";
        let bytes = build_docx_bytes(md, Path::new("/tmp")).unwrap();
        let back = crate::markdown::docx_to_md::docx_bytes_to_md(&bytes).unwrap();
        // 占位文本读回时方括号按 Markdown 规则转义
        assert!(
            back.contains("!\\[示意图\\](no-such-image-xyz.png)"),
            "md:\n{back}"
        );
    }

    #[test]
    fn local_image_is_embedded() {
        // 造一个 1x1 PNG 到临时目录
        let dir = std::env::temp_dir().join(format!("docsy-md-img-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let img_path = dir.join("pixel.png");
        let mut png = Cursor::new(Vec::new());
        let img = image::DynamicImage::new_rgba8(1, 1);
        img.write_to(&mut png, image::ImageFormat::Png).unwrap();
        std::fs::write(&img_path, png.into_inner()).unwrap();

        let bytes = build_docx_bytes("![点](pixel.png)", &dir).unwrap();
        let docx = docx_rs::read_docx(&bytes).unwrap();
        assert!(!docx.images.is_empty(), "图片应嵌入 docx");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_markdown_produces_empty_doc() {
        let bytes = build_docx_bytes("", Path::new(".")).unwrap();
        let docx = docx_rs::read_docx(&bytes).unwrap();
        assert!(docx.document.children.is_empty());
    }

    #[test]
    fn horizontal_rule_emits_blank_paragraph() {
        let bytes = build_docx_bytes("hello\n\n---\n\nworld", Path::new(".")).unwrap();
        let docx = docx_rs::read_docx(&bytes).unwrap();
        assert_eq!(docx.document.children.len(), 3);
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();
        // 分割线转为纯空段落（回车空行），不带有任何边框或方框
        assert!(!document_xml.contains("<w:pBdr"));
    }
}
