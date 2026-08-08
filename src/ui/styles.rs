// 面板样式 Tab：样式参数 + 实时预览

use eframe::egui;

use crate::app;

const FONTS: &[&str] = &["Microsoft YaHei", "SimHei", "SimSun", "KaiTi", "DengXian"];

pub fn show(ui: &mut egui::Ui) {
    let mut changed = false;
    {
        let mut state = app::STATE.lock().unwrap();
        let gs = state.as_mut().unwrap();

        egui::Grid::new("styles_grid")
            .num_columns(2)
            .spacing([24.0, 14.0])
            .show(ui, |ui| {
                ui.label("面板宽度");
                changed |= ui.add(egui::Slider::new(&mut gs.config.panel_width, 260..=600).text("px")).changed();
                ui.end_row();

                ui.label("背景颜色");
                changed |= ui.color_edit_button_srgb(&mut gs.config.bg_color).changed();
                ui.end_row();

                ui.label("背景透明度");
                changed |= ui.add(egui::Slider::new(&mut gs.config.bg_alpha, 20..=255).text("0-255")).changed();
                ui.end_row();

                ui.label("字体");
                let mut sel = FONTS.iter().position(|f| gs.config.font_family == *f).unwrap_or(0);
                if egui::ComboBox::from_id_salt("font_family")
                    .selected_text(FONTS[sel])
                    .show_ui(ui, |ui| {
                        for (i, f) in FONTS.iter().enumerate() {
                            ui.selectable_value(&mut sel, i, *f);
                        }
                    })
                    .response
                    .changed()
                {
                    gs.config.font_family = FONTS[sel].to_string();
                    changed = true;
                }
                ui.end_row();

                ui.label("字号");
                changed |= ui.add(egui::Slider::new(&mut gs.config.font_size, 12..=40).text("px")).changed();
                ui.end_row();

                ui.label("字体加粗");
                changed |= ui.checkbox(&mut gs.config.font_bold, "粗体").changed();
                ui.end_row();

                ui.label("文字颜色");
                changed |= ui.color_edit_button_srgb(&mut gs.config.text_color).changed();
                ui.end_row();

                ui.label("描边颜色");
                changed |= ui.color_edit_button_srgb(&mut gs.config.outline_color).changed();
                ui.end_row();

                ui.label("描边大小");
                changed |= ui.add(egui::Slider::new(&mut gs.config.outline_size, 0..=4).text("px")).changed();
                ui.end_row();
            });
    }
    if changed {
        app::save_config_and_sync();
    }

    ui.separator();
    ui.label("预览（模拟悬浮面板效果）");
    preview(ui);
}

/// 画一个模拟悬浮面板预览
fn preview(ui: &mut egui::Ui) {
    let state = app::STATE.lock().unwrap();
    let Some(gs) = state.as_ref() else { return; };
    let cfg = &gs.config;

    let width = (cfg.panel_width as f32).min(560.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 240.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 面板背景
    let bg = egui::Color32::from_rgba_unmultiplied(cfg.bg_color[0], cfg.bg_color[1], cfg.bg_color[2], cfg.bg_alpha);
    painter.rect_filled(rect, egui::CornerRadius::same(6), bg);

    // 描边函数：粗描边文字
    let fg = egui::Color32::from_rgb(cfg.text_color[0], cfg.text_color[1], cfg.text_color[2]);
    let oc = egui::Color32::from_rgb(cfg.outline_color[0], cfg.outline_color[1], cfg.outline_color[2]);
    let font_id = egui::FontId::proportional(cfg.font_size as f32);
    let _ = cfg.font_bold; // 预览用同一字形，真实加粗由 GDI 渲染决定

    let samples: [&str; 5] = ["方案 1: 常用话术", "1. 集合", "2. 跟我上", "3. 撤退", "4. 666"];
    let n = cfg.outline_size.max(1) as i32;
    for (i, text) in samples.iter().enumerate() {
        let pos = egui::pos2(rect.left() + 12.0, rect.top() + 24.0 + i as f32 * (cfg.font_size as f32 + 10.0));
        for dx in -n..=n {
            for dy in -n..=n {
                if dx == 0 && dy == 0 { continue; }
                painter.text(
                    egui::pos2(pos.x + dx as f32, pos.y + dy as f32),
                    egui::Align2::LEFT_TOP,
                    *text,
                    font_id.clone(),
                    oc,
                );
            }
        }
        painter.text(pos, egui::Align2::LEFT_TOP, *text, font_id.clone(), fg);
    }
}