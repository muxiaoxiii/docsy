use anyhow::{Context, Result};
use image::{imageops::FilterType, DynamicImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, LazyLock, Mutex};
use std::time::UNIX_EPOCH;

use super::opencv_enhancer;

const ANALYSIS_WIDTH: u32 = 96;
const ANALYSIS_HEIGHT: u32 = 96;
const BLOCKS_X: usize = 12;
const BLOCKS_Y: usize = 12;
const MAX_ANCHOR_DISTANCE: usize = 2;
const MAX_FEATURE_CACHE_ITEMS: usize = 2048;
const MAX_PAIR_CACHE_ITEMS: usize = 8192;
const ALGORITHM_VERSION: &str = "rust-multisignal-v2-dynamic-local";

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FrameSelectionArgs {
    pub paths: Vec<String>,
    pub duplicate_threshold: f64,
    pub scene_threshold: f64,
    pub overlap_min: f64,
    pub overlap_max: f64,
    pub user_decisions: HashMap<String, String>,
}

impl Default for FrameSelectionArgs {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            duplicate_threshold: 0.965,
            scene_threshold: 0.55,
            overlap_min: 0.05,
            overlap_max: 0.10,
            user_decisions: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FrameSelectionResult {
    pub algorithm_version: String,
    pub items: Vec<FrameDecision>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FrameDecision {
    pub path: String,
    pub engine_decision: String,
    pub relation: String,
    pub reason: String,
    pub compared_to: Option<usize>,
    pub confidence: f64,
    pub similarity: f64,
    pub overlap_ratio: Option<f64>,
    pub shift_x: i32,
    pub shift_y: i32,
    pub blur_score: f64,
    pub change_ratio: f64,
    pub continuity_risk: bool,
    pub enhanced_by: Option<String>,
    pub opencv_match_ratio: Option<f64>,
    pub opencv_inlier_ratio: Option<f64>,
}

#[derive(Clone)]
struct FrameFeatures {
    path: String,
    luma: Vec<u8>,
    color_blocks: Vec<[f64; 3]>,
    difference_hash: u64,
    blur_score: f64,
    texture_score: f64,
    cache_stamp: u128,
}

#[derive(Clone)]
struct FeatureCacheEntry {
    size: u64,
    modified_millis: u128,
    features: FrameFeatures,
}

static FEATURE_CACHE: LazyLock<Mutex<HashMap<String, FeatureCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
struct PairMetrics {
    relation: &'static str,
    confidence: f64,
    similarity: f64,
    overlap_ratio: Option<f64>,
    shift_x: i32,
    shift_y: i32,
    change_ratio: f64,
    enhanced_by: Option<&'static str>,
    opencv_match_ratio: Option<f64>,
    opencv_inlier_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PairCacheKey {
    previous_path: String,
    previous_stamp: u128,
    current_path: String,
    current_stamp: u128,
    duplicate_threshold: u64,
    scene_threshold: u64,
}

static PAIR_CACHE: LazyLock<Mutex<HashMap<PairCacheKey, PairMetrics>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy)]
struct ShiftMatch {
    score: f64,
    overlap_ratio: f64,
    shift_x: i32,
    shift_y: i32,
}

#[derive(Debug, Clone, Copy)]
struct SelectionThresholds {
    duplicate: f64,
    scene: f64,
    overlap_min: f64,
    overlap_max: f64,
}

#[derive(Debug, Clone, Copy)]
struct AdaptiveThresholds {
    duplicate: f64,
    overlap_max: f64,
    base_duplicate: f64,
    base_overlap_max: f64,
}

impl AdaptiveThresholds {
    fn new(duplicate: f64, overlap_max: f64) -> Self {
        Self {
            duplicate,
            overlap_max,
            base_duplicate: duplicate,
            base_overlap_max: overlap_max,
        }
    }

    fn has_feedback_adjustment(self) -> bool {
        (self.duplicate - self.base_duplicate).abs() > f64::EPSILON
            || (self.overlap_max - self.base_overlap_max).abs() > f64::EPSILON
    }

