// 系统托盘：常驻图标 + 菜单（显示面板 / 打开设置 / 退出）

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

use crate::logger;

/// 生成 32x32 托盘图标（深蓝底 + 青色圆环 + 白点）
fn make_icon() -> Icon {
    let w = 32u32;
    let h = 32u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let dx = x as f32 + 0.5 - 15.5;
            let dy = y as f32 + 0.5 - 15.5;
            let d = (dx * dx + dy * dy).sqrt();
            // 背景深蓝灰
            rgba[i..i + 4].copy_from_slice(&[20, 22, 26, 255]);
            // 青色圆环
            if d < 14.0 && d > 10.0 {
                rgba[i..i + 4].copy_from_slice(&[34, 211, 238, 255]);
            }
            // 中心白点
            if d < 4.0 {
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, w, h).unwrap_or_else(|e| {
        logger::error(&format!("托盘图标创建失败: {}", e));
        // 兜底：1x1 透明像素
        Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).unwrap()
    })
}

/// 初始化托盘（保持 TrayIcon 引用存活）
pub fn init() {
    let show = MenuItem::new("显示/隐藏面板", true, None);
    let settings = MenuItem::new("打开设置", true, None);
    let quit = MenuItem::new("退出", true, None);
    let menu = Menu::new();
    if let Err(e) = menu.append_items(&[&show, &settings, &quit]) {
        logger::error(&format!("托盘菜单创建失败: {}", e));
        return;
    }

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("300高速咏唱装置")
        .with_icon(make_icon())
        .build();

    match tray {
        Ok(t) => {
            // TrayIcon 非 Send，无法放 static；泄漏以保持存活（进程生命周期）
            Box::leak(Box::new(t));
            logger::info("系统托盘已就绪");
        }
        Err(e) => logger::error(&format!("托盘初始化失败: {}", e)),
    }
}

/// 处理托盘事件（每帧轮询，非阻塞）
pub fn poll() {
    // 菜单项点击
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        match event.id.0.as_str() {
            "显示/隐藏面板" => crate::app::toggle_panel_from_tray(),
            "打开设置" => show_settings_window(),
            "退出" => {
                logger::info("托盘退出");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    // 托盘图标点击（左键 = 显示面板）
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if matches!(event, TrayIconEvent::Click { .. }) {
            crate::app::toggle_panel_from_tray();
        }
    }
}

/// 显示主设置窗口（通过 eframe 上下文）
fn show_settings_window() {
    if let Some(ctx) = crate::app::UI_CTX.lock().unwrap().clone() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
}