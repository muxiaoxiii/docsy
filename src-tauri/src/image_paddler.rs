use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeArgs {
    pub folder: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    pub images: Vec<ImageInfo>,
    pub groups: Vec<ImageGroup>,
    pub recommended: RecommendedSettings,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
}

#[derive(Debug, Serialize)]
pub struct ImageGroup {
    pub prefix: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct RecommendedSettings {
    pub orientation: String,
    pub layout: String,
    pub dpi: u32,
}

#[derive(Debug, Deserialize)]
pub struct RunArgs {
    pub folder: String,
    pub output_format: String,
    pub layout: String,
    pub orientation: String,
    pub dpi: u32,
    pub scale_mode: String,
}

#[derive(Debug, Serialize)]
pub struct RunResult {
    pub output_path: String,
    pub pages: u32,
    pub images: u32,
}

pub fn analyze(args: &AnalyzeArgs) -> Result<AnalyzeResult> {
    let mut images = Vec::new();
    let dir = std::fs::read_dir(&args.folder)?;

    for entry in dir {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tif" | "tiff") {
                if let Ok(img) = image::open(&path) {
                    images.push(ImageInfo {
                        path: path.display().to_string(),
                        width: img.width(),
                        height: img.height(),
                        file_size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                    });
                }
            }
        }
    }

    images.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(AnalyzeResult {
        images,
        groups: vec![],
        recommended: RecommendedSettings {
            orientation: "portrait".into(),
            layout: "2x2".into(),
            dpi: 300,
        },
    })
}

pub fn run(args: &RunArgs) -> Result<RunResult> {
    let analyze_result = analyze(&AnalyzeArgs {
        folder: args.folder.clone(),
    })?;

    let output_dir = std::path::Path::new(&args.folder).join("_docsy_image_out");
    std::fs::create_dir_all(&output_dir)?;

    let output_path = output_dir.join(format!("output.{}", args.output_format));

    // TODO: implement actual image layout engine

    Ok(RunResult {
        output_path: output_path.display().to_string(),
        pages: 1,
        images: analyze_result.images.len() as u32,
    })
}
