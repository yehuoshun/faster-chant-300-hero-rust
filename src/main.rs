// 300高速咏唱装置 Rust 版
// 全局键盘钩子 → 状态机 → 悬浮窗 → 按键模拟

mod logger;
mod updater;
mod scheme;
mod search;
mod config;
mod state;
pub mod input;
mod burst;
mod overlay;

use std::io;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use windows::core::PWSTR;
use windows::Win32::Foundation::{LPARAM, WPARAM, LRESULT, HWND, CloseHandle};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_ESCAPE, VK_SPACE, KBDLLHOOKSTRUCT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN, WM_KEYUP, WM_SYSKEYUP,
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

use state::{StateMachine, Page, ActionResult};
use config::Config;
use scheme::SchemeManager;
use overlay::{Overlay, OverlayContent};
use burst::BurstController;

// ── 全局状态 ──

static STATE: Lazy<Mutex<Option<GlobalState>>> = Lazy::new(|| Mutex::new(None));

struct GlobalState {
    sm: StateMachine,
    config: Config,
    scheme_mgr: SchemeManager,
    overlay: Overlay,
    burst: BurstController,
    panel_visible: bool,
    burst_id: Option<u8>,
}

fn data_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("fcd_data")
}

fn init() -> Option<GlobalState> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);

    let config = Config::load(&dir).unwrap_or_default();
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
    let overlay = Overlay::new()?;

    Some(GlobalState {
        sm, config, scheme_mgr, overlay,
        burst: BurstController::new(),
        panel_visible: false,
        burst_id: None,
    })
}

// ── 窗口检测 ──
// only_300=true: 严格校验前台窗口所属进程的 exe 名包含 "300"
// only_300=false: 退化为标题包含 "300" 的宽松检测
fn find_game_window(only_300: bool) -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() { return None; }
        if !IsWindowVisible(hwnd).as_bool() { return None; }

        if only_300 {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 { return None; }

            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size);
            let _ = CloseHandle(process);
            if ok.is_err() { return None; }

            let path = String::from_utf16_lossy(&buf[..size as usize]);
            let exe = path.rsplit('\\').next().unwrap_or(&path).to_lowercase();
            if exe.contains("300") { Some(hwnd) } else { None }
        } else {
            let mut buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len == 0 { return None; }
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            if title.contains("300") { Some(hwnd) } else { None }
        }
    }
}

// ── 键盘钩子 ──

extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code < 0 {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
    let vk = kb.vkCode;
    let is_down = w_param.0 == WM_KEYDOWN as usize || w_param.0 == WM_SYSKEYDOWN as usize;
    let is_up = w_param.0 == WM_KEYUP as usize || w_param.0 == WM_SYSKEYUP as usize;

    if !is_down && !is_up {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let mut state = STATE.lock().unwrap();
    let gs = match state.as_mut() {
        Some(s) => s,
        None => return unsafe { CallNextHookEx(None, n_code, w_param, l_param) },
    };

    // 空格键状态跟踪
    if vk == VK_SPACE.0 as u32 {
        gs.sm.set_space(is_down);
    }

    // Esc 退出（关闭面板 + 停止连发）
    if vk == VK_ESCAPE.0 as u32 && is_down {
        gs.overlay.hide();
        gs.panel_visible = false;
        gs.burst.stop();
        gs.sm.reset();
        logger::info("Esc 关闭面板");
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let trigger = gs.config.trigger_key;

    // 检测游戏窗口（复用结果，避免每键多次探测）
    let game_hwnd = find_game_window(gs.config.only_300);
    let game_focused = game_hwnd.is_some();

    if !game_focused && gs.panel_visible {
        gs.overlay.hide();
        gs.panel_visible = false;
        gs.sm.reset();
        gs.burst.stop();
    }

    if !game_focused {
        drop(state);
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    if !is_down {
        drop(state);
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    // 触发键：切换面板
    if vk == trigger as u32 {
        if gs.panel_visible {
            gs.overlay.hide();
            gs.panel_visible = false;
            gs.burst.stop();
            gs.sm.reset();
        } else {
            // 面板贴到游戏窗口旁（按配置左右侧）
            if let Some(hwnd) = game_hwnd {
                position_overlay(&gs.overlay, hwnd, gs.config.panel_left);
            }
            gs.overlay.show();
            gs.panel_visible = true;
            refresh_overlay(gs);
        }
        drop(state);
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    // 面板未显示时忽略其他按键
    if !gs.panel_visible {
        drop(state);
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    // 路由到状态机
    let action = gs.sm.handle_key(vk, &gs.scheme_mgr);
    execute_action(gs, action);

    // 屏蔽热键：面板激活时，除触发键/Esc 外的按键不穿透到游戏
    if gs.config.shield_hotkey {
        drop(state);
        return LRESULT(1);
    }

    drop(state);
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn refresh_overlay(gs: &GlobalState) {
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

fn execute_action(gs: &mut GlobalState, action: ActionResult) {
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
            refresh_overlay(gs);
        }
        ActionResult::SendMessage(msg) => {
            input::send_message(&msg, gs.config.public_chat, gs.config.chat_mode);
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

// ── 窗口位置 ──
// 面板贴到游戏窗口内测（左/右由配置决定）
fn position_overlay(overlay: &Overlay, game_hwnd: HWND, panel_left: bool) {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    unsafe {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(game_hwnd, &mut rect).is_ok() {
            let x = if panel_left {
                rect.left + 10
            } else {
                rect.right - overlay.width() - 10
            };
            overlay.set_position(x, rect.top + 50);
        }
    }
}

// ── 主函数 ──

fn main() -> io::Result<()> {
    logger::info(&format!("程序启动 v{}", env!("CARGO_PKG_VERSION")));
    logger::info(&format!("数据目录: {:?}", data_dir()));

    updater::check_update();

    // 初始化全局状态
    let gs = init().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "初始化失败")
    })?;
    *STATE.lock().unwrap() = Some(gs);

    logger::info("初始化完成，安装键盘钩子");

    let hinst = unsafe { GetModuleHandleW(None).unwrap_or_default() };
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0)
    };

    match hook {
        Ok(h) => {
            logger::info("键盘钩子安装成功，等待触发键...");

            // 消息循环
            let mut msg = unsafe { std::mem::zeroed() };
            loop {
                let ret = unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetMessageW(
                        &mut msg, None, 0, 0,
                    )
                };
                if ret.0 <= 0 { break; }
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                    let _ = windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                }
            }

            unsafe { UnhookWindowsHookEx(h); }
            logger::info("程序正常退出");
        }
        Err(e) => {
            logger::error(&format!("安装钩子失败: {:?}", e));
            logger::error("请以管理员权限运行");
            eprintln!("按回车退出...");
            let mut s = String::new();
            io::stdin().read_line(&mut s)?;
        }
    }

    Ok(())
}