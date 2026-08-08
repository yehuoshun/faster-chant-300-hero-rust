// 基本设置 Tab

use eframe::egui;

use crate::app;

/// 可选触发键（名称, vkCode）
const TRIGGERS: &[(&str, u16)] = &[
    ("~ 键", 0xC0),
    ("F1", 0x70), ("F2", 0x71), ("F3", 0x72), ("F4", 0x73),
    ("F5", 0x74), ("F6", 0x75), ("F7", 0x76), ("F8", 0x77),
    ("F9", 0x78), ("F10", 0x79), ("F11", 0x7A), ("F12", 0x7B),
    ("Tab", 0x09), ("CapsLock", 0x14),
];

pub fn show(ui: &mut egui::Ui) {
    let mut changed = false;
    {
        let mut state = app::STATE.lock().unwrap();
        let gs = state.as_mut().unwrap();

        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                // 启动按键
                ui.label("启动按键");
                let mut sel = TRIGGERS.iter().position(|(_, c)| *c == gs.config.trigger_key).unwrap_or(0);
                if egui::ComboBox::from_id_salt("trigger_key")
                    .selected_text(TRIGGERS[sel].0)
                    .show_ui(ui, |ui| {
                        for (i, (name, _)) in TRIGGERS.iter().enumerate() {
                            ui.selectable_value(&mut sel, i, *name);
                        }
                    })
                    .response
                    .changed()
                {
                    gs.config.trigger_key = TRIGGERS[sel].1;
                    changed = true;
                }
                ui.end_row();

                // 仅限三百
                ui.label("仅限三百");
                changed |= ui.checkbox(&mut gs.config.only_300, "只在前台窗口为 300 时响应").changed();
                ui.end_row();

                // 全体发言
                ui.label("全体发言");
                changed |= ui.checkbox(&mut gs.config.public_chat, "发送时自动按 Shift 进全体频道").changed();
                ui.end_row();

                // 聊天模式
                ui.label("聊天模式");
                changed |= ui.checkbox(&mut gs.config.chat_mode, "不模拟回车，只粘贴文本").changed();
                ui.end_row();

                // 热键屏蔽
                ui.label("热键屏蔽");
                changed |= ui.checkbox(&mut gs.config.shield_hotkey, "面板激活时按键不穿透到游戏").changed();
                ui.end_row();

                // 自动回首页
                ui.label("自动回首页");
                changed |= ui.checkbox(&mut gs.config.auto_back, "二级面板发送后自动返回首页").changed();
                ui.end_row();

                // 面板位置
                ui.label("面板位置");
                let mut left = gs.config.panel_left;
                if ui.radio_value(&mut left, true, "左侧").changed() || ui.radio_value(&mut left, false, "右侧").changed() {
                    gs.config.panel_left = left;
                    changed = true;
                }
                ui.end_row();

                // 连发间隔
                ui.label("连发间隔");
                changed |= ui.add(egui::Slider::new(&mut gs.config.burst_interval, 0..=9).text("秒 (0=关闭)")).changed();
                ui.end_row();
            });

        ui.separator();
        ui.label(
            egui::RichText::new("提示：游戏内按启动按键呼出面板，数字键选择发言，0 键搜索切换方案，Esc 关闭面板。")
                .small()
                .weak(),
        );
    }
    if changed {
        app::save_config_and_sync();
    }
}