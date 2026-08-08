// 输入模拟模块
// SendInput 模拟按键 + arboard 剪贴板

use std::sync::Mutex;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LCONTROL, VK_RETURN, VK_RSHIFT, VK_V,
};

/// 发送锁：串行化所有发送流程，避免手动发送与连发线程争抢剪贴板
static SEND_LOCK: once_cell::sync::Lazy<Mutex<()>> =
    once_cell::sync::Lazy::new(|| Mutex::new(()));

/// 截断日志文本，按字符边界安全截断（避免 UTF-8 切片 panic）
fn truncate_for_log(text: &str) -> String {
    text.chars().take(20).collect::<String>()
}

/// 发送消息
pub fn send_message(text: &str, public_chat: bool, chat_mode: bool) {
    crate::logger::info(&format!("发送: {}...", truncate_for_log(text)));

    // 持有锁直到整个发送序列完成，防止并发截胡剪贴板
    let _guard = SEND_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if let Err(e) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
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

    key_down(VK_LCONTROL);
    key_press(VK_V);
    key_up(VK_LCONTROL);

    std::thread::sleep(std::time::Duration::from_millis(20));

    if !chat_mode {
        key_press(VK_RETURN);
    }
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