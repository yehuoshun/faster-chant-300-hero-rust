// 全局键盘钩子：按键回调 + 钩子安装 + 消息循环

use std::io;

use windows::Win32::Foundation::{LPARAM, WPARAM, LRESULT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_SPACE};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN, WM_KEYUP, WM_SYSKEYUP,
    KBDLLHOOKSTRUCT,
};

use crate::app::{execute_action, refresh_overlay, STATE};
use crate::logger;
use crate::window::{find_game_window, position_overlay};

/// 低层键盘钩子回调
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
        logger::info("游戏窗口失去焦点，面板隐藏");
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
            logger::info("触发键关闭面板");
            gs.overlay.hide();
            gs.panel_visible = false;
            gs.burst.stop();
            gs.sm.reset();
        } else {
            // 面板贴到游戏窗口旁（按配置左右侧）
            if let Some(hwnd) = game_hwnd {
                position_overlay(&gs.overlay, hwnd, gs.config.panel_left);
            }
            logger::info(&format!("呼出面板，当前方案: {}", gs.sm.scheme_id()));
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
        logger::debug(&format!("热键屏蔽: vk={}", vk));
        drop(state);
        return LRESULT(1);
    }

    drop(state);
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// 安装键盘钩子并进入消息循环（阻塞直到退出）
pub fn run_hook() -> io::Result<()> {
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
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 <= 0 { break; }
                unsafe {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
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
