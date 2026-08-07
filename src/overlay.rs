// 悬浮窗模块
// 透明分层窗口 + GDI 文字描边渲染

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
            hInstance: hinstance,
            hIcon: unsafe { LoadIconW(None, IDI_APPLICATION).ok() },
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).ok() },
            hbrBackground: HBRUSH(0),
            lpszMenuName: None,
            lpszClassName: w!("FcdOverlayClass"),
        };

        if unsafe { RegisterClassW(&wc) } == 0 {
            // 可能已注册
        }

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
            )
        };

        if hwnd.is_invalid() {
            return None;
        }

        // 设置窗口透明度
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_COLORKEY);
        }

        Some(Self {
            hwnd,
            width: 400,
            height: 300,
        })
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
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
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

    /// 渲染内容
    fn render(&self, content: &OverlayContent) {
        let width = self.width;
        let height = self.height;

        unsafe {
            // 获取窗口 DC
            let hdc_window = GetDC(self.hwnd);
            if hdc_window.is_invalid() {
                return;
            }

            // 创建兼容 DC 和位图
            let hdc_mem = CreateCompatibleDC(hdc_window);
            let hbitmap = CreateCompatibleBitmap(hdc_window, width, height);
            let _old_bmp = SelectObject(hdc_mem, hbitmap);

            // 创建画刷填充背景（半透明）
            let brush = CreateSolidBrush(COLORREF(0x80000000)); // ARGB: 黑色半透明
            let _ = FillRect(hdc_mem, &RECT {
                left: 0, top: 0,
                right: width, bottom: height,
            }, brush);
            let _ = DeleteObject(brush);

            // 创建字体
            let font = CreateFontW(
                24, 0, 0, 0, FW_BOLD, 0, 0, 0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                DEFAULT_QUALITY,
                DEFAULT_PITCH,
                w!("Microsoft YaHei"),
            );
            let _old_font = SelectObject(hdc_mem, font);

            // 设置文字颜色
            let _ = SetBkMode(hdc_mem, 1); // TRANSPARENT
            let _ = SetTextColor(hdc_mem, COLORREF(0xFFFFFFFF)); // 白色

            // 渲染内容
            match content {
                OverlayContent::Home { items, active } => {
                    for (i, item) in items.iter().enumerate() {
                        if i >= 9 { break; }
                        let text = format!("{}. {}", i + 1, item);
                        let y = 10 + i as i32 * 30;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
                OverlayContent::Secondary { index, items } => {
                    let title = format!("二级面板 {}", index);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, item) in items.iter().enumerate() {
                        if i >= 10 { break; }
                        let text = format!("{}. {}", i, item);
                        let y = 40 + i as i32 * 25;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
                OverlayContent::Search { query, results } => {
                    let title = format!("搜索: {}", query);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, (id, name)) in results.iter().enumerate() {
                        if i >= 9 { break; }
                        let text = format!("{}. {} [{}]", i + 1, name, id);
                        let y = 40 + i as i32 * 25;
                        self.draw_text(hdc_mem, &text, 10, y);
                    }
                }
            }

            // 使用 UpdateLayeredWindow 更新窗口
            let size = SIZE { cx: width, cy: height };
            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: 0, y: 0 }; // 需要用 GetWindowRect 获取实际位置
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER,
                BlendFlags: 0,
                SourceConstantAlpha: 200,
                AlphaFormat: AC_SRC_ALPHA,
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                hdc_window,
                &pt_dst,
                &size,
                hdc_mem,
                &pt_src,
                COLORREF(0),
                &blend,
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

    /// 绘制带描边的文字
    fn draw_text(&self, hdc: HDC, text: &str, x: i32, y: i32) {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            // 描边（黑色）
            let _ = SetTextColor(hdc, COLORREF(0xFF000000));
            let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)];
            for (dx, dy) in offsets {
                let _ = TextOutW(hdc, x + dx, y + dy, PCWSTR(wide.as_ptr()), text.len() as i32);
            }

            // 填充（白色）
            let _ = SetTextColor(hdc, COLORREF(0xFFFFFFFF));
            let _ = TextOutW(hdc, x, y, PCWSTR(wide.as_ptr()), text.len() as i32);
        }
    }

    /// 窗口过程
    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}

/// 悬浮窗内容
pub enum OverlayContent {
    /// 首页
    Home {
        items: Vec<String>,
        active: u8,
    },
    /// 二级面板
    Secondary {
        index: u8,
        items: [String; 10],
    },
    /// 搜索页
    Search {
        query: String,
        results: Vec<(u8, String)>,
    },
}

#[cfg(test)]
mod tests {
    // 在 CI 环境下无法测试窗口创建
    // 这些测试需要在有桌面环境的 Windows 上运行

    #[test]
    fn test_overlay_content_creation() {
        use super::OverlayContent;
        let content = OverlayContent::Home {
            items: vec!["测试".into()],
            active: 0,
        };
        assert!(matches!(content, OverlayContent::Home { .. }));
    }
}