    fn observe(
        &mut self,
        relation: &str,
        engine_decision: &str,
        user_decision: Option<&str>,
        overlap_min: f64,
    ) {
        match (relation, engine_decision, user_decision) {
            ("duplicate", "exclude", Some("keep")) => {
                self.duplicate = (self.duplicate + 0.006).min(self.base_duplicate + 0.025);
            }
            ("duplicate", "keep", Some("exclude")) => {
                self.duplicate = (self.duplicate - 0.004).max(self.base_duplicate - 0.015);
            }
            ("continuous_vertical" | "continuous_horizontal", "exclude", Some("keep")) => {
                self.overlap_max = (self.overlap_max + 0.015).min(self.base_overlap_max + 0.05);
            }
            ("continuous_vertical" | "continuous_horizontal", "keep", Some("exclude")) => {
                self.overlap_max = (self.overlap_max - 0.01)
                    .max(overlap_min)
                    .max(self.base_overlap_max - 0.04);
            }
            _ => {}
        }
    }
}

pub fn analyze_with_progress<C, P>(
    args: FrameSelectionArgs,
    is_cancelled: C,
    mut progress: P,
) -> Result<FrameSelectionResult>
where
    C: Fn() -> bool + Sync,
    P: FnMut(&'static str, usize, usize),
{
    let overlap_min = args.overlap_min.clamp(0.02, 0.80);
    let overlap_max = args.overlap_max.clamp(overlap_min, 0.90);
    let duplicate_threshold = args.duplicate_threshold.clamp(0.80, 0.999);
    let scene_threshold = args.scene_threshold.clamp(0.20, 0.80);
    let mut warnings = Vec::new();
    let mut frames = Vec::new();
    let paths = args.paths;
    let input_count = paths.len();
    progress("features", 0, input_count);

    let worker_count = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2)
        .clamp(1, 6)
        .min(input_count.max(1));
    let next_index = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    let mut feature_results = std::iter::repeat_with(|| None)
        .take(input_count)
        .collect::<Vec<Option<std::result::Result<FrameFeatures, String>>>>();
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let sender = sender.clone();
            let paths = &paths;
            let next_index = &next_index;
            let is_cancelled = &is_cancelled;
            scope.spawn(move || loop {
                if is_cancelled() {
                    break;
                }
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                if index >= paths.len() {
                    break;
                }
                let result = load_features_cached(&paths[index]).map_err(|error| error.to_string());
                if sender.send((index, result)).is_err() {
                    break;
                }
            });
        }
        drop(sender);
        let mut completed = 0_usize;
        for (index, result) in receiver {
            feature_results[index] = Some(result);
            completed += 1;
            if completed == 1 || completed == input_count || completed.is_multiple_of(5) {
                progress("features", completed, input_count);
            }
        }
    });
    if is_cancelled() {
        anyhow::bail!("智能筛选已取消");
    }
    for (index, result) in feature_results.into_iter().enumerate() {
        match result {
            Some(Ok(features)) => frames.push(features),
            Some(Err(error)) => warnings.push(format!("{}：{}", paths[index], error)),
            None => warnings.push(format!("{}：未完成图片特征读取", paths[index])),
        }
    }
    let decisions = Vec::with_capacity(frames.len());
    if frames.is_empty() {
        return Ok(FrameSelectionResult {
            algorithm_version: algorithm_version(),
            items: decisions,
            warnings,
        });
    }
    let comparison_count = frames.len().saturating_sub(1);
    progress("compare", 0, comparison_count);
    let decisions = decide_sequence(
        &frames,
        SelectionThresholds {
            duplicate: duplicate_threshold,
            scene: scene_threshold,
            overlap_min,
            overlap_max,
        },
        &args.user_decisions,
        |index| {
            if index == 1 || index == comparison_count || index % 5 == 0 {
                progress("compare", index, comparison_count);
            }
        },
        &is_cancelled,
    )?;
    progress("finalize", frames.len(), frames.len());
    Ok(FrameSelectionResult {
        algorithm_version: algorithm_version(),
        items: decisions,
        warnings,
    })
}

