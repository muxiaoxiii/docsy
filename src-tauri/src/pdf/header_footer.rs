use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::external::ExternalTool;

use super::annotations;
use super::artifacts;
use super::content_text;
use super::normalize::normalize_pdf_to_a4;
use super::page_info::get_page_infos;
use super::preview::{render_preview, PreviewResult};
use super::qpdf;
use super::{same_path, temp_named_path};
use crate::util::fs::{set_private_permissions, TempPathGuard};

// 以下仅测试使用
#[cfg(test)]
use lopdf::content::{Content, Operation};
#[cfg(test)]
use lopdf::{dictionary, Document, Object, Stream};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooterJob {
    #[serde(alias = "input")]
    pub input_path: String,
    #[serde(alias = "output")]
    pub output_path: String,
    #[serde(default = "default_page_start")]
    page_start: u32,
    #[serde(default)]
    total_pages: Option<u32>,
    #[serde(default)]
    normalize_a4: bool,
    #[serde(default = "default_a4_orientation")]
    a4_orientation: String,
    #[serde(default = "default_raster_dpi")]
    raster_dpi: u32,
    #[serde(default)]
    cleanup: CleanupConfig,
    #[serde(default)]
    header: Option<OverlayTextConfig>,
    #[serde(default)]
    footer: Option<OverlayTextConfig>,
    #[serde(default)]
    extra_overlays: Vec<OverlayTextConfig>,
    #[serde(default)]
    pub bookmarks: Vec<BookmarkConfig>,
    #[serde(default)]
    pub bookmark_remove_existing: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupConfig {
    #[serde(default)]
    header_enabled: bool,
    #[serde(default)]
    footer_enabled: bool,
    #[serde(default)]
    force_delete_header: bool,
    #[serde(default)]
    force_delete_footer: bool,
    #[serde(default = "default_cleanup_zone_mm")]
    header_height_mm: f32,
    #[serde(default = "default_cleanup_zone_mm")]
    footer_height_mm: f32,
    #[serde(default)]
    plain_header_targets: Vec<PlainTextCleanupTargetConfig>,
    #[serde(default)]
    plain_footer_targets: Vec<PlainTextCleanupTargetConfig>,
    #[serde(default)]
    artifact_header_targets: Vec<ArtifactCleanupTargetConfig>,
    #[serde(default)]
    artifact_footer_targets: Vec<ArtifactCleanupTargetConfig>,
    #[serde(default)]
    header_replacement: Option<OverlayTextConfig>,
    #[serde(default)]
    footer_replacement: Option<OverlayTextConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactCleanupTargetConfig {
    #[serde(default)]
    artifact_id: Option<String>,
    #[serde(default)]
    normalized_text: String,
    #[serde(default = "default_page_start")]
    page_start: u32,
    #[serde(default)]
    page_end: u32,
    #[serde(default)]
    docsy_kind: Option<String>,
    #[serde(default)]
    replacement_text: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlainTextCleanupTargetConfig {
    text: String,
    #[serde(default)]
    normalized_text: String,
    #[serde(default = "default_page_start")]
    page_start: u32,
    #[serde(default)]
    page_end: u32,
    #[serde(default)]
    bbox: Option<PlainTextCleanupBBoxConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlainTextCleanupBBoxConfig {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    page: u32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub page_index: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTextConfig {
    pub text: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f32,
    #[serde(default = "default_align")]
    pub align: String,
    #[serde(default)]
    pub offset_x_mm: f32,
    #[serde(default = "default_text_color")]
    pub color: String,
    #[serde(default)]
    pub page_start: Option<u32>,
    #[serde(default)]
    pub page_end: Option<u32>,
    #[serde(default)]
    pub number_style: String,
    #[serde(default)]
    pub number_offset: i32,
    #[serde(default)]
    pub number_total: Option<u32>,
    #[serde(default)]
    pub artifact_kind: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAnnotationRule {
    #[serde(default, alias = "remove")]
    pub remove_annotations: bool,
    #[serde(default)]
    pub kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooterResult {
    pub input_path: String,
    pub output_path: String,
    pub pages: u32,
    pub normalized: bool,
    pub cleaned: bool,
    pub semantic_removed: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooterFailure {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchHeaderFooterResult {
    pub results: Vec<HeaderFooterResult>,
    pub failed: Vec<HeaderFooterFailure>,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub processed: usize,
    #[serde(default)]
    pub total: usize,
}

impl BatchHeaderFooterResult {
    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
            failed: Vec::new(),
            cancelled: false,
            processed: 0,
            total: 0,
        }
    }
}

fn default_page_start() -> u32 {
    1
}

fn default_raster_dpi() -> u32 {
    200
}

fn default_font_size() -> f32 {
    10.0
}

fn default_margin_mm() -> f32 {
    10.0
}

fn default_cleanup_zone_mm() -> f32 {
    18.0
}

fn default_align() -> String {
    "center".to_string()
}

fn default_text_color() -> String {
    "#000000".to_string()
}

fn default_a4_orientation() -> String {
    "preserve".to_string()
}

pub fn overlay_text(args: &HeaderFooterJob) -> Result<HeaderFooterResult> {
    process_job(args, None)
}

/// MDG-001: 带取消支持的 batch_overlay 包装。
///
/// 在每个 item 处理前检查 CancellationToken，
/// 用户取消时返回已处理的部分结果（不会丢失已完成的工作）。
/// progress 回调在每个 item 处理前触发，参数为 (当前序号, 总数, 输入路径)，
/// 不关心进度的调用方可传空闭包。
pub fn batch_overlay_cancellable(
    items: &[HeaderFooterJob],
    token: &tokio_util::sync::CancellationToken,
    progress: &dyn Fn(usize, usize, &str),
) -> Result<BatchHeaderFooterResult> {
    let mut results = Vec::new();
    let mut failed = Vec::new();
    let total = items.len();

    for (index, job) in items.iter().enumerate() {
        progress(index + 1, total, &job.input_path);
        if token.is_cancelled() {
            return Ok(BatchHeaderFooterResult {
                results,
                failed,
                cancelled: true,
                processed: index,
                total,
            });
        }

        match process_job(job, Some(token)) {
            Ok(result) => results.push(result),
            Err(err) => failed.push(HeaderFooterFailure {
                path: job.input_path.clone(),
                message: err.to_string(),
            }),
        }
    }

    Ok(BatchHeaderFooterResult {
        results,
        failed,
        cancelled: false,
        processed: total,
        total,
    })
}

pub fn batch_overlay(items: &[HeaderFooterJob]) -> Result<BatchHeaderFooterResult> {
    if let Some(first) = items.first() {
        log::debug!(
            "[batch_overlay] first item bookmarks={:?}, bookmarkRemoveExisting={:?}",
            first.bookmarks,
            first.bookmark_remove_existing
        );
    }

    let mut results = Vec::new();
    let mut failed = Vec::new();
    let total = items.len();

    for job in items {
        match process_job(job, None) {
            Ok(result) => results.push(result),
            Err(err) => failed.push(HeaderFooterFailure {
                path: job.input_path.clone(),
                message: err.to_string(),
            }),
        }
    }

    Ok(BatchHeaderFooterResult {
        results,
        failed,
        cancelled: false,
        processed: total,
        total,
    })
}

pub fn preview_overlay(
    job: &HeaderFooterJob,
    page: Option<u32>,
    dpi: Option<u32>,
    annotation_rule: Option<&PreviewAnnotationRule>,
) -> Result<PreviewResult> {
    let mut job = job.clone();
    let page = page.unwrap_or(1);
    let dpi = dpi.unwrap_or(120);

    let preview_output = temp_named_path("docsy_hf_preview", "pdf");
    job.output_path = preview_output.to_string_lossy().to_string();
    let annotation_temp = if annotation_rule.is_some_and(|r| r.remove_annotations) {
        let rule = annotation_rule.unwrap();
        let temp = annotations::delete_annotations_to_temp(&job.input_path, &rule.kinds)
            .context("预览前删除批注失败")?;
        job.input_path = temp.to_string_lossy().to_string();
        Some(temp)
    } else {
        None
    };

    let result = process_job(&job, None).and_then(|_| {
        render_preview(&super::preview::PreviewArgs {
            input_path: job.output_path.clone(),
            page,
            dpi,
        })
    });
    let _ = fs::remove_file(&preview_output);
    cleanup_temp(annotation_temp);
    result
}

// 书签函数已拆分到 bookmarks.rs，此处 re-export 保持公共 API 不变。
pub use super::bookmarks::{apply_bookmarks, has_pdf_bookmarks, remove_pdf_bookmarks};

fn process_job(
    args: &HeaderFooterJob,
    token: Option<&tokio_util::sync::CancellationToken>,
) -> Result<HeaderFooterResult> {
    let job_start = std::time::Instant::now();
    let input = Path::new(&args.input_path);
    if !input.exists() {
        anyhow::bail!("原始 PDF 不存在: {}", input.display());
    }
    let file_name = input
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let output_requested = Path::new(&args.output_path);
    let output_path = unique_output_path(output_requested);
    let output = output_path.as_path();
    if same_path(input, output) {
        anyhow::bail!("输出路径不能和原始 PDF 相同，请另存为副本");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("创建输出目录失败")?;
    }

    let t0 = std::time::Instant::now();
    let semantic_deleted_path = edit_or_delete_standard_artifacts_if_requested(args)?;
    let artifact_elapsed = t0.elapsed().as_millis();
    let _semantic_temp_guard = semantic_deleted_path
        .as_ref()
        .map(|result| TempPathGuard::new(result.path.clone()));
    let artifact_removed = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.changed_count())
        .unwrap_or(0);
    let mut warnings = standard_artifact_processing_warnings(args, semantic_deleted_path.as_ref());
    let semantic_input = semantic_deleted_path
        .as_ref()
        .map(|artifact_result| artifact_result.path.as_path())
        .unwrap_or(input);
    let t1 = std::time::Instant::now();
    let plain_deleted_path =
        delete_confirmed_plain_text_header_footer_if_requested(args, semantic_input)?;
    let plain_elapsed = t1.elapsed().as_millis();
    let _plain_text_temp_guard = plain_deleted_path
        .as_ref()
        .map(|(path, _)| TempPathGuard::new(path.clone()));
    let plain_removed = plain_deleted_path
        .as_ref()
        .map(|(_, result)| result.removed())
        .unwrap_or(0);
    warnings.extend(plain_text_processing_warnings(
        args,
        plain_deleted_path.as_ref().map(|(_, result)| result),
    ));
    let cleanup_input = plain_deleted_path
        .as_ref()
        .map(|(path, _)| path.as_path())
        .unwrap_or(semantic_input);
    let semantic_removed = artifact_removed + plain_removed;
    if (args.cleanup.header_enabled
        || args.cleanup.footer_enabled
        || !args.cleanup.plain_header_targets.is_empty()
        || !args.cleanup.plain_footer_targets.is_empty())
        && semantic_removed == 0
    {
        warnings.push(
            "已请求删除现有页眉页脚，但没有找到可安全删除的匹配内容；原文未被遮盖或改写"
                .to_string(),
        );
    }
    let semantic_rebuild_overlays = artifact_rebuild_overlays(args, semantic_deleted_path.as_ref());

    let t2 = std::time::Instant::now();
    let normalized_path = if args.normalize_a4 {
        Some(normalize_pdf_to_a4(
            cleanup_input,
            args.raster_dpi,
            &args.a4_orientation,
        )?)
    } else {
        None
    };
    let normalize_elapsed = t2.elapsed().as_millis();
    let _normalized_temp_guard = normalized_path
        .as_ref()
        .map(|path| TempPathGuard::new(path.clone()));
    let work_input = normalized_path.as_deref().unwrap_or(cleanup_input);
    let work_input_str = work_input.to_string_lossy().to_string();
    let page_infos = get_page_infos(&work_input_str)?;
    let pages = page_infos.len() as u32;
    let page_start = args.page_start.max(1);
    let total_pages = args.total_pages.unwrap_or(pages);
    let end_page = page_start + pages.saturating_sub(1);
    if total_pages < end_page {
        let uses_page_placeholders = [args.header.as_ref(), args.footer.as_ref()]
            .into_iter()
            .flatten()
            .any(overlay_uses_page_placeholders)
            || args
                .extra_overlays
                .iter()
                .any(overlay_uses_page_placeholders);
        if uses_page_placeholders {
            anyhow::bail!(
                "全局总页数 {total_pages} 小于当前 PDF 的结束页码 {end_page}，页码占位符将越界"
            );
        }
        warnings.push(format!(
            "全局总页数 {total_pages} 小于当前 PDF 的结束页码 {end_page}，页码按 {total_pages} 截断显示"
        ));
    }
    let cleaned = args.cleanup.header_enabled || args.cleanup.footer_enabled;
    if args.header.is_none()
        && args.footer.is_none()
        && args.extra_overlays.is_empty()
        && semantic_rebuild_overlays.is_empty()
    {
        fs::copy(work_input, output).context("复制 PDF 失败")?;
        // Bookmarks are now written after merge, not during per-file processing
        cleanup_temp(normalized_path);
        cleanup_plain_text_temp(plain_deleted_path);
        cleanup_semantic_temp(semantic_deleted_path);
        return Ok(HeaderFooterResult {
            input_path: args.input_path.clone(),
            output_path: output.to_string_lossy().to_string(),
            pages,
            normalized: args.normalize_a4,
            cleaned,
            semantic_removed,
            warnings,
        });
    }

    let extra_overlays = combined_extra_overlays(args, &semantic_rebuild_overlays);
    crate::app_log::info(
        "pdf.header_footer",
        "overlay plan",
        serde_json::json!({
            "file": &file_name,
            "header": args.header.as_ref().map(|config| serde_json::json!({
                "text": &config.text,
                "region": config.region,
                "pageStart": config.page_start,
                "pageEnd": config.page_end,
            })),
            "footer": args.footer.as_ref().map(|config| serde_json::json!({
                "text": &config.text,
                "region": config.region,
                "pageStart": config.page_start,
                "pageEnd": config.page_end,
            })),
            "extraCount": extra_overlays.len(),
            "extra": extra_overlays.iter().take(8).map(|config| serde_json::json!({
                "text": &config.text,
                "region": config.region,
                "pageStart": config.page_start,
                "pageEnd": config.page_end,
            })).collect::<Vec<_>>(),
        }),
    );
    let t3 = std::time::Instant::now();
    let (overlay_pdf, mut overlay_warnings) = build_overlay_pdf(
        args.header.as_ref(),
        args.footer.as_ref(),
        &extra_overlays,
        &page_infos,
        page_start,
        total_pages,
    )?;
    warnings.append(&mut overlay_warnings);
    let overlay_build_elapsed = t3.elapsed().as_millis();
    let overlay_path = TempPathGuard::new(temp_named_path("docsy_overlay", "pdf"));
    fs::write(overlay_path.path(), &overlay_pdf).context("写入临时页眉页脚层失败")?;
    // 文件在 guard 创建后才写入，这里收紧临时文件权限（非 unix 为 no-op）
    let _ = set_private_permissions(overlay_path.path());

    let qpdf_tool = crate::external::QpdfTool;
    let bin = qpdf_tool.binary_path()?;
    let overlay_output = TempPathGuard::new(temp_named_path("docsy_overlay_result", "pdf"));
    let t4 = std::time::Instant::now();
    // Redirect stderr to null to avoid "Broken pipe (os error 32)" when qpdf
    // writes progress information to stderr and the pipe is already closed.
    // On failure, re-run briefly to capture the error detail.
    let mut command = crate::external::hidden_command(&bin);
    command
        .arg(work_input)
        .arg("--overlay")
        .arg(overlay_path.path())
        .arg("--")
        .arg(overlay_output.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let command_status = run_qpdf_overlay(&mut command, token)?;
    let qpdf_elapsed = t4.elapsed().as_millis();

    cleanup_temp(normalized_path);
    cleanup_plain_text_temp(plain_deleted_path);
    cleanup_semantic_temp(semantic_deleted_path);

    if !super::qpdf::status_is_success(&command_status) {
        anyhow::bail!(
            "qpdf overlay 失败，退出码: {}",
            command_status.code().unwrap_or(-1)
        );
    }
    // qpdf 刚生成的临时结果文件，收紧权限（非 unix 为 no-op）
    let _ = set_private_permissions(overlay_output.path());
    write_optimized_or_copy(overlay_output.path(), output).context("写入页眉页脚处理结果失败")?;
    // Bookmarks are now written after merge, not during per-file processing

    let total_elapsed = job_start.elapsed().as_millis();
    log::info!(
        "process_job.timing file={:?} total={}ms artifact={}ms plain_delete={}ms normalize={}ms overlay_build={}ms qpdf={}ms pages={}",
        file_name, total_elapsed, artifact_elapsed, plain_elapsed, normalize_elapsed,
        overlay_build_elapsed, qpdf_elapsed, pages
    );

    Ok(HeaderFooterResult {
        input_path: args.input_path.clone(),
        output_path: output.to_string_lossy().to_string(),
        pages,
        normalized: args.normalize_a4,
        cleaned,
        semantic_removed,
        warnings: std::mem::take(&mut warnings),
    })
}

/// 在页眉页脚批处理中执行 qpdf overlay，并在用户取消时立即终止当前子进程。
/// 普通单文件调用没有取消令牌，保持原来的等待行为。
fn run_qpdf_overlay(
    command: &mut std::process::Command,
    token: Option<&tokio_util::sync::CancellationToken>,
) -> Result<std::process::ExitStatus> {
    let Some(token) = token else {
        return command.status().context("执行 qpdf overlay 失败");
    };

    let mut child = command.spawn().context("启动 qpdf overlay 失败")?;
    let operation_id = format!("qpdf:overlay:{}", child.id());
    if let Some(registry) = crate::get_subprocess_registry() {
        registry.register(&operation_id, child.id());
    }

    let result = loop {
        if token.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            break Err(anyhow::anyhow!("操作已取消"));
        }
        match child.try_wait().context("等待 qpdf overlay 失败")? {
            Some(status) => break Ok(status),
            None => std::thread::sleep(std::time::Duration::from_millis(80)),
        }
    };

    if let Some(registry) = crate::get_subprocess_registry() {
        registry.unregister(&operation_id);
    }
    result
}

fn delete_confirmed_plain_text_header_footer_if_requested(
    args: &HeaderFooterJob,
    input: &Path,
) -> Result<Option<(PathBuf, content_text::PlainTextCleanupResult)>> {
    if args.cleanup.plain_header_targets.is_empty() && args.cleanup.plain_footer_targets.is_empty()
    {
        return Ok(None);
    }
    let plan = content_text::PlainTextCleanupPlan {
        header_targets: args
            .cleanup
            .plain_header_targets
            .iter()
            .map(plain_text_target_from_config)
            .collect(),
        footer_targets: args
            .cleanup
            .plain_footer_targets
            .iter()
            .map(plain_text_target_from_config)
            .collect(),
        header_zone_mm: args.cleanup.header_height_mm,
        footer_zone_mm: args.cleanup.footer_height_mm,
    };
    content_text::delete_plain_header_footer_to_temp(&input.to_string_lossy(), &plan)
}

fn plain_text_target_from_config(
    config: &PlainTextCleanupTargetConfig,
) -> content_text::PlainTextTarget {
    let page_start = config.page_start.max(1);
    content_text::PlainTextTarget {
        text: config.text.clone(),
        normalized_text: if config.normalized_text.is_empty() {
            config.text.clone()
        } else {
            config.normalized_text.clone()
        },
        page_start,
        page_end: config.page_end.max(page_start),
        bbox: config
            .bbox
            .as_ref()
            .map(|bbox| content_text::PlainTextTargetBBox {
                x0: bbox.x0,
                y0: bbox.y0,
                x1: bbox.x1,
                y1: bbox.y1,
                page: bbox.page,
                width: bbox.width,
                height: bbox.height,
            }),
    }
}

fn write_optimized_or_copy(input: &Path, output: &Path) -> Result<()> {
    match qpdf::optimize_to(input, output) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(input, output).context("复制 PDF 处理结果失败")?;
            Ok(())
        }
    }
}

// 收敛说明：统一委托 crate::util::fs::unique_output_path。与原本地实现的差异仅在
// 撞名场景：原实现从 `-1` 起编号、超限后回退覆盖原路径，现从 `-2` 起、超限回退
// 时间戳命名（不再覆盖已有文件）。
fn unique_output_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("output");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    crate::util::fs::unique_output_path(parent, stem, extension)
}

struct StandardArtifactProcessingResult {
    path: PathBuf,
    removed_header: usize,
    removed_footer: usize,
    edited_header: usize,
    edited_footer: usize,
    removed_header_pages: BTreeSet<usize>,
    removed_footer_pages: BTreeSet<usize>,
}

impl StandardArtifactProcessingResult {
    fn changed_count(&self) -> usize {
        self.removed_header + self.removed_footer + self.edited_header + self.edited_footer
    }
}

fn edit_or_delete_standard_artifacts_if_requested(
    args: &HeaderFooterJob,
) -> Result<Option<StandardArtifactProcessingResult>> {
    if !args.cleanup.header_enabled
        && !args.cleanup.footer_enabled
        && args.cleanup.artifact_header_targets.is_empty()
        && args.cleanup.artifact_footer_targets.is_empty()
    {
        return Ok(None);
    }
    let page_infos = get_page_infos(&args.input_path)?;
    let page_count = page_infos.len();
    let header_texts = args
        .cleanup
        .header_replacement
        .as_ref()
        .filter(|_| !args.cleanup.force_delete_header)
        .map(|config| {
            artifact_replacement_texts(config, page_count, args.page_start, args.total_pages)
        })
        .unwrap_or_default();
    let footer_texts = args
        .cleanup
        .footer_replacement
        .as_ref()
        .filter(|_| !args.cleanup.force_delete_footer)
        .map(|config| {
            artifact_replacement_texts(config, page_count, args.page_start, args.total_pages)
        })
        .unwrap_or_default();
    let edit_result = artifacts::edit_header_footer_artifacts_to_temp(
        &args.input_path,
        &artifacts::HeaderFooterArtifactEditPlan {
            remove_header: args.cleanup.header_enabled,
            remove_footer: args.cleanup.footer_enabled,
            header_texts,
            footer_texts,
            header_targets: args
                .cleanup
                .artifact_header_targets
                .iter()
                .map(artifact_target_from_config)
                .collect(),
            footer_targets: args
                .cleanup
                .artifact_footer_targets
                .iter()
                .map(artifact_target_from_config)
                .collect(),
        },
    )?;
    Ok(
        edit_result.map(|(path, result)| StandardArtifactProcessingResult {
            path,
            removed_header: result.removed_header,
            removed_footer: result.removed_footer,
            edited_header: result.edited_header,
            edited_footer: result.edited_footer,
            removed_header_pages: result.removed_header_pages,
            removed_footer_pages: result.removed_footer_pages,
        }),
    )
}

fn artifact_target_from_config(
    target: &ArtifactCleanupTargetConfig,
) -> artifacts::HeaderFooterArtifactEditTarget {
    artifacts::HeaderFooterArtifactEditTarget {
        artifact_id: target.artifact_id.clone(),
        normalized_text: target.normalized_text.clone(),
        page_start: target.page_start.max(1),
        page_end: target.page_end.max(target.page_start.max(1)),
        docsy_kind: target.docsy_kind.clone(),
        replacement_text: target.replacement_text.clone(),
    }
}

fn combined_extra_overlays(
    args: &HeaderFooterJob,
    semantic_rebuild_overlays: &[OverlayTextConfig],
) -> Vec<OverlayTextConfig> {
    let mut overlays =
        Vec::with_capacity(args.extra_overlays.len() + semantic_rebuild_overlays.len());
    overlays.extend(args.extra_overlays.iter().cloned());
    overlays.extend(semantic_rebuild_overlays.iter().cloned());
    overlays
}

fn artifact_rebuild_overlays(
    args: &HeaderFooterJob,
    result: Option<&StandardArtifactProcessingResult>,
) -> Vec<OverlayTextConfig> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut overlays = Vec::new();
    if !args.cleanup.force_delete_header {
        if let Some(config) = args.cleanup.header_replacement.as_ref() {
            overlays.extend(artifact_rebuild_overlays_for_region(
                config,
                "header",
                &result.removed_header_pages,
            ));
        }
    }
    if !args.cleanup.force_delete_footer {
        if let Some(config) = args.cleanup.footer_replacement.as_ref() {
            overlays.extend(artifact_rebuild_overlays_for_region(
                config,
                "footer",
                &result.removed_footer_pages,
            ));
        }
    }
    overlays
}

fn artifact_rebuild_overlays_for_region(
    config: &OverlayTextConfig,
    region: &str,
    zero_based_pages: &BTreeSet<usize>,
) -> Vec<OverlayTextConfig> {
    contiguous_page_ranges(zero_based_pages)
        .into_iter()
        .map(|(start, end)| {
            let mut overlay = config.clone();
            overlay.region = region.to_string();
            overlay.page_start = Some(start);
            overlay.page_end = Some(end);
            overlay
        })
        .collect()
}

fn contiguous_page_ranges(zero_based_pages: &BTreeSet<usize>) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let mut start: Option<u32> = None;
    let mut previous: Option<u32> = None;
    for page in zero_based_pages {
        let page = (*page as u32).saturating_add(1);
        match (start, previous) {
            (Some(range_start), Some(prev)) if page == prev + 1 => {
                start = Some(range_start);
                previous = Some(page);
            }
            (Some(range_start), Some(prev)) => {
                ranges.push((range_start, prev));
                start = Some(page);
                previous = Some(page);
            }
            _ => {
                start = Some(page);
                previous = Some(page);
            }
        }
    }
    if let (Some(range_start), Some(prev)) = (start, previous) {
        ranges.push((range_start, prev));
    }
    ranges
}

fn standard_artifact_processing_warnings(
    args: &HeaderFooterJob,
    result: Option<&StandardArtifactProcessingResult>,
) -> Vec<String> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    if args.cleanup.header_replacement.is_some()
        && !args.cleanup.force_delete_header
        && result.removed_header > 0
    {
        warnings.push("部分标准页眉无法原位编辑，已删除后按当前页眉设置重建".to_string());
    }
    if args.cleanup.footer_replacement.is_some()
        && !args.cleanup.force_delete_footer
        && result.removed_footer > 0
    {
        warnings.push("部分标准页脚无法原位编辑，已删除后按当前页脚设置重建".to_string());
    }
    warnings
}

