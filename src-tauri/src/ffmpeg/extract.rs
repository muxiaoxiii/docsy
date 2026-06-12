use anyhow::Result;

pub fn extract(args: &serde_json::Value) -> Result<serde_json::Value> {
    let ffmpeg = crate::external::FfmpegTool;
    let bin = ffmpeg.binary_path()?;

    let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
    let output_dir = args.get("output_dir").and_then(|v| v.as_str()).unwrap_or(".");

    let mut cmd = std::process::Command::new(&bin);
    cmd.arg("-i").arg(input);

    // Mode: per_second or interval
    if let Some(fps) = args.get("fps").and_then(|v| v.as_f64()) {
        cmd.arg("-vf").arg(format!("fps={}", fps));
    }

    let output_pattern = format!("{}/frame_%04d.jpg", output_dir);
    cmd.arg(&output_pattern);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("ffmpeg 抽帧失败");
    }

    Ok(serde_json::json!({ "output_dir": output_dir }))
}
