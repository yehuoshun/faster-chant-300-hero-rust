// 阶段 0：键盘钩子最小验证 + 自动更新 + 日志
// 编译：cargo build --release
// 运行：需要管理员权限

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
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, WPARAM, LRESULT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT,
    VK_MENU, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7,
    VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_RCONTROL, VK_RETURN,
    VK_RMENU, VK_RSHIFT, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn key_name(vk_code: u32) -> &'static str {
    let vk = vk_code as u16;
    if vk == VK_ESCAPE.0 { return "Esc"; }
    if vk == VK_RETURN.0 { return "Enter"; }
    if vk == VK_SPACE.0 { return "Space"; }
    if vk == VK_BACK.0 { return "Backspace"; }
    if vk == VK_TAB.0 { return "Tab"; }
    if vk == VK_CAPITAL.0 { return "CapsLock"; }
    if vk == VK_CONTROL.0 || vk == VK_LCONTROL.0 || vk == VK_RCONTROL.0 { return "Ctrl"; }
    if vk == VK_MENU.0 || vk == VK_LMENU.0 || vk == VK_RMENU.0 { return "Alt"; }
    if vk == VK_SHIFT.0 || vk == VK_LSHIFT.0 || vk == VK_RSHIFT.0 { return "Shift"; }
    if vk == VK_OEM_1.0 { return ";"; }
    if vk == VK_OEM_2.0 { return "/"; }
    if vk == VK_OEM_3.0 { return "~"; }
    if vk == VK_OEM_4.0 { return "["; }
    if vk == VK_OEM_5.0 { return "\\"; }
    if vk == VK_OEM_6.0 { return "]"; }
    if vk == VK_OEM_7.0 { return "'"; }
    if vk == VK_OEM_PLUS.0 { return "="; }
    if vk == VK_OEM_MINUS.0 { return "-"; }
    if vk == VK_OEM_COMMA.0 { return ","; }
    if vk == VK_OEM_PERIOD.0 { return "."; }
    if (0x30..=0x39).contains(&vk_code) { return "数字键"; }
    if (0x41..=0x5A).contains(&vk_code) { return "字母键"; }
    if (0x60..=0x69).contains(&vk_code) { return "小键盘数字"; }
    if (0x70..=0x7B).contains(&vk_code) { return "功能键"; }
    "其他"
}

extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode;

        if w_param.0 == WM_KEYDOWN as usize || w_param.0 == WM_SYSKEYDOWN as usize {
            let name = key_name(vk);
            logger::debug(&format!("按下 vkCode={} (0x{:02X}) {}", vk, vk, name));

            if vk as u16 == VK_ESCAPE.0 {
                logger::info("检测到 Esc，准备退出");
                RUNNING.store(false, Ordering::SeqCst);
            }
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn main() -> io::Result<()> {
    logger::info(&format!("程序启动 v{}", env!("CARGO_PKG_VERSION")));
    logger::info(&format!("exe路径: {:?}", std::env::current_exe().unwrap_or_default()));

    // 自动更新检查
    updater::check_update();

    let hinst = unsafe { GetModuleHandleW(None).unwrap_or_default() };

    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0)
    };

    match hook {
        Ok(h) => {
            logger::info("键盘钩子安装成功，开始监听");

            while RUNNING.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
            }

            unsafe { UnhookWindowsHookEx(h); }
            logger::info("键盘钩子已卸载，程序正常退出");
        }
        Err(e) => {
            logger::error(&format!("安装键盘钩子失败: {:?}", e));
            logger::error("请以管理员权限运行此程序");
            eprintln!("按回车退出...");
            let mut s = String::new();
            io::stdin().read_line(&mut s)?;
        }
    }

    Ok(())
}