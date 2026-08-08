// 方案管理 Tab：方案列表 + 编辑

use eframe::egui;

use crate::app;
use crate::scheme::Scheme;
use crate::ui::MainApp;

pub fn show(ui: &mut egui::Ui, main: &mut MainApp) {
    let mut state = app::STATE.lock().unwrap();
    let gs = state.as_mut().unwrap();
    let active = gs.scheme_mgr.active();

    // ── 左侧：方案列表 ──
    egui::SidePanel::left("scheme_list")
        .resizable(false)
        .default_width(240.0)
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut main.scheme_filter).desired_width(150.0));
                if ui.button("＋").on_hover_text("新建方案").clicked() {
                    if let Some(b) = gs.scheme_mgr.blank_id() {
                        let _ = gs.scheme_mgr.append(b);
                        main.selected_scheme = Some(b);
                    }
                }
            });
            ui.label(egui::RichText::new("搜索：拼音首字母 / 编号").small().weak());
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let filter = main.scheme_filter.trim().to_uppercase();
                let entries: Vec<(u8, String)> = gs
                    .scheme_mgr
                    .list()
                    .filter(|(id, s)| {
                        filter.is_empty()
                            || s.spell().starts_with(&filter)
                            || format!("{:02}", id).contains(&filter)
                    })
                    .map(|(id, s)| (id, s.name.clone()))
                    .collect();

                for (id, name) in &entries {
                    let label = if *id == active {
                        format!("{:02}  {}  ✓", id, name)
                    } else {
                        format!("{:02}  {}", id, name)
                    };
                    if ui.selectable_label(main.selected_scheme == Some(*id), label).clicked() {
                        main.selected_scheme = Some(*id);
                    }
                }
                if entries.is_empty() {
                    ui.label(egui::RichText::new("暂无方案，点击「＋」新建").weak());
                }
            });

            ui.separator();
            if main.selected_scheme.is_some() && ui.button("删除当前方案").clicked() {
                let id = main.selected_scheme.unwrap();
                if gs.scheme_mgr.remove(id) {
                    main.selected_scheme = None;
                    app::save_config_and_sync();
                }
            }
        });

    // ── 右侧：方案编辑 ──
    egui::CentralPanel::default().show_inside(ui, |ui| {
        let Some(id) = main.selected_scheme else {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("选择左侧方案进行编辑，或点「＋」新建").size(16.0).weak());
            });
            return;
        };

        let mut scheme = gs.scheme_mgr.get(id).cloned().unwrap_or_default();
        let mut dirty = false;

        ui.horizontal(|ui| {
            ui.strong(format!("方案 {:02}", id));
            if ui.button("设为激活").clicked() {
                gs.scheme_mgr.set_active(id);
                gs.config.active_scheme = id;
                app::save_config_and_sync();
            }
            if ui.button("重命名(改编号)").clicked() {
                rename_scheme(gs, id);
            }
        });
        ui.separator();

        ui.label("方案名称");
        dirty |= ui.text_edit_singleline(&mut scheme.name).changed();

        dirty |= ui.checkbox(&mut scheme.use_secondary, "启用二级面板（按两次数字键）").changed();
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.strong("一级面板（数字键 1-9）");
            egui::Grid::new("primary_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for i in 0..9 {
                    ui.label(format!("{}", i + 1));
                    dirty |= ui.add(egui::TextEdit::singleline(&mut scheme.primary[i]).desired_width(320.0)).changed();
                    ui.end_row();
                }
            });

            if scheme.use_secondary {
                ui.separator();
                ui.strong("二级面板（9 组 × 10 条）");
                for g in 0..9 {
                    egui::CollapsingHeader::new(format!("二级面板 {}（数字键 {}-9 进入）", g + 1, g + 1))
                        .default_open(g == 0)
                        .show(ui, |ui| {
                            egui::Grid::new(("sec_grid", g)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                                for j in 0..10 {
                                    ui.label(format!("{}", j));
                                    dirty |= ui.add(egui::TextEdit::singleline(&mut scheme.secondary[g][j]).desired_width(300.0)).changed();
                                    ui.end_row();
                                }
                            });
                        });
                }
            }
        });

        if dirty {
            gs.scheme_mgr.update(id, scheme);
        }
    });
}

/// 重命名方案（改编号）：输入新编号后调用
fn rename_scheme(gs: &mut crate::app::GlobalState, id: u8) {
    // 简单实现：找到第一个空编号并迁移，提示日志
    if let Some(blank) = gs.scheme_mgr.blank_id() {
        if gs.scheme_mgr.rename(id, blank) {
            crate::logger::info(&format!("方案 {} 已迁移到 {}", id, blank));
        }
    }
}

/// 供 MainApp 使用（避免未使用告警时的占位）
#[allow(dead_code)]
fn _scheme_default() -> Scheme {
    Scheme::default()
}