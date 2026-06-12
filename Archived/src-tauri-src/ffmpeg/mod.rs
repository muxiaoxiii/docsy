//! FFmpeg 视频抽帧模块
//!
//! 功能：
//! 1. 检测系统 ffmpeg 安装状态
//! 2. 下载安装 ffmpeg（支持私有地址）
//! 3. 读取视频信息（帧率、分辨率、时长等）
//! 4. 执行视频抽帧（支持时间轴叠加）

pub mod detect;
pub mod extract;
pub mod probe;