fn decide_sequence<P, C>(
    frames: &[FrameFeatures],
    thresholds: SelectionThresholds,
    user_decisions: &HashMap<String, String>,
    mut progress: P,
    is_cancelled: C,
) -> Result<Vec<FrameDecision>>
where
    P: FnMut(usize),
    C: Fn() -> bool,
{
    if frames.is_empty() {
        return Ok(Vec::new());
    }
    let mut decisions = Vec::with_capacity(frames.len());
    decisions.push(FrameDecision {
        path: frames[0].path.clone(),
        engine_decision: "keep".into(),
        relation: "first".into(),
        reason: "第一张图片作为当前序列的起点保留".into(),
        compared_to: None,
        confidence: 1.0,
        similarity: 0.0,
        overlap_ratio: None,
        shift_x: 0,
        shift_y: 0,
        blur_score: frames[0].blur_score,
        change_ratio: 0.0,
        continuity_risk: false,
        enhanced_by: None,
        opencv_match_ratio: None,
        opencv_inlier_ratio: None,
    });
    let mut last_anchor = if effective_decision(&frames[0].path, "keep", user_decisions) == "keep" {
        Some(0_usize)
    } else {
        None
    };
    let mut adaptive = AdaptiveThresholds::new(thresholds.duplicate, thresholds.overlap_max);
    for index in 1..frames.len() {
        if is_cancelled() {
            anyhow::bail!("智能筛选已取消");
        }
        let anchor_is_local = last_anchor
            .map(|anchor| index.saturating_sub(anchor) <= MAX_ANCHOR_DISTANCE)
            .unwrap_or(false);
        let compared_to = if anchor_is_local {
            last_anchor.unwrap_or(index - 1)
        } else {
            index - 1
        };
        let metrics = compare_frames(
            &frames[compared_to],
            &frames[index],
            adaptive.duplicate,
            thresholds.scene,
        );
        let mut engine_decision = "keep";
        let mut continuity_risk = false;
        let current_user_decision = user_decisions.get(&frames[index].path).map(String::as_str);
        let compared_user_decision = user_decisions
            .get(&frames[compared_to].path)
            .map(String::as_str);
        let reason = if !anchor_is_local {
            format!(
                "与上次保留图距离过远，改为只比较相邻第{}张并建议保留，避免跨帧误判",
                compared_to + 1
            )
        } else {
            let anchor_note = if compared_user_decision == Some("keep") {
                "依据你的保留决定，"
            } else if adaptive.has_feedback_adjustment() {
                "已根据前面的人工选择调整判断，"
            } else {
                ""
            };
            match metrics.relation {
                "duplicate" => {
                    if frames[index].blur_score > frames[compared_to].blur_score + 0.08
                        && current_user_decision != Some("exclude")
                        && compared_user_decision != Some("keep")
                    {
                        decisions[compared_to].engine_decision = "exclude".into();
                        decisions[compared_to].reason = format!(
                            "与后续第{}张高度重复，后图清晰度更高，建议改为保留后图",
                            index + 1
                        );
                        format!("{anchor_note}与比较图高度重复，但当前图片更清晰，建议保留当前图片")
                    } else {
                        engine_decision = "exclude";
                        format!(
                            "{anchor_note}与第{}张高度重复，相似度{}%，建议排除当前图片",
                            compared_to + 1,
                            percent(metrics.similarity)
                        )
                    }
                }
                "continuous_vertical" | "continuous_horizontal" => {
                    let overlap = metrics.overlap_ratio.unwrap_or(1.0);
                    if overlap > adaptive.overlap_max + 0.025 {
                        engine_decision = "exclude";
                        format!(
                            "检测到连续{}，与第{}张重合{}%，高于目标{}%-{}%，建议跳过",
                            if metrics.relation == "continuous_vertical" {
                                "纵向画面"
                            } else {
                                "横向画面"
                            },
                            compared_to + 1,
                            percent(overlap),
                            percent(thresholds.overlap_min),
                            percent(adaptive.overlap_max)
                        )
                    } else if overlap + 0.015 < thresholds.overlap_min {
                        continuity_risk = true;
                        format!(
                        "{anchor_note}检测到连续{}，但重合仅{}%，可能存在内容缺口，保留并提示核对",
                        if metrics.relation == "continuous_vertical" {
                            "纵向画面"
                        } else {
                            "横向画面"
                        },
                        percent(overlap)
                    )
                    } else {
                        format!(
                            "{anchor_note}检测到连续{}，重合{}%，处于目标{}%-{}%附近，建议保留",
                            if metrics.relation == "continuous_vertical" {
                                "纵向画面"
                            } else {
                                "横向画面"
                            },
                            percent(overlap),
                            percent(thresholds.overlap_min),
                            percent(adaptive.overlap_max)
                        )
                    }
                }
                "scene_cut" => {
                    format!("{anchor_note}检测到场景或关键画面切换，前后图片均保留")
                }
                "changed" => {
                    format!(
                        "{anchor_note}主体画面相近但约{}%区域发生变化，为避免遗漏关键内容予以保留",
                        percent(metrics.change_ratio)
                    )
                }
                _ => {
                    format!("{anchor_note}无法可靠判断与比较图关系，按安全原则保留并等待用户确认")
                }
            }
        };
        decisions.push(FrameDecision {
            path: frames[index].path.clone(),
            engine_decision: engine_decision.into(),
            relation: metrics.relation.into(),
            reason,
            compared_to: Some(compared_to),
            confidence: metrics.confidence,
            similarity: metrics.similarity,
            overlap_ratio: metrics.overlap_ratio,
            shift_x: metrics.shift_x,
            shift_y: metrics.shift_y,
            blur_score: frames[index].blur_score,
            change_ratio: metrics.change_ratio,
            continuity_risk,
            enhanced_by: metrics.enhanced_by.map(str::to_string),
            opencv_match_ratio: metrics.opencv_match_ratio,
            opencv_inlier_ratio: metrics.opencv_inlier_ratio,
        });
        if effective_decision(&frames[index].path, engine_decision, user_decisions) == "keep" {
            last_anchor = Some(index);
        }
        adaptive.observe(
            metrics.relation,
            engine_decision,
            current_user_decision,
            thresholds.overlap_min,
        );
        progress(index);
    }
    Ok(decisions)
}

fn effective_decision<'a>(
    path: &str,
    engine_decision: &'a str,
    user_decisions: &'a HashMap<String, String>,
) -> &'a str {
    user_decisions
        .get(path)
        .map(String::as_str)
        .filter(|decision| matches!(*decision, "keep" | "exclude"))
        .unwrap_or(engine_decision)
}

