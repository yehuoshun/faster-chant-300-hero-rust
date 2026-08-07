// 输入模拟模块
// 使用 SendInput 替代 keybd_event（Windows 推荐方式）
// 模拟按键序列：回车→Ctrl+V→回车（游戏模式）/ Ctrl+V（聊天模式）

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LCONTROL, VK_RETURN, VK_RSHIFT, VK_V,
};

/// 发送消息（模拟键盘输入）
pub fn send_message(text: &str, public_chat: bool, chat_mode: bool) {
    crate::logger::info(&format!("发送: {}...", &text[..text.len().min(20)]));

    if let Err(e) = set_clipboard(text) {
        crate::logger::error(&format!("剪贴板失败: {}", e));
        return;
    }

    std::thread::sleep(std::time::Duration::from_millis(30));

    if !chat_mode {
        if public_chat {
            key_down(VK_RSHIFT);
        }
        key_press(VK_RETURN);
        if public_chat {
            key_up(VK_RSHIFT);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Ctrl+V
    key_down(VK_LCONTROL);
    key_press(VK_V);
    key_up(VK_LCONTROL);

    std::thread::sleep(std::time::Duration::from_millis(20));

    if !chat_mode {
        key_press(VK_RETURN);
    }

    crate::logger::debug("消息已发送");
}

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

/// 设置剪贴板文本（Win32 API）
fn set_clipboard(text: &str) -> Result<(), String> {
    use std::ptr;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GlobalFree, GMEM_MOVEABLE,
    };
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_size = wide.len() * 2;

    unsafe {
        OpenClipboard(HWND::default()).map_err(|e| format!("OpenClipboard: {:?}", e))?;
        let _ = EmptyClipboard();

        let handle = GlobalAlloc(GMEM_MOVEABLE, byte_size)
            .map_err(|e| format!("GlobalAlloc: {:?}", e))?;

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            let _ = GlobalFree(handle);
            let _ = CloseClipboard();
            return Err("GlobalLock failed".into());
        }

        ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, byte_size);

        let _ = GlobalUnlock(handle);

        SetClipboardData(CF_UNICODETEXT.0 as u32, handle)
            .map_err(|e| format!("SetClipboardData: {:?}", e))?;

        CloseClipboard().map_err(|e| format!("CloseClipboard: {:?}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vk_values() {
        assert_eq!(VK_RETURN.0, 0x0D);
        assert_eq!(VK_V.0, 0x56);
        assert_eq!(VK_LCONTROL.0, 0xA2);
        assert_eq!(VK_RSHIFT.0, 0xA1);
    }
}