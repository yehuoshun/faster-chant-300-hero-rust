// 悬浮窗模块
// 透明分层窗口 + GDI 文字渲染（样式由 PanelStyle 控制）

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::config::Config;

/// 悬浮面板样式（来自 Config，用于 GDI 渲染）
#[derive(Debug, Clone)]
pub struct PanelStyle {
    pub width: i32,
    pub bg_color: [u8; 3],
    pub bg_alpha: u8,
    pub font_family: String,
    pub font_size: i32,
    pub font_bold: bool,
    pub text_color: [u8; 3],
    pub outline_color: [u8; 3],
    pub outline_size: u8,
}

impl PanelStyle {
    pub fn from_config(c: &Config) -> Self {
        Self {
            width: c.panel_width as i32,
            bg_color: c.bg_color,
            bg_alpha: c.bg_alpha,
            font_family: c.font_family.clone(),
            font_size: c.font_size as i32,
            font_bold: c.font_bold,
            text_color: c.text_color,
            outline_color: c.outline_color,
            outline_size: c.outline_size,
        }
    }

    /// 根据字号计算面板高度（容纳标题 + 10 行）
    fn height(&self) -> i32 {
        40 + 10 * (self.font_size + 6) + 16
    }

    fn row_h(&self) -> i32 {
        self.font_size + 6
    }
}

fn colorref(rgb: [u8; 3]) -> COLORREF {
    COLORREF(((rgb[2] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[0] as u32)
}

/// 悬浮窗
pub struct Overlay {
    hwnd: HWND,
    style: PanelStyle,
    width: i32,
    height: i32,
}

// HWND 是 Windows 句柄（裸指针包装），跨线程传递句柄本身是安全的。
// 否则 static STATE 中的 GlobalState 无法满足 Sync（Mutex 要求 T: Send）。
unsafe impl Send for Overlay {}
unsafe impl Sync for Overlay {}

impl Overlay {
    /// 创建悬浮窗
    pub fn new(style: &PanelStyle) -> Option<Self> {
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

        let width = style.width;
        let height = style.height();

        // 创建分层窗口
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("FcdOverlayClass"),
                w!("FCD Overlay"),
                WS_POPUP,
                0, 0, width, height,
                None,
                None,
                hinstance,
                None,
            ).ok()?
        };

        Some(Self { hwnd, style: style.clone(), width, height })
    }

    /// 更新样式（含尺寸变化时调整窗口大小）
    pub fn set_style(&mut self, style: &PanelStyle) {
        self.style = style.clone();
        let new_w = style.width;
        let new_h = style.height();
        if new_w != self.width || new_h != self.height {
            self.width = new_w;
            self.height = new_h;
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0, 0,
                    self.width, self.height,
                    SWP_NOMOVE | SWP_NOACTIVATE,
                );
            }
        }
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

    /// 面板宽度（供定位到游戏窗口旁使用）
    pub fn width(&self) -> i32 {
        self.width
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
            let brush = CreateSolidBrush(colorref(self.style.bg_color));
            let _ = FillRect(hdc_mem, &RECT { left: 0, top: 0, right: self.width, bottom: self.height }, brush);
            let _ = DeleteObject(brush);

            // 字体
            let weight = if self.style.font_bold { FW_BOLD.0 } else { FW_NORMAL.0 };
            let font_face: Vec<u16> = self.style.font_family.encode_utf16().chain(Some(0)).collect();
            let font = CreateFontW(
                self.style.font_size, 0, 0, 0, // cheight, cwidth, cescapement, corientation
                0, 0, 0, // bitalic, bunderline, cstrikeout
                weight, // cweight
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                DEFAULT_QUALITY.0 as u32,
                DEFAULT_PITCH.0 as u32,
                PCWSTR(font_face.as_ptr()),
            );
            let _old_font = SelectObject(hdc_mem, font);

            let _ = SetBkMode(hdc_mem, BACKGROUND_MODE(1));

            let row = self.style.row_h();
            match content {
                OverlayContent::Home { items, active, name } => {
                    let title = format!("方案{}: {}", active, name);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, item) in items.iter().enumerate().take(9) {
                        let text = format!("{}. {}", i + 1, item);
                        self.draw_text(hdc_mem, &text, 10, 40 + i as i32 * row);
                    }
                }
                OverlayContent::Secondary { index, items } => {
                    let title = format!("二级面板 {}", index);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, item) in items.iter().enumerate().take(10) {
                        let text = format!("{}. {}", i, item);
                        self.draw_text(hdc_mem, &text, 10, 40 + i as i32 * row);
                    }
                }
                OverlayContent::Search { query, results } => {
                    let title = format!("搜索: {}", query);
                    self.draw_text(hdc_mem, &title, 10, 10);
                    for (i, (id, name)) in results.iter().enumerate().take(9) {
                        let text = format!("{}. {} [{}]", i + 1, name, id);
                        self.draw_text(hdc_mem, &text, 10, 40 + i as i32 * row);
                    }
                }
            }

            // UpdateLayeredWindow
            let size = SIZE { cx: self.width, cy: self.height };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: self.style.bg_alpha,
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
        let n = self.style.outline_size.max(1) as i32;
        unsafe {
            // 描边（包围格子）
            let _ = SetTextColor(hdc, colorref(self.style.outline_color));
            for dx in -n..=n {
                for dy in -n..=n {
                    if dx == 0 && dy == 0 { continue; }
                    let _ = TextOutW(hdc, x + dx, y + dy, &wide);
                }
            }
            // 填充
            let _ = SetTextColor(hdc, colorref(self.style.text_color));
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
    Home { items: Vec<String>, active: u8, name: String },
    Secondary { index: u8, items: [String; 10] },
    Search { query: String, results: Vec<(u8, String)> },
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_content_creation() {
        use super::OverlayContent;
        let c = OverlayContent::Home { items: vec!["test".into()], active: 0, name: "方案".into() };
        assert!(matches!(c, OverlayContent::Home { .. }));
    }

    #[test]
    fn test_panel_style_height() {
        use super::PanelStyle;
        let s = PanelStyle {
            width: 400, bg_color: [0,0,0], bg_alpha: 200,
            font_family: "微软雅黑".into(), font_size: 24, font_bold: true,
            text_color: [255,255,255], outline_color: [0,0,0], outline_size: 1,
        };
        assert!(s.height() > 300);
    }
}
