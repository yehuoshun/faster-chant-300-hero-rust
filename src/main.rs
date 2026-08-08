// 300高速咏唱装置 Rust 版
// 全局键盘钩子 → 状态机 → 悬浮窗 → 按键模拟

mod app;
mod burst;
mod config;
mod hook;
mod input;
mod logger;
mod overlay;
mod scheme;
mod search;
mod state;
mod updater;
mod window;

use std::io;

use crate::app::data_dir;

fn main() -> io::Result<()> {
    logger::info(&format!("程序启动 v{}", env!("CARGO_PKG_VERSION")));
    logger::info(&format!("数据目录: {:?}", data_dir()));

    updater::check_update();

    // 初始化全局状态
    let gs = app::init().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "初始化失败")
    })?;
    *app::STATE.lock().unwrap() = Some(gs);

    hook::run_hook()
}
