use std::sync::OnceLock;
use std::path::PathBuf;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init() {
    let dir = log_dir_path();
    LOG_DIR.set(dir.clone()).ok();
    std::fs::create_dir_all(&dir).ok();

    let log_file = current_log_path().unwrap_or_else(|| dir.join("docsy.log"));

    // Simple file logging setup
    // In production, use tracing or fern for structured JSON Lines logging
    log::set_max_level(log::LevelFilter::Debug);
    eprintln!("日志目录: {}", dir.display());
    eprintln!("日志文件: {}", log_file.display());
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_default();

        log::error!(
            target: "panic",
            "thread '{}' panicked at '{}', {}",
            thread_name, payload, location
        );

        default_hook(info);
    }));
}

pub fn current_log_path() -> Option<PathBuf> {
    let dir = log_dir_path();
    let date = chrono::Local::now().format("%Y%m%d");
    Some(dir.join(format!("docsy-{}.log", date)))
}

pub fn log_dir() -> Option<PathBuf> {
    Some(log_dir_path())
}

fn log_dir_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Docsy")
        .join("logs")
}
