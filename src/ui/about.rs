// 关于 Tab

use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.heading("⛩ 300 高速咏唱装置");
    ui.label(format!("版本 v{}", env!("CARGO_PKG_VERSION")));
    ui.add_space(8.0);
    ui.hyperlink("https://github.com/yehuoshun/faster-chant-300-hero-rust");
    ui.add_space(8.0);
    if ui.button("检查更新").clicked() {
        crate::updater::check_update();
    }
    ui.separator();
    ui.label(
        egui::RichText::new(
            "使用说明：\n\
             · 启动后常驻系统托盘，游戏窗口在前台时按启动按键呼出悬浮面板\n\
             · 数字键 1-9 选择发言发送；0 键进入搜索页（拼音首字母 / 编号切换方案）\n\
             · 开启二级面板后需按两次数字键：第一次进二级面板，第二次选内容发送\n\
             · 连发间隔大于 0 时，首页按 1-9 直接连发对应二级面板内容\n\
             · 配置与方案保存在 exe 同目录 fcd_data/，日志在 log/",
        )
        .size(13.0),
    );
}