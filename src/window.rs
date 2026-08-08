// 窗口检测与面板定位

use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, CloseHandle};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

use crate::overlay::Overlay;

/// 检测前台窗口是否为游戏窗口
/// only_300=true: 严格校验前台窗口所属进程的 exe 名包含 "300"
/// only_300=false: 退化为标题包含 "300" 的宽松检测
pub fn find_game_window(only_300: bool) -> Option<HWND> {
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

/// 面板贴到游戏窗口内测（左/右由配置决定）
pub fn position_overlay(overlay: &Overlay, game_hwnd: HWND, panel_left: bool) {
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
