// 日志系统
// 目标：只看日志就能定位 90% 的问题
// 日志文件位于 exe 同目录下的 log/ 文件夹，文件名 faster-chant.log

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOGGER: once_cell::sync::Lazy<Mutex<Logger>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Logger::new()));

struct Logger {
    file: Option<File>,
    max_size: u64,
}

impl Logger {
    fn new() -> Self {
        let log_path = log_path();
        // 确保 log/ 目录存在，否则打开文件会失败
        if let Some(dir) = log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();

        if let Some(ref _f) = file {
            let _ = writeln!(&std::io::stderr(), "[日志] 日志文件: {}", log_path.display());
        }

        Self {
            file,
            max_size: 2 * 1024 * 1024, // 2MB 自动轮转
        }
    }

    fn rotate(&mut self) {
        let log_path = log_path();
        let bak_path = log_path.with_extension("log.bak");

        if let Ok(meta) = std::fs::metadata(&log_path) {
            if meta.len() > self.max_size {
                let _ = std::fs::rename(&log_path, &bak_path);
                self.file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .ok();
                if let Some(ref mut f) = self.file {
                    let _ = writeln!(f, "[日志] 日志文件已轮转，旧日志保存为 .log.bak");
                }
            }
        }
    }

    fn write(&mut self, level: &str, msg: &str) {
        let ts = timestamp();
        let line = format!("{} [{}] {}\n", ts, level, msg);

        // 写控制台
        let _ = io::stderr().write_all(line.as_bytes());

        // 写文件
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

fn timestamp() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let ms = dur.subsec_millis();

    // 东八区
    let total_secs = secs + 8 * 3600;
    let days = total_secs / 86400;
    let time_of_day = total_secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // 简单日期计算（从 Unix epoch 开始，到 2026 年为止）
    let (y, mon, d) = civil_date(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", y, mon, d, h, m, s, ms)
}

fn civil_date(mut days: u64) -> (u64, u64, u64) {
    // 从 1970-01-01 开始计算
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn log_path() -> PathBuf {
    // exe 同目录下的 log/ 文件夹
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let dir = exe
        .parent()
        .unwrap_or(Path::new("."))
        .join("log");
    dir.join("faster-chant.log")
}

// === 公开 API ===

fn log(level: &str, msg: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.rotate();
        logger.write(level, msg);
    }
}

pub fn debug(msg: &str) { log("DEBUG", msg); }
pub fn info(msg: &str)  { log("INFO ", msg); }
pub fn warn(msg: &str)  { log("WARN ", msg); }
pub fn error(msg: &str) { log("ERROR", msg); }
pub fn fatal(msg: &str) {
    log("FATAL", msg);
    std::process::exit(1);
}