// 阶段 0：键盘钩子最小验证
// 功能：安装全局键盘钩子，打印按键码，按 Esc 退出
// 编译：cargo build --release
// 运行：需要管理员权限

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use windows::core::WParam;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT,
    VK_MENU, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7,
    VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_RCONTROL, VK_RETURN,
    VK_RMENU, VK_RSHIFT, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

/// VK 常量在 windows 0.58 中是 u16 类型
fn vk_eq(vk: u32, target: u16) -> bool {
    vk == target as u32
}

fn key_name(vk_code: u32) -> &'static str {
    if vk_eq(vk_code, VK_ESCAPE) { return "Esc"; }
    if vk_eq(vk_code, VK_RETURN) { return "Enter"; }
    if vk_eq(vk_code, VK_SPACE) { return "Space"; }
    if vk_eq(vk_code, VK_BACK) { return "Backspace"; }
    if vk_eq(vk_code, VK_TAB) { return "Tab"; }
    if vk_eq(vk_code, VK_CAPITAL) { return "CapsLock"; }
    if vk_eq(vk_code, VK_CONTROL) || vk_eq(vk_code, VK_LCONTROL) || vk_eq(vk_code, VK_RCONTROL) { return "Ctrl"; }
    if vk_eq(vk_code, VK_MENU) || vk_eq(vk_code, VK_LMENU) || vk_eq(vk_code, VK_RMENU) { return "Alt"; }
    if vk_eq(vk_code, VK_SHIFT) || vk_eq(vk_code, VK_LSHIFT) || vk_eq(vk_code, VK_RSHIFT) { return "Shift"; }
    if vk_eq(vk_code, VK_OEM_1) { return ";"; }
    if vk_eq(vk_code, VK_OEM_2) { return "/"; }
    if vk_eq(vk_code, VK_OEM_3) { return "~"; }
    if vk_eq(vk_code, VK_OEM_4) { return "["; }
    if vk_eq(vk_code, VK_OEM_5) { return "\\"; }
    if vk_eq(vk_code, VK_OEM_6) { return "]"; }
    if vk_eq(vk_code, VK_OEM_7) { return "'"; }
    if vk_eq(vk_code, VK_OEM_PLUS) { return "="; }
    if vk_eq(vk_code, VK_OEM_MINUS) { return "-"; }
    if vk_eq(vk_code, VK_OEM_COMMA) { return ","; }
    if vk_eq(vk_code, VK_OEM_PERIOD) { return "."; }
    if (0x30..=0x39).contains(&vk_code) { return "数字键"; }
    if (0x41..=0x5A).contains(&vk_code) { return "字母键"; }
    if (0x60..=0x69).contains(&vk_code) { return "小键盘数字"; }
    if (0x70..=0x7B).contains(&vk_code) { return "功能键"; }
    "其他"
}

extern "system" fn keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code >= 0 {
        let kb = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode;

        if w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN {
            let name = key_name(vk);
            println!("[按下] vkCode={:3} (0x{:02X}) | {}", vk, vk, name);

            if vk == VK_ESCAPE as u32 {
                println!(">>> 检测到 Esc，退出中...");
                RUNNING.store(false, Ordering::SeqCst);
            }
        } else if w_param as u32 == WM_KEYUP || w_param as u32 == WM_SYSKEYUP {
            let name = key_name(vk);
            println!("[释放] vkCode={:3} (0x{:02X}) | {}", vk, vk, name);
        }
    }

    unsafe { CallNextHookEx(None, n_code, WParam(w_param), LPARAM(l_param)) }
}

fn main() -> io::Result<()> {
    println!("============================================");
    println!("  300高速咏唱装置 - 阶段0：键盘钩子验证");
    println!("  按任意键查看按键码，按 Esc 退出");
    println!("  注意：需要管理员权限运行");
    println!("============================================\n");

    let hinst = unsafe { GetModuleHandleW(None).unwrap_or_default() };

    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0)
    };

    match hook {
        Ok(h) => {
            println!("[成功] 键盘钩子已安装，开始监听按键...\n");

            while RUNNING.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(50));
            }

            unsafe { UnhookWindowsHookEx(h); }
            println!("\n[成功] 键盘钩子已卸载，程序退出。");
        }
        Err(e) => {
            eprintln!("[错误] 安装键盘钩子失败：{:?}", e);
            eprintln!("请以管理员权限运行此程序。");
            eprintln!("按回车退出...");
            let mut s = String::new();
            io::stdin().read_line(&mut s)?;
        }
    }

    Ok(())
}