fn algorithm_version() -> String {
    if opencv_enhancer::is_available() {
        format!("{ALGORITHM_VERSION}+opencv-4.13-phase")
    } else {
        ALGORITHM_VERSION.into()
    }
}

fn load_features_cached(path: &str) -> Result<FrameFeatures> {
    let metadata = std::fs::metadata(path).with_context(|| "无法读取图片信息")?;
    let size = metadata.len();
    let modified_millis = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or(0);
    if let Ok(cache) = FEATURE_CACHE.lock() {
        if let Some(entry) = cache.get(path) {
            if entry.size == size && entry.modified_millis == modified_millis {
                return Ok(entry.features.clone());
            }
        }
    }
    let mut features = load_features(path)?;
    features.cache_stamp = ((size as u128) << 64) ^ modified_millis;
    if let Ok(mut cache) = FEATURE_CACHE.lock() {
        if cache.len() >= MAX_FEATURE_CACHE_ITEMS && !cache.contains_key(path) {
            cache.clear();
        }
        cache.insert(
            path.to_string(),
            FeatureCacheEntry {
                size,
                modified_millis,
                features: features.clone(),
            },
        );
    }
    Ok(features)
}

fn load_features(path: &str) -> Result<FrameFeatures> {
    let image = image::open(path).with_context(|| "无法解码图片")?;
    let reduced = image.resize_exact(ANALYSIS_WIDTH, ANALYSIS_HEIGHT, FilterType::Triangle);
    let rgb = reduced.to_rgb8();
    let luma = DynamicImage::ImageRgb8(rgb.clone()).to_luma8().into_raw();
    let color_blocks = build_color_blocks(rgb.as_raw());
    let difference_hash = difference_hash(&DynamicImage::ImageRgb8(rgb.clone()));
    let blur_score = laplacian_variance(&luma);
    let texture_score = gradient_texture(&luma);
    Ok(FrameFeatures {
        path: path.to_string(),
        luma,
        color_blocks,
        difference_hash,
        blur_score,
        texture_score,
        cache_stamp: 0,
    })
}

fn compare_frames(
    previous: &FrameFeatures,
    current: &FrameFeatures,
    duplicate_threshold: f64,
    scene_threshold: f64,
) -> PairMetrics {
    let key = PairCacheKey {
        previous_path: previous.path.clone(),
        previous_stamp: previous.cache_stamp,
        current_path: current.path.clone(),
        current_stamp: current.cache_stamp,
        duplicate_threshold: duplicate_threshold.to_bits(),
        scene_threshold: scene_threshold.to_bits(),
    };
    let cacheable = previous.cache_stamp != 0 && current.cache_stamp != 0;
    if cacheable {
        if let Ok(cache) = PAIR_CACHE.lock() {
            if let Some(metrics) = cache.get(&key) {
                return metrics.clone();
            }
        }
    }
    let metrics = compare_frames_uncached(previous, current, duplicate_threshold, scene_threshold);
    if cacheable {
        if let Ok(mut cache) = PAIR_CACHE.lock() {
            if cache.len() >= MAX_PAIR_CACHE_ITEMS && !cache.contains_key(&key) {
                cache.clear();
            }
            cache.insert(key, metrics.clone());
        }
    }
    metrics
}

fn compare_frames_uncached(
    previous: &FrameFeatures,
    current: &FrameFeatures,
    duplicate_threshold: f64,
    scene_threshold: f64,
) -> PairMetrics {
    let mut metrics = compare_frames_rust(previous, current, duplicate_threshold, scene_threshold);
    let has_orb_texture = previous.texture_score.min(current.texture_score) >= 0.10;
    let needs_enhanced_review = match metrics.relation {
        "uncertain" => metrics.similarity >= 0.60 && has_orb_texture,
        "changed" => metrics.confidence < 0.80 && has_orb_texture,
        "scene_cut" => metrics.similarity >= scene_threshold - 0.04 && has_orb_texture,
        "continuous_vertical" | "continuous_horizontal" => {
            metrics.confidence < 0.88 && has_orb_texture
        }
        _ => false,
    };
    if needs_enhanced_review {
        if let Some(enhanced) = opencv_enhancer::compare_luma(
            &previous.luma,
            &current.luma,
            ANALYSIS_WIDTH,
            ANALYSIS_HEIGHT,
        ) {
            metrics.enhanced_by = Some("OpenCV 相位相关");
            metrics.opencv_match_ratio = Some(enhanced.match_ratio);
            metrics.opencv_inlier_ratio = Some(enhanced.inlier_ratio);
            let shift_x = enhanced.shift_x.round() as i32;
            let shift_y = enhanced.shift_y.round() as i32;
            let has_consistent_motion =
                enhanced.confidence >= 0.24 && (shift_x.abs() >= 4 || shift_y.abs() >= 4);
            if has_consistent_motion {
                let vertical = shift_y.abs() >= shift_x.abs();
                metrics.relation = if vertical {
                    "continuous_vertical"
                } else {
                    "continuous_horizontal"
                };
                metrics.confidence = metrics.confidence.max(enhanced.confidence);
                metrics.overlap_ratio = Some(
                    if vertical {
                        1.0 - shift_y.abs() as f64 / ANALYSIS_HEIGHT as f64
                    } else {
                        1.0 - shift_x.abs() as f64 / ANALYSIS_WIDTH as f64
                    }
                    .clamp(0.0, 1.0),
                );
                metrics.shift_x = shift_x;
                metrics.shift_y = shift_y;
            }
        }
    }
    metrics
}

