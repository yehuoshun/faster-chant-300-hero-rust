// 全局配置模块
// 对应原 C++ 的：仅限三百、全体发言、显示位置、启动按键、热键屏蔽、切回首页、聊天模式、连发间隔

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 仅限 300 英雄（焦点窗口必须是 300.exe）
    pub only_300: bool,
    /// 全体发言（发送时按 Shift）
    pub public_chat: bool,
    /// 显示位置：true=左侧，false=右侧
    pub panel_left: bool,
    /// 启动按键 vkCode（默认 ~ 键 0xC0）
    pub trigger_key: u16,
    /// 屏蔽热键（激活时不让热键穿透到游戏）
    pub shield_hotkey: bool,
    /// 发送后自动切回首页
    pub auto_back: bool,
    /// 聊天模式（不模拟回车，只粘贴）
    pub chat_mode: bool,
    /// 连发间隔（0=关闭，1-9=秒）
    pub burst_interval: u8,
    /// 当前激活方案编号
    pub active_scheme: u8,
    /// 是否启用二级面板
    pub use_secondary: bool,

    // ── 面板样式 ──
    /// 面板宽度（像素）
    pub panel_width: u16,
    /// 背景颜色 RGB
    pub bg_color: [u8; 3],
    /// 背景透明度 0-255
    pub bg_alpha: u8,
    /// 字体族
    pub font_family: String,
    /// 字号（像素）
    pub font_size: u16,
    /// 字体加粗
    pub font_bold: bool,
    /// 文字颜色 RGB
    pub text_color: [u8; 3],
    /// 描边颜色 RGB
    pub outline_color: [u8; 3],
    /// 描边大小（像素）
    pub outline_size: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            only_300: true,
            public_chat: true,
            panel_left: true,
            trigger_key: 0xC0, // ~ 键
            shield_hotkey: false,
            auto_back: true,
            chat_mode: false,
            burst_interval: 0,
            active_scheme: 0,
            use_secondary: true,
            panel_width: 400,
            bg_color: [0, 0, 0],
            bg_alpha: 200,
            font_family: "Microsoft YaHei".into(),
            font_size: 24,
            font_bold: true,
            text_color: [255, 255, 255],
            outline_color: [0, 0, 0],
            outline_size: 1,
        }
    }
}

impl Config {
    /// 从文件加载，不存在则返回默认
    pub fn load(dir: &PathBuf) -> Self {
        let path = dir.join("config.json");
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            let cfg = Self::default();
            cfg.save(dir);
            crate::logger::info("创建默认配置文件");
            cfg
        }
    }

    /// 保存到文件
    pub fn save(&self, dir: &PathBuf) {
        let _ = std::fs::create_dir_all(dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("config.json"), &json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let cfg = Config::default();
        assert!(cfg.only_300);
        assert!(cfg.public_chat);
        assert_eq!(cfg.trigger_key, 0xC0);
        assert_eq!(cfg.burst_interval, 0);
    }

    #[test]
    fn test_save_load() {
        let dir = PathBuf::from("fcd-test-config");
        let _ = std::fs::remove_dir_all(&dir);

        let mut cfg = Config::default();
        cfg.only_300 = false;
        cfg.burst_interval = 3;
        cfg.save(&dir);

        let loaded = Config::load(&dir);
        assert!(!loaded.only_300);
        assert_eq!(loaded.burst_interval, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}