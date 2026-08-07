// 悬浮窗模块
// 透明分层窗口 + GDI 文字渲染

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// 悬浮窗
pub struct Overlay {
    hwnd: HWND,
    width: i32,
    height: i32,
}

impl Overlay {
    /// 创建悬浮窗
    pub fn new() -> Option<Self> {
        let hinstance = unsafe { GetModuleHandleW(None).ok()? };

        // 注册窗口类
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(hinstance.0),
            hIcon: unsafe { LoadIconW(None, IDI_APPLICATION).ok() }.unwrap_or_default(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).ok() }.unwrap_or_default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: w!("FcdOverlayClass"),
        };

        unsafe { let _ = RegisterClassW(&wc); }

        // 创建分层窗口
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("FcdOverlayClass"),
                w!("FCD Overlay"),
                WS_POPUP,
                0, 0, 400, 300,
                None,
                None,
                hinstance,
                None,
            ).ok()?
        };

        // 设置窗口透明度
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_COLORKEY);
        }

        Some(Self { hwnd, width: 400, height: 300 })
    }

    /// 显示窗口
    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNA);
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    /// 隐藏窗口
    pub fn hide(&self) {
        unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
    }

    /// 设置位置
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x, y,
                self.width, self.height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    /// 更新内容
    pub fn update(&self, content: &OverlayContent) {
        self.render(content);
    }

    /// 渲染
    fn render(&self, content: &OverlayContent) {
        unsafe {
            let hdc_window = GetDC(self.hwnd);
            if hdc_window.is_invalid() { return; }

            let hdc_mem = CreateCompatibleDC(hdc_window);
            let hbitmap = CreateCompatibleBitmap(hdc_window, self.width, self.height);
            let _old_bmp = SelectObject(hdc_mem, hbitmap);

            // 背景
            let brush = CreateSolidBrush(COLORREF(0x80000000));
            let _ = FillRect(hdc_mem, &RECT { left: 0, top: 0, right: self.width, bottom: self.height }, brush);
            let _ = DeleteObject(brush);

            // 字体
            let font = CreateFontW(
                24, 0, 0, 0, // cheight, cwidth, cescapement, corientation
                0, 0, 0, // bitalic, bunderline, cstrikeout
                FW_BOLD.0, // cweight
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                DEFAULT_PITCH.0 as u32,
                w!("Microsoft YaHei"),
            );
            let _old_font = SelectObject(hdc_mem, font);

            let _ = SetBkMode(hdc_mem, BACKGROUND_MODE(1));
            let _ = SetTextColor(hdc_mem, COLORREF(0xFFFFFFFF));

            match content {
                OverlayContent::Home { items, active: _ } => {
                    for (i, item) in items.iter().enumerate().take(9) {
                        let text = format!("{}. {}", i + 1, item);
                        let y = 10 + i as i32 * 30;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
                OverlayContent::Secondary { index, items } => {
                    let title = format!("二级面板 {}", index);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, item) in items.iter().enumerate().take(10) {
                        let text = format!("{}. {}", i, item);
                        let y = 40 + i as i32 * 25;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
                OverlayContent::Search { query, results } => {
                    let title = format!("搜索: {}", query);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, (id, name)) in results.iter().enumerate().take(9) {
                        let text = format!("{}. {} [{}]", i + 1, name, id);
                        let y = 40 + i as i32 * 25;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
            }

            // UpdateLayeredWindow
            let size = SIZE { cx: self.width, cy: self.height };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 200,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                hdc_window,
                Some(&pt_src),
                Some(&size),
                hdc_mem,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            // 清理
            let _ = SelectObject(hdc_mem, _old_font);
            let _ = DeleteObject(font);
            let _ = SelectObject(hdc_mem, _old_bmp);
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(self.hwnd, hdc_window);
        }
    }

    fn draw_text(&self, hdc: HDC, text: &str, x: i32, y: i32) {
        let wide: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            // 描边
            let _ = SetTextColor(hdc, COLORREF(0xFF000000));
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)] {
                let _ = TextOutW(hdc, x + dx, y + dy, &wide);
            }
            // 填充
            let _ = SetTextColor(hdc, COLORREF(0xFFFFFFFF));
            let _ = TextOutW(hdc, x, y, &wide);
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_DESTROY => { unsafe { let _ = PostQuitMessage(0); } LRESULT(0) }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}

pub enum OverlayContent {
    Home { items: Vec<String>, active: u8 },
    Secondary { index: u8, items: [String; 10] },
    Search { query: String, results: Vec<(u8, String)> },
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_content_creation() {
        use super::OverlayContent;
        let c = OverlayContent::Home { items: vec!["test".into()], active: 0 };
        assert!(matches!(c, OverlayContent::Home { .. }));
    }
}