fn compare_frames_rust(
    previous: &FrameFeatures,
    current: &FrameFeatures,
    duplicate_threshold: f64,
    scene_threshold: f64,
) -> PairMetrics {
    let hash_similarity =
        1.0 - (previous.difference_hash ^ current.difference_hash).count_ones() as f64 / 64.0;
    let pixel_similarity = similarity_bytes(&previous.luma, &current.luma);
    let color_similarity = color_similarity(&previous.color_blocks, &current.color_blocks);
    let similarity = (hash_similarity * 0.34 + pixel_similarity * 0.36 + color_similarity * 0.30)
        .clamp(0.0, 1.0);
    let change_ratio = changed_block_ratio(&previous.color_blocks, &current.color_blocks);
    if similarity >= duplicate_threshold && change_ratio <= 0.08 {
        return PairMetrics {
            relation: "duplicate",
            confidence: similarity,
            similarity,
            overlap_ratio: Some(1.0),
            shift_x: 0,
            shift_y: 0,
            change_ratio,
            enhanced_by: None,
            opencv_match_ratio: None,
            opencv_inlier_ratio: None,
        };
    }
    let shift = best_shift(&previous.luma, &current.luma);
    let texture = previous.texture_score.min(current.texture_score);
    let shift_confidence = (shift.score * (0.72 + texture * 0.28)).clamp(0.0, 1.0);
    if shift_confidence >= 0.86 && (shift.shift_x.abs() >= 4 || shift.shift_y.abs() >= 4) {
        return PairMetrics {
            relation: if shift.shift_y.abs() >= shift.shift_x.abs() {
                "continuous_vertical"
            } else {
                "continuous_horizontal"
            },
            confidence: shift_confidence,
            similarity,
            overlap_ratio: Some(shift.overlap_ratio),
            shift_x: shift.shift_x,
            shift_y: shift.shift_y,
            change_ratio,
            enhanced_by: None,
            opencv_match_ratio: None,
            opencv_inlier_ratio: None,
        };
    }
    if similarity < scene_threshold && shift_confidence < scene_threshold + 0.08 {
        return PairMetrics {
            relation: "scene_cut",
            confidence: (1.0 - similarity).clamp(0.0, 1.0),
            similarity,
            overlap_ratio: None,
            shift_x: 0,
            shift_y: 0,
            change_ratio,
            enhanced_by: None,
            opencv_match_ratio: None,
            opencv_inlier_ratio: None,
        };
    }
    if similarity >= 0.62 && change_ratio >= 0.08 {
        return PairMetrics {
            relation: "changed",
            confidence: similarity.max(change_ratio),
            similarity,
            overlap_ratio: None,
            shift_x: 0,
            shift_y: 0,
            change_ratio,
            enhanced_by: None,
            opencv_match_ratio: None,
            opencv_inlier_ratio: None,
        };
    }
    PairMetrics {
        relation: "uncertain",
        confidence: 0.45,
        similarity,
        overlap_ratio: None,
        shift_x: 0,
        shift_y: 0,
        change_ratio,
        enhanced_by: None,
        opencv_match_ratio: None,
        opencv_inlier_ratio: None,
    }
}

fn build_color_blocks(rgb: &[u8]) -> Vec<[f64; 3]> {
    let width = ANALYSIS_WIDTH as usize;
    let height = ANALYSIS_HEIGHT as usize;
    let block_width = width / BLOCKS_X;
    let block_height = height / BLOCKS_Y;
    let mut blocks = Vec::with_capacity(BLOCKS_X * BLOCKS_Y);
    for block_y in 0..BLOCKS_Y {
        for block_x in 0..BLOCKS_X {
            let mut sum = [0_u64; 3];
            let mut count = 0_u64;
            for y in block_y * block_height..(block_y + 1) * block_height {
                for x in block_x * block_width..(block_x + 1) * block_width {
                    let offset = (y * width + x) * 3;
                    sum[0] += rgb[offset] as u64;
                    sum[1] += rgb[offset + 1] as u64;
                    sum[2] += rgb[offset + 2] as u64;
                    count += 1;
                }
            }
            blocks.push([
                sum[0] as f64 / count as f64,
                sum[1] as f64 / count as f64,
                sum[2] as f64 / count as f64,
            ]);
        }
    }
    blocks
}

