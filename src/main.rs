// 300高速咏唱装置 Rust 版
// 全局键盘钩子 → 状态机 → 悬浮窗 → 按键模拟
// 入口：托盘常驻 + egui 设置窗口 + 后台钩子线程

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
mod tray;
mod ui;
mod updater;
mod window;

use eframe::egui;

fn main() -> eframe::Result {
    // 崩溃时把 panic + 堆栈写入日志（最先安装）
    logger::install_panic_hook();

    logger::info(&format!("程序启动 v{}", env!("CARGO_PKG_VERSION")));
    logger::info(&format!("数据目录: {:?}", app::data_dir()));

    updater::check_update();

    // 初始化全局状态
    let Some(gs) = app::init() else {
        logger::error("初始化失败（悬浮窗创建失败）");
        std::process::exit(1);
    };
    *app::STATE.lock().unwrap() = Some(gs);

    // 键盘钩子跑在独立线程（低层钩子需要消息循环）
    std::thread::spawn(|| {
        let _ = hook::run_hook();
    });

    // 系统托盘常驻
    tray::init();

    // 主设置窗口
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 640.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("300 高速咏唱装置 - 设置"),
        ..Default::default()
    };
    eframe::run_native(
        "300 高速咏唱装置",
        options,
        Box::new(|cc| Ok(Box::new(ui::MainApp::new(cc)))),
    )
}