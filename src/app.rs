// 应用状态核心：全局状态、初始化、面板渲染、动作分发

use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::burst::BurstController;
use crate::config::Config;
use crate::logger;
use crate::overlay::{Overlay, OverlayContent, PanelStyle};
use crate::scheme::SchemeManager;
use crate::state::{ActionResult, Page, StateMachine};

/// 全局状态（单例，供键盘钩子回调访问）
pub static STATE: Lazy<Mutex<Option<GlobalState>>> = Lazy::new(|| Mutex::new(None));

pub struct GlobalState {
    pub sm: StateMachine,
    pub config: Config,
    pub scheme_mgr: SchemeManager,
    pub overlay: Overlay,
    pub burst: BurstController,
    pub panel_visible: bool,
    pub burst_id: Option<u8>,
}

/// 数据目录（与 exe 同级 fcd_data）
pub fn data_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("fcd_data")
}

/// 初始化全局状态
pub fn init() -> Option<GlobalState> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);

    let config = Config::load(&dir);
    let mut scheme_mgr = SchemeManager::init(dir.clone());
    // config.active_scheme 优先作为激活方案（若对应方案存在）
    if scheme_mgr.contains(config.active_scheme) {
        scheme_mgr.set_active(config.active_scheme);
    }
    let sm = StateMachine::new(
        scheme_mgr.active(),
        config.use_secondary,
        config.burst_interval,
        config.auto_back,
    );
    let overlay = Overlay::new(&PanelStyle::from_config(&config))?;

    Some(GlobalState {
        sm, config, scheme_mgr, overlay,
        burst: BurstController::new(),
        panel_visible: false,
        burst_id: None,
    })
}

/// 根据当前页面状态刷新悬浮面板（先同步样式再渲染）
pub fn refresh_overlay(gs: &mut GlobalState) {
    gs.overlay.set_style(&PanelStyle::from_config(&gs.config));
    let scheme = gs.scheme_mgr.get(gs.sm.scheme_id());
    let content = match gs.sm.page() {
        Page::Home => {
            let items = if let Some(s) = scheme {
                s.primary.iter().map(|s| s.clone()).collect()
            } else {
                vec!["".into(); 9]
            };
            OverlayContent::Home {
                items,
                active: gs.sm.scheme_id(),
                name: scheme.map(|s| s.name.clone()).unwrap_or_default(),
            }
        }
        Page::Secondary(_) => {
            let items = if let Some(s) = scheme {
                let idx = match gs.sm.page() {
                    Page::Secondary(n) => *n as usize - 1,
                    _ => 0,
                };
                s.secondary[idx].clone()
            } else {
                [(); 10].map(|_| String::new())
            };
            OverlayContent::Secondary {
                index: match gs.sm.page() {
                    Page::Secondary(n) => *n,
                    _ => 0,
                },
                items,
            }
        }
        Page::Search => {
            OverlayContent::Search {
                query: String::new(),
                results: vec![],
            }
        }
    };
    gs.overlay.update(&content);
}

/// 执行状态机返回的动作
pub fn execute_action(gs: &mut GlobalState, action: ActionResult) {
    match action {
        ActionResult::None => {}
        ActionResult::SwitchPage(_) => {
            refresh_overlay(gs);
        }
        ActionResult::SwitchScheme(id) => {
            gs.scheme_mgr.set_active(id);
            gs.sm.update_config(
                id,
                gs.config.use_secondary,
                gs.config.burst_interval,
                gs.config.auto_back,
            );
            gs.config.active_scheme = id;
            let dir = data_dir();
            gs.config.save(&dir);
            refresh_overlay(gs);
        }
        ActionResult::SendMessage(msg) => {
            crate::input::send_message(&msg, gs.config.public_chat, gs.config.chat_mode);
            refresh_overlay(gs);
        }
        ActionResult::StartBurst(scheme_id, secondary_index) => {
            // 从方案取二级面板内容并启动连发线程
            let items = match gs.scheme_mgr.get(scheme_id) {
                Some(scheme) => {
                    let idx = secondary_index.saturating_sub(1) as usize;
                    if idx < scheme.secondary.len() {
                        scheme.secondary[idx].to_vec()
                    } else {
                        Vec::new()
                    }
                }
                None => Vec::new(),
            };
            gs.burst.start(
                scheme_id,
                secondary_index,
                gs.config.burst_interval,
                gs.config.public_chat,
                gs.config.chat_mode,
                items,
            );
        }
        ActionResult::SetBurstInterval(interval) => {
            gs.config.burst_interval = interval;
            if interval == 0 {
                gs.burst.stop(); // 间隔 0 = 关闭连发
            }
            let dir = data_dir();
            gs.config.save(&dir); // 落盘，重启不丢
            gs.sm.update_config(
                gs.sm.scheme_id(),
                gs.config.use_secondary,
                interval,
                gs.config.auto_back,
            );
            logger::info(&format!("连发间隔: {}秒", interval));
        }
        ActionResult::Close => {
            gs.overlay.hide();
            gs.panel_visible = false;
            gs.sm.reset();
        }
        ActionResult::UpdateSearch(query, results) => {
            gs.overlay.update(&OverlayContent::Search { query, results });
        }
    }
}

// ── UI/托盘辅助 ──

/// egui 窗口上下文（供托盘菜单操作主窗口）
pub static UI_CTX: Lazy<Mutex<Option<egui::Context>>> = Lazy::new(|| Mutex::new(None));

/// 保存配置并同步运行态（UI 修改后调用）
pub fn save_config_and_sync() {
    let mut state = STATE.lock().unwrap();
    if let Some(gs) = state.as_mut() {
        let dir = data_dir();
        gs.config.save(&dir);
        gs.sm.update_config(
            gs.scheme_mgr.active(),
            gs.config.use_secondary,
            gs.config.burst_interval,
            gs.config.auto_back,
        );
        gs.overlay.set_style(&PanelStyle::from_config(&gs.config));
    }
}

/// 托盘菜单：显示/隐藏悬浮面板（需游戏窗口在前台）
pub fn toggle_panel_from_tray() {
    let mut state = STATE.lock().unwrap();
    if let Some(gs) = state.as_mut() {
        if gs.panel_visible {
            gs.overlay.hide();
            gs.panel_visible = false;
            return;
        }
        let game = crate::window::find_game_window(gs.config.only_300);
        if let Some(hwnd) = game {
            crate::window::position_overlay(&gs.overlay, hwnd, gs.config.panel_left);
            gs.overlay.show();
            gs.panel_visible = true;
            refresh_overlay(gs);
        } else {
            logger::info("托盘呼面板：未检测到游戏窗口");
        }
    }
}
