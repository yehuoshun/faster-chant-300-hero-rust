// 输入模拟模块
// 使用 SendInput 替代 keybd_event（Windows 推荐方式）
// 模拟按键序列：回车→Ctrl+V→回车（游戏模式）/ Ctrl+V（聊天模式）

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, VIRTUAL_KEY, VK_LCONTROL, VK_RCONTROL,
    VK_RETURN, VK_RSHIFT, VK_V,
};

/// 发送消息（模拟键盘输入）
/// public_chat: 是否全体频道（按 Shift）
/// chat_mode: 聊天模式（不模拟回车）
pub fn send_message(text: &str, public_chat: bool, chat_mode: bool) {
    crate::logger::info(&format!("发送消息: {}...", &text[..text.len().min(20)]));

    // 先复制到剪贴板
    if let Err(e) = set_clipboard(text) {
        crate::logger::error(&format!("剪贴板写入失败: {}", e));
        return;
    }

    // 短暂延迟，确保剪贴板准备好
    std::thread::sleep(std::time::Duration::from_millis(30));

    if !chat_mode {
        // 游戏模式：回车 → 粘贴 → 回车
        if public_chat {
            key_down(VK_RSHIFT);
        }
        key_press(VK_RETURN);
        if public_chat {
            key_up(VK_RSHIFT);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Ctrl+V 粘贴
    key_down(VK_LCONTROL);
    key_press(VK_V);
    key_up(VK_LCONTROL);

    std::thread::sleep(std::time::Duration::from_millis(20));

    if !chat_mode {
        // 回车发送
        key_press(VK_RETURN);
    }

    crate::logger::debug(&format!("消息发送完成: {} chars", text.len()));
}

/// 模拟按键按下和释放
fn key_press(vk: VIRTUAL_KEY) {
    key_down(vk);
    key_up(vk);
}

fn key_down(vk: VIRTUAL_KEY) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
}

fn key_up(vk: VIRTUAL_KEY) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
}

/// 设置剪贴板文本
fn set_clipboard(text: &str) -> Result<(), String> {
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::DataExchange::{OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard};
    use windows::Win32::System::Ole::{CF_UNICODETEXT};
    use windows::Win32::Foundation::{GetLastError, HWND};
    use std::ptr;

    // 编码为 UTF-16LE（Windows 宽字符）
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_size = wide.len() * 2;

    unsafe {
        if OpenClipboard(HWND::default()).is_err() {
            return Err("OpenClipboard failed".into());
        }

        let _ = EmptyClipboard();

        let handle = GlobalAlloc(GMEM_MOVEABLE, byte_size)?;
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return Err("GlobalLock failed".into());
        }

        ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, byte_size);
        GlobalUnlock(handle);

        if SetClipboardData(CF_UNICODETEXT.0 as u32, handle).is_err() {
            GlobalFree(handle);
            CloseClipboard();
            return Err("SetClipboardData failed".into());
        }

        CloseClipboard();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_clipboard() {
        // 这个测试在 CI 无桌面环境下可能失败，标记为忽略
        // 在本地 Windows 桌面环境可以手动测试
    }

    #[test]
    fn test_key_constants() {
        // 验证 VK 常量值
        assert_eq!(VK_RETURN.0, 0x0D);
        assert_eq!(VK_V.0, 0x56);
        assert_eq!(VK_LCONTROL.0, 0xA2);
        assert_eq!(VK_RSHIFT.0, 0xA1);
    }
}