fn difference_hash(image: &DynamicImage) -> u64 {
    let gray = image.resize_exact(9, 8, FilterType::Triangle).to_luma8();
    let mut hash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            hash <<= 1;
            if gray.get_pixel(x, y)[0] > gray.get_pixel(x + 1, y)[0] {
                hash |= 1;
            }
        }
    }
    hash
}

fn similarity_bytes(left: &[u8], right: &[u8]) -> f64 {
    let count = left.len().min(right.len());
    if count == 0 {
        return 0.0;
    }
    let difference = left
        .iter()
        .zip(right)
        .take(count)
        .map(|(a, b)| (*a as f64 - *b as f64).abs())
        .sum::<f64>();
    (1.0 - difference / (count as f64 * 255.0)).clamp(0.0, 1.0)
}

fn color_similarity(left: &[[f64; 3]], right: &[[f64; 3]]) -> f64 {
    let count = left.len().min(right.len());
    if count == 0 {
        return 0.0;
    }
    let difference = left
        .iter()
        .zip(right)
        .take(count)
        .map(|(a, b)| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs())
        .sum::<f64>();
    (1.0 - difference / (count as f64 * 255.0 * 3.0)).clamp(0.0, 1.0)
}

fn changed_block_ratio(left: &[[f64; 3]], right: &[[f64; 3]]) -> f64 {
    let count = left.len().min(right.len());
    if count == 0 {
        return 1.0;
    }
    let changed = left
        .iter()
        .zip(right)
        .take(count)
        .filter(|(a, b)| {
            ((a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()) / 3.0 > 22.0
        })
        .count();
    changed as f64 / count as f64
}

fn laplacian_variance(luma: &[u8]) -> f64 {
    let width = ANALYSIS_WIDTH as usize;
    let height = ANALYSIS_HEIGHT as usize;
    let mut values = Vec::with_capacity((width - 2) * (height - 2));
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let center = luma[y * width + x] as f64 * -4.0;
            let value = center
                + luma[(y - 1) * width + x] as f64
                + luma[(y + 1) * width + x] as f64
                + luma[y * width + x - 1] as f64
                + luma[y * width + x + 1] as f64;
            values.push(value);
        }
    }
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    (variance / 1800.0).clamp(0.0, 1.0)
}

fn gradient_texture(luma: &[u8]) -> f64 {
    let width = ANALYSIS_WIDTH as usize;
    let height = ANALYSIS_HEIGHT as usize;
    let mut sum = 0.0;
    let mut count = 0_usize;
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let current = luma[y * width + x] as f64;
            sum += (current - luma[y * width + x + 1] as f64).abs();
            sum += (current - luma[(y + 1) * width + x] as f64).abs();
            count += 2;
        }
    }
    if count == 0 {
        0.0
    } else {
        (sum / count as f64 / 18.0).clamp(0.0, 1.0)
    }
}

fn best_shift(previous: &[u8], current: &[u8]) -> ShiftMatch {
    let width = ANALYSIS_WIDTH as usize;
    let height = ANALYSIS_HEIGHT as usize;
    let min_overlap_x = (width as f64 * 0.05).ceil() as usize;
    let min_overlap_y = (height as f64 * 0.05).ceil() as usize;
    let mut best = ShiftMatch {
        score: 0.0,
        overlap_ratio: 0.0,
        shift_x: 0,
        shift_y: 0,
    };
    for vertical in [true, false] {
        let max_shift = if vertical {
            height.saturating_sub(min_overlap_y)
        } else {
            width.saturating_sub(min_overlap_x)
        };
        for direction in [-1_i32, 1_i32] {
            let candidate = best_axis_shift(
                previous, current, width, height, vertical, direction, max_shift,
            );
            if candidate.score > best.score {
                best = candidate;
            }
        }
    }
    best
}