fn plain_text_processing_warnings(
    args: &HeaderFooterJob,
    result: Option<&content_text::PlainTextCleanupResult>,
) -> Vec<String> {
    let Some(result) = result else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    if result.removed_header > 0
        && (args.header.is_some() || args.cleanup.header_replacement.is_some())
    {
        warnings.push("普通文本型页眉已转换为 Docsy 页眉，原字体格式无法无损保留".to_string());
    }
    if result.removed_footer > 0
        && (args.footer.is_some() || args.cleanup.footer_replacement.is_some())
    {
        warnings.push("普通文本型页脚已转换为 Docsy 页脚，原字体格式无法无损保留".to_string());
    }
    // 诊断信息：当 bbox 匹配被使用时，告知用户
    let bbox_used = result
        .diagnostics
        .iter()
        .filter(|d| d.reason == content_text::DeleteSkipReason::FontUndecodable)
        .count();
    if bbox_used > 0 {
        warnings.push(format!(
            "有 {} 处页眉页脚因字体编码问题无法直接删除，已通过位置匹配（bbox）自动处理",
            bbox_used
        ));
    }
    warnings
}

fn artifact_replacement_texts(
    config: &OverlayTextConfig,
    page_count: usize,
    page_start: u32,
    total_pages: Option<u32>,
) -> Vec<String> {
    let page_start = page_start.max(1);
    let total_pages = total_pages.unwrap_or(page_count as u32);
    (0..page_count)
        .map(|index| expand_config_placeholders(config, page_start + index as u32, total_pages))
        .collect()
}

