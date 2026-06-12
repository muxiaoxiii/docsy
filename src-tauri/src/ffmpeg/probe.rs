use anyhow::Result;
use crate::external::ExternalTool;

pub fn probe_video(path: &str) -> Result<serde_json::Value> {
    let ffmpeg = crate::external::FfmpegTool;
    let bin = ffmpeg.binary_path()?;
    let ffprobe = bin.parent().unwrap_or(&std::path::PathBuf::from(".")).join("ffprobe");

    let output = std::process::Command::new(&ffprobe)
        .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("ffprobe 失败");
    }

    let info: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(info)
}