fn best_axis_shift(
    previous: &[u8],
    current: &[u8],
    width: usize,
    height: usize,
    vertical: bool,
    direction: i32,
    max_shift: usize,
) -> ShiftMatch {
    if max_shift < 4 {
        return ShiftMatch {
            score: 0.0,
            overlap_ratio: 0.0,
            shift_x: 0,
            shift_y: 0,
        };
    }

    // First scan the whole range on a sparse pixel grid and every third shift.
    // Then refine only around the best candidate at full resolution. This keeps
    // 5-10% overlap detection while avoiding hundreds of full 96x96 scans.
    let mut coarse_shift = 4_usize;
    let mut coarse_score = -1.0_f64;
    let mut shift = 4_usize;
    while shift <= max_shift {
        let (shift_x, shift_y) = axis_offsets(vertical, direction, shift);
        let score =
            shifted_similarity_sampled(previous, current, shift_x, shift_y, width, height, 3);
        if score > coarse_score {
            coarse_score = score;
            coarse_shift = shift;
        }
        shift = shift.saturating_add(3);
    }
    if !(max_shift - 4).is_multiple_of(3) {
        let (shift_x, shift_y) = axis_offsets(vertical, direction, max_shift);
        let score =
            shifted_similarity_sampled(previous, current, shift_x, shift_y, width, height, 3);
        if score > coarse_score {
            coarse_shift = max_shift;
        }
    }

    let start = coarse_shift.saturating_sub(3).max(4);
    let end = coarse_shift.saturating_add(3).min(max_shift);
    let mut best = ShiftMatch {
        score: 0.0,
        overlap_ratio: 0.0,
        shift_x: 0,
        shift_y: 0,
    };
    for refined_shift in start..=end {
        let (shift_x, shift_y) = axis_offsets(vertical, direction, refined_shift);
        let score = shifted_similarity(previous, current, shift_x, shift_y, width, height);
        if score > best.score {
            best = ShiftMatch {
                score,
                overlap_ratio: if vertical {
                    (height - refined_shift) as f64 / height as f64
                } else {
                    (width - refined_shift) as f64 / width as f64
                },
                shift_x,
                shift_y,
            };
        }
    }
    best
}

fn axis_offsets(vertical: bool, direction: i32, shift: usize) -> (i32, i32) {
    if vertical {
        (0, shift as i32 * direction)
    } else {
        (shift as i32 * direction, 0)
    }
}

fn shifted_similarity(
    previous: &[u8],
    current: &[u8],
    shift_x: i32,
    shift_y: i32,
    width: usize,
    height: usize,
) -> f64 {
    shifted_similarity_sampled(previous, current, shift_x, shift_y, width, height, 1)
}