// Overlay PDF 构建已拆分到 overlay_pdf.rs，此处引入所需函数。
use super::overlay_pdf::{build_overlay_pdf, overlay_uses_page_placeholders};

// Overlay 字体与文本算子已拆分到 overlay_font.rs，此处引入所需函数和类型。
use super::overlay_font::expand_config_placeholders;

fn cleanup_temp(path: Option<PathBuf>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

fn cleanup_semantic_temp(result: Option<StandardArtifactProcessingResult>) {
    if let Some(result) = result {
        let _ = fs::remove_file(result.path);
    }
}

fn cleanup_plain_text_temp(result: Option<(PathBuf, content_text::PlainTextCleanupResult)>) {
    if let Some((path, _)) = result {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn artifact_rebuild_overlays_keep_only_removed_page_ranges() {
        let config = OverlayTextConfig {
            text: "替代页眉".to_string(),
            region: "header".to_string(),
            font_family: "songti".to_string(),
            font_size: 10.0,
            margin_mm: 10.0,
            align: "right".to_string(),
            offset_x_mm: 0.0,
            color: "#000000".to_string(),
            page_start: None,
            page_end: None,
            number_style: String::new(),
            number_offset: 0,
            number_total: None,
            artifact_kind: "HeaderText".to_string(),
        };
        let pages = BTreeSet::from([0_usize, 2, 3]);

        let overlays = artifact_rebuild_overlays_for_region(&config, "header", &pages);

        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0].page_start, Some(1));
        assert_eq!(overlays[0].page_end, Some(1));
        assert_eq!(overlays[1].page_start, Some(3));
        assert_eq!(overlays[1].page_end, Some(4));
    }

    #[test]
    fn processing_cjk_header_does_not_embed_full_font() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let input = temp_named_path("docsy_hf_process_input", "pdf");
        let output = temp_named_path("docsy_hf_process_output", "pdf");
        let deleted = temp_named_path("docsy_hf_process_deleted", "pdf");
        create_simple_test_pdf(&input);

        let result = process_job(
            &HeaderFooterJob {
                input_path: input.to_string_lossy().to_string(),
                output_path: output.to_string_lossy().to_string(),
                page_start: 1,
                total_pages: Some(1),
                normalize_a4: false,
                a4_orientation: default_a4_orientation(),
                raster_dpi: default_raster_dpi(),
                cleanup: CleanupConfig::default(),
                header: Some(OverlayTextConfig {
                    text: "测试页眉3".to_string(),
                    region: "header".to_string(),
                    font_family: "songti".to_string(),
                    font_size: 10.0,
                    margin_mm: 10.0,
                    align: "right".to_string(),
                    offset_x_mm: 0.0,
                    color: "#000000".to_string(),
                    page_start: None,
                    page_end: None,
                    number_style: String::new(),
                    number_offset: 0,
                    number_total: None,
                    artifact_kind: "HeaderText".to_string(),
                }),
                footer: None,
                extra_overlays: Vec::new(),
                bookmarks: Vec::new(),
                bookmark_remove_existing: false,
            },
            None,
        )
        .unwrap();

        let output_size = fs::metadata(&result.output_path).unwrap().len();
        assert!(
            output_size < 1_000_000,
            "processed PDF too large: {}",
            output_size
        );
        let processed = Document::load(&result.output_path).unwrap();
        assert!(processed
            .objects
            .values()
            .all(|object| !format!("{object:?}").contains("STSong-Light")));
        let check =
            crate::external::hidden_command(crate::external::QpdfTool.binary_path().unwrap())
                .arg("--check")
                .arg(&result.output_path)
                .output()
                .unwrap();
        assert!(
            check.status.success(),
            "{}",
            String::from_utf8_lossy(&check.stderr)
        );
        if let Ok(text_output) = crate::external::hidden_command("pdftotext")
            .arg(&result.output_path)
            .arg("-")
            .output()
        {
            if text_output.status.success() {
                let extracted = String::from_utf8_lossy(&text_output.stdout);
                assert!(
                    extracted.contains("测试页眉3"),
                    "processed PDF text missing inserted header: {}",
                    extracted
                );
            }
        }
        let deleted_result = artifacts::delete_header_footer_artifacts_file(
            &result.output_path,
            &deleted.to_string_lossy(),
            artifacts::HeaderFooterArtifactTargets {
                header: true,
                footer: false,
            },
        )
        .unwrap();
        assert!(deleted_result.removed_header > 0);
        if let Ok(text_output) = crate::external::hidden_command("pdftotext")
            .arg(&deleted)
            .arg("-")
            .output()
        {
            if text_output.status.success() {
                let extracted = String::from_utf8_lossy(&text_output.stdout);
                assert!(!extracted.contains("测试页眉3"));
            }
        }
        let _ = fs::remove_file(input);
        let _ = fs::remove_file(result.output_path);
        let _ = fs::remove_file(output);
        let _ = fs::remove_file(deleted);
    }

    #[test]
    fn confirmed_plain_header_edit_rebuilds_as_standard_artifact() {
        if crate::external::QpdfTool.binary_path().is_err() {
            return;
        }
        let input = temp_named_path("docsy_plain_header_input", "pdf");
        let output = temp_named_path("docsy_plain_header_output", "pdf");
        create_plain_header_test_pdf(&input, "Legacy Header");

        let result = process_job(
            &HeaderFooterJob {
                input_path: input.to_string_lossy().to_string(),
                output_path: output.to_string_lossy().to_string(),
                page_start: 1,
                total_pages: Some(1),
                normalize_a4: false,
                a4_orientation: default_a4_orientation(),
                raster_dpi: default_raster_dpi(),
                cleanup: CleanupConfig {
                    header_enabled: true,
                    header_height_mm: 25.0,
                    plain_header_targets: vec![PlainTextCleanupTargetConfig {
                        text: "Legacy Header".to_string(),
                        normalized_text: "LegacyHeader".to_string(),
                        page_start: 1,
                        page_end: 1,
                        bbox: Some(PlainTextCleanupBBoxConfig {
                            x0: 75.0,
                            y0: 20.0,
                            x1: 170.0,
                            y1: 40.0,
                            page: 1,
                            width: 595.0,
                            height: 842.0,
                        }),
                    }],
                    ..CleanupConfig::default()
                },
                header: None,
                footer: None,
                extra_overlays: vec![OverlayTextConfig {
                    text: "Updated Header".to_string(),
                    region: "header".to_string(),
                    font_family: "auto".to_string(),
                    font_size: 12.0,
                    margin_mm: 10.0,
                    align: "left".to_string(),
                    offset_x_mm: 10.0,
                    color: "#000000".to_string(),
                    page_start: Some(1),
                    page_end: Some(1),
                    number_style: String::new(),
                    number_offset: 0,
                    number_total: None,
                    artifact_kind: "HeaderText".to_string(),
                }],
                bookmarks: Vec::new(),
                bookmark_remove_existing: false,
            },
            None,
        )
        .unwrap();

        let inspection = artifacts::inspect_meaningful_header_footer_artifacts(
            Path::new(&result.output_path),
            1,
        )
        .unwrap();
        assert!(inspection.occurrences.iter().any(|occurrence| {
            occurrence.region == "header"
                && occurrence.text.as_deref() == Some("Updated Header")
                && occurrence.docsy_kind.as_deref() == Some("HeaderText")
        }));

        if let Ok(text_output) = crate::external::hidden_command("pdftotext")
            .arg(&result.output_path)
            .arg("-")
            .output()
        {
            if text_output.status.success() {
                let extracted = String::from_utf8_lossy(&text_output.stdout);
                assert!(!extracted.contains("Legacy Header"));
                assert!(extracted.contains("Updated Header"));
            }
        }

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(result.output_path);
        let _ = fs::remove_file(output);
    }

    fn create_simple_test_pdf(path: &Path) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new("Td", vec![80.into(), 500.into()]),
                Operation::new("Tj", vec![Object::string_literal("body text")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }

    fn create_plain_header_test_pdf(path: &Path, header: &str) {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new(
                    "Tm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        1.into(),
                        80.into(),
                        812.into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal(header)]),
                Operation::new("ET", vec![]),
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 12.into()]),
                Operation::new(
                    "Tm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        1.into(),
                        80.into(),
                        500.into(),
                    ],
                ),
                Operation::new("Tj", vec![Object::string_literal("body text")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }
}

#[test]
fn bookmark_serde_camelcase() {
    let json = r#"{"inputPath":"/tmp/a.pdf","outputPath":"/tmp/b.pdf","bookmarks":[{"enabled":true,"label":"测试","pageIndex":0}],"bookmarkRemoveExisting":false}"#;
    let job: HeaderFooterJob = serde_json::from_str(json).unwrap();
    assert_eq!(job.bookmarks.len(), 1, "bookmarks should have 1 item");
    assert_eq!(job.bookmarks[0].label, "测试");
    assert_eq!(job.bookmarks[0].page_index, 0);
    assert!(!job.bookmark_remove_existing);
}
