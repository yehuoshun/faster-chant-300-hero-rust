// 阶段 0：键盘钩子最小验证
// 功能：安装全局键盘钩子，打印按键码，按 Esc 退出
// 编译：cargo build --release
// 运行：需要管理员权限（否则无法捕获以管理员运行的程序按键）

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, HHOOK,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    VK_ESCAPE, VK_RETURN, VK_SPACE, VK_BACK, VK_SHIFT, VK_CONTROL, VK_MENU,
    VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL, VK_LMENU, VK_RMENU,
    VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7,
    VK_OEM_PLUS, VK_OEM_MINUS, VK_OEM_COMMA, VK_OEM_PERIOD,
    VK_CAPITAL, VK_TAB,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

/// 按键名称映射
fn key_name(vk_code: u32) -> &'static str {
    match vk_code {
        v if v == VK_ESCAPE.0 as u32 => "Esc",
        v if v == VK_RETURN.0 as u32 => "Enter",
        v if v == VK_SPACE.0 as u32 => "Space",
        v if v == VK_BACK.0 as u32 => "Backspace",
        v if v == VK_TAB.0 as u32 => "Tab",
        v if v == VK_CAPITAL.0 as u32 => "CapsLock",
        v if v == VK_CONTROL.0 as u32 || v == VK_LCONTROL.0 as u32 || v == VK_RCONTROL.0 as u32 => "Ctrl",
        v if v == VK_MENU.0 as u32 || v == VK_LMENU.0 as u32 || v == VK_RMENU.0 as u32 => "Alt",
        v if v == VK_SHIFT.0 as u32 || v == VK_LSHIFT.0 as u32 || v == VK_RSHIFT.0 as u32 => "Shift",
        v if v == VK_OEM_1.0 as u32 => ";",
        v if v == VK_OEM_2.0 as u32 => "/",
        v if v == VK_OEM_3.0 as u32 => "~",
        v if v == VK_OEM_4.0 as u32 => "[",
        v if v == VK_OEM_5.0 as u32 => "\\",
        v if v == VK_OEM_6.0 as u32 => "]",
        v if v == VK_OEM_7.0 as u32 => "'",
        v if v == VK_OEM_PLUS.0 as u32 => "=",
        v if v == VK_OEM_MINUS.0 as u32 => "-",
        v if v == VK_OEM_COMMA.0 as u32 => ",",
        v if v == VK_OEM_PERIOD.0 as u32 => ".",
        v if (0x30..=0x39).contains(&v) => "数字键",
        v if (0x41..=0x5A).contains(&v) => "字母键",
        v if (0x60..=0x69).contains(&v) => "小键盘数字",
        v if (0x70..=0x7B).contains(&v) => "功能键",
        _ => "其他",
    }
}

extern "system" fn keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code >= 0 {
        let kb = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode;

        match w_param as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let name = key_name(vk);
                println!(
                    "[按下] vkCode={:3} (0x{:02X}) | {}",
                    vk, vk, name
                );

                if vk == VK_ESCAPE.0 as u32 {
                    println!(">>> 检测到 Esc，退出中...");
                    RUNNING.store(false, Ordering::SeqCst);
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                // 只打印，不处理
                let name = key_name(vk);
                println!(
                    "[释放] vkCode={:3} (0x{:02X}) | {}",
                    vk, vk, name
                );
            }
            _ => {}
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn main() -> io::Result<()> {
    println!("============================================");
    println!("  300高速咏唱装置 - 阶段0：键盘钩子验证");
    println!("  按任意键查看按键码，按 Esc 退出");
    println!("  注意：需要管理员权限运行");
    println!("============================================\n");

    // 安装键盘钩子
    let hinst = unsafe { GetModuleHandleW(None).unwrap_or(HINSTANCE::default()) };

    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0)
    };

    if hook.is_invalid() {
        eprintln!("[错误] 安装键盘钩子失败！请以管理员权限运行。");
        eprintln!("按回车退出...");
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        return Ok(());
    }

    println!("[成功] 键盘钩子已安装，开始监听按键...\n");

    // 消息循环：直到 Esc 按下
    while RUNNING.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(50));
    }

    // 卸载钩子
    unsafe { UnhookWindowsHookEx(hook); }
    println!("\n[成功] 键盘钩子已卸载，程序退出。");

    Ok(())
}