fn shifted_similarity_sampled(
    previous: &[u8],
    current: &[u8],
    shift_x: i32,
    shift_y: i32,
    width: usize,
    height: usize,
    requested_step: usize,
) -> f64 {
    let (previous_x, current_x, overlap_width) = if shift_x >= 0 {
        (shift_x as usize, 0, width.saturating_sub(shift_x as usize))
    } else {
        (
            0,
            (-shift_x) as usize,
            width.saturating_sub((-shift_x) as usize),
        )
    };
    let (previous_y, current_y, overlap_height) = if shift_y >= 0 {
        (shift_y as usize, 0, height.saturating_sub(shift_y as usize))
    } else {
        (
            0,
            (-shift_y) as usize,
            height.saturating_sub((-shift_y) as usize),
        )
    };
    if overlap_width == 0 || overlap_height == 0 {
        return 0.0;
    }
    let sample_step = if overlap_width.min(overlap_height) <= 12 {
        1
    } else {
        requested_step.max(1)
    };
    let mut difference = 0.0;
    let mut count = 0_usize;
    for y in (0..overlap_height).step_by(sample_step) {
        for x in (0..overlap_width).step_by(sample_step) {
            let left = previous[(previous_y + y) * width + previous_x + x] as f64;
            let right = current[(current_y + y) * width + current_x + x] as f64;
            difference += (left - right).abs();
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    (1.0 - difference / (count as f64 * 255.0)).clamp(0.0, 1.0)
}

fn percent(value: f64) -> i64 {
    (value.clamp(0.0, 1.0) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(path: &str, luma: Vec<u8>) -> FrameFeatures {
        let rgb = luma
            .iter()
            .flat_map(|value| [*value, *value, *value])
            .collect::<Vec<_>>();
        FrameFeatures {
            path: path.into(),
            color_blocks: build_color_blocks(&rgb),
            difference_hash: 0,
            blur_score: laplacian_variance(&luma),
            texture_score: gradient_texture(&luma),
            cache_stamp: 0,
            luma,
        }
    }

    fn identical_sequence(count: usize) -> Vec<FrameFeatures> {
        let pixels = (0..ANALYSIS_WIDTH * ANALYSIS_HEIGHT)
            .map(|index| ((index * 37 + 17) % 251) as u8)
            .collect::<Vec<_>>();
        (0..count)
            .map(|index| features(&format!("frame-{index}.png"), pixels.clone()))
            .collect()
    }

    fn decide_for_test(
        frames: &[FrameFeatures],
        user_decisions: &HashMap<String, String>,
    ) -> Vec<FrameDecision> {
        decide_sequence(
            frames,
            SelectionThresholds {
                duplicate: 0.965,
                scene: 0.55,
                overlap_min: 0.05,
                overlap_max: 0.10,
            },
            user_decisions,
            |_| {},
            || false,
        )
        .unwrap()
    }

    #[test]
    fn identical_bytes_are_fully_similar() {
        assert_eq!(similarity_bytes(&[1, 2, 3], &[1, 2, 3]), 1.0);
    }

    #[test]
    fn shifted_similarity_finds_exact_vertical_overlap() {
        let width = 8;
        let height = 8;
        let previous = (0..width * height)
            .map(|value| value as u8)
            .collect::<Vec<_>>();
        let mut current = vec![0_u8; width * height];
        for y in 0..6 {
            for x in 0..width {
                current[y * width + x] = previous[(y + 2) * width + x];
            }
        }
        assert_eq!(
            shifted_similarity(&previous, &current, 0, 2, width, height),
            1.0
        );
    }

    #[test]
    fn multi_signal_gate_classifies_duplicates() {
        let pixels = (0..ANALYSIS_WIDTH * ANALYSIS_HEIGHT)
            .map(|index| ((index * 37 + 17) % 251) as u8)
            .collect::<Vec<_>>();
        let previous = features("a.png", pixels.clone());
        let current = features("b.png", pixels);
        let metrics = compare_frames(&previous, &current, 0.965, 0.55);
        assert_eq!(metrics.relation, "duplicate");
        assert!(metrics.confidence > 0.99);
    }

    #[test]
    fn multi_signal_gate_keeps_scene_cuts() {
        let count = (ANALYSIS_WIDTH * ANALYSIS_HEIGHT) as usize;
        let previous = features("a.png", vec![0; count]);
        let current = features("b.png", vec![255; count]);
        let metrics = compare_frames(&previous, &current, 0.965, 0.55);
        assert_eq!(metrics.relation, "scene_cut");
    }

    #[test]
    fn multi_signal_gate_detects_ten_percent_vertical_continuity() {
        let width = ANALYSIS_WIDTH as usize;
        let height = ANALYSIS_HEIGHT as usize;
        let previous_pixels = (0..width * height)
            .map(|index| ((index * 73 + index / width * 19 + 11) % 251) as u8)
            .collect::<Vec<_>>();
        let overlap_rows = 10;
        let mut current_pixels = (0..width * height)
            .map(|index| ((index * 29 + index / width * 47 + 131) % 251) as u8)
            .collect::<Vec<_>>();
        for y in 0..overlap_rows {
            for x in 0..width {
                current_pixels[y * width + x] =
                    previous_pixels[(height - overlap_rows + y) * width + x];
            }
        }
        let previous = features("a.png", previous_pixels);
        let current = features("b.png", current_pixels);
        let metrics = compare_frames(&previous, &current, 0.965, 0.55);
        assert_eq!(metrics.relation, "continuous_vertical");
        assert!(
            (metrics.overlap_ratio.unwrap() - overlap_rows as f64 / height as f64).abs() < 0.001
        );
    }

    #[test]
    fn sequence_never_compares_beyond_local_window() {
        let frames = identical_sequence(12);
        let decisions = decide_for_test(&frames, &HashMap::new());
        for (index, decision) in decisions.iter().enumerate().skip(1) {
            assert!(index - decision.compared_to.unwrap() <= MAX_ANCHOR_DISTANCE);
        }
        assert_eq!(decisions[3].engine_decision, "keep");
        assert_eq!(decisions[3].compared_to, Some(2));
        assert!(decisions[3].reason.contains("距离过远"));
    }

    #[test]
    fn user_keep_becomes_the_next_dynamic_anchor() {
        let frames = identical_sequence(6);
        let mut user_decisions = HashMap::new();
        user_decisions.insert(frames[2].path.clone(), "keep".into());
        let decisions = decide_for_test(&frames, &user_decisions);
        assert_eq!(decisions[3].compared_to, Some(2));
        assert!(decisions[3].reason.contains("依据你的保留决定"));
    }

    #[test]
    fn user_excludes_are_removed_from_anchor_candidates() {
        let frames = identical_sequence(6);
        let mut user_decisions = HashMap::new();
        user_decisions.insert(frames[0].path.clone(), "exclude".into());
        user_decisions.insert(frames[1].path.clone(), "exclude".into());
        let decisions = decide_for_test(&frames, &user_decisions);
        assert_eq!(decisions[1].engine_decision, "keep");
        assert_eq!(decisions[2].engine_decision, "keep");
        assert_eq!(decisions[3].compared_to, Some(2));
    }

    #[test]
    fn user_overrides_adjust_only_safe_downstream_thresholds() {
        let mut adaptive = AdaptiveThresholds::new(0.965, 0.10);
        adaptive.observe("duplicate", "exclude", Some("keep"), 0.05);
        assert!(adaptive.duplicate > 0.965);
        adaptive.observe("continuous_vertical", "exclude", Some("keep"), 0.05);
        assert!(adaptive.overlap_max > 0.10);
        let before = adaptive;
        adaptive.observe("changed", "keep", Some("exclude"), 0.05);
        assert_eq!(adaptive.duplicate, before.duplicate);
        assert_eq!(adaptive.overlap_max, before.overlap_max);
    }
}
