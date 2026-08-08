// 主设置窗口（egui）：左侧导航 + Tab 内容分发

pub mod about;
pub mod schemes;
pub mod settings;
pub mod styles;

use eframe::egui;

use crate::app;

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Settings,
    Styles,
    Schemes,
    About,
}

pub struct MainApp {
    pub tab: Tab,
    /// 方案管理：当前选中的方案编号
    pub selected_scheme: Option<u8>,
    /// 方案管理：搜索过滤词
    pub scheme_filter: String,
}

impl MainApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_cjk_fonts(&cc.egui_ctx);
        Self {
            tab: Tab::Settings,
            selected_scheme: None,
            scheme_filter: String::new(),
        }
    }
}

/// 加载系统中文字体，否则中文会显示为方块
fn setup_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",   // 微软雅黑
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf", // 黑体
        "C:\\Windows\\Fonts\\simsun.ttc", // 宋体
    ];
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(data));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push("cjk".to_owned());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 供托盘菜单操作主窗口
        *app::UI_CTX.lock().unwrap() = Some(ctx.clone());
        // 处理托盘事件
        crate::tray::poll();

        // 关闭窗口 → 隐藏到托盘（程序继续跑钩子）
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // 顶栏
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⛩ 300 高速咏唱装置");
                ui.separator();
                let running = app::STATE.lock().unwrap().is_some();
                ui.label(if running {
                    egui::RichText::new("● 钩子运行中").color(egui::Color32::from_rgb(34, 197, 94))
                } else {
                    egui::RichText::new("○ 未运行").color(egui::Color32::from_rgb(148, 163, 184))
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("显示/隐藏面板").clicked() {
                        app::toggle_panel_from_tray();
                    }
                });
            });
        });

        // 左侧导航
        egui::SidePanel::left("nav")
            .resizable(false)
            .default_width(140.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.selectable_value(&mut self.tab, Tab::Settings, "⚙ 基本设置");
                ui.selectable_value(&mut self.tab, Tab::Styles, "🎨 面板样式");
                ui.selectable_value(&mut self.tab, Tab::Schemes, "📋 方案管理");
                ui.selectable_value(&mut self.tab, Tab::About, "ℹ 关于");
            });

        // 内容区
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Settings => settings::show(ui),
                Tab::Styles => styles::show(ui),
                Tab::Schemes => schemes::show(ui, self),
                Tab::About => about::show(ui),
            }
        });
    }
}