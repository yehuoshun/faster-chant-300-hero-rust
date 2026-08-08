// 按键状态机
// 对应原 C++ key.cpp 中的 CKey::get() 逻辑
// 状态：首页 → 二级面板 → 搜索页 → 连发

use crate::scheme::SchemeManager;

/// 面板页面
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    /// 首页（显示 1-9 条快捷发言）
    Home,
    /// 二级面板（1-9），显示该索引下的 10 条发言
    Secondary(u8),
    /// 搜索页面（切换方案）
    Search,
}

/// 搜索模式
#[derive(Debug, Clone)]
enum SearchMode {
    /// 按编号搜索（输入了数字）
    ById(String),
    /// 按拼音搜索（输入了字母）
    BySpell(String),
}

/// 按键处理结果
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// 无操作
    None,
    /// 切换页面
    SwitchPage(Page),
    /// 切换方案
    SwitchScheme(u8),
    /// 发送消息
    SendMessage(String),
    /// 触发连发
    StartBurst(u8, u8), // (方案编号, 二级面板索引)
    /// 设置连发间隔
    SetBurstInterval(u8), // 0=关闭, 1-9=秒
    /// 关闭面板
    Close,
    /// 更新搜索结果显示
    UpdateSearch(String, Vec<(u8, String)>), // (输入内容, 搜索结果)
}

/// 按键状态机
pub struct StateMachine {
    page: Page,
    scheme_id: u8,
    search_mode: Option<SearchMode>,
    /// 空格是否按下（用于连发快捷键）
    space_held: bool,
    /// 连发间隔
    burst_interval: u8,
    /// 二级面板是否启用
    use_secondary: bool,
    /// 是否自动切回首页
    auto_back: bool,
}

impl StateMachine {
    pub fn new(scheme_id: u8, use_secondary: bool, burst_interval: u8, auto_back: bool) -> Self {
        Self {
            page: Page::Home,
            scheme_id,
            search_mode: None,
            space_held: false,
            burst_interval,
            use_secondary,
            auto_back,
        }
    }

    pub fn page(&self) -> &Page {
        &self.page
    }

    pub fn scheme_id(&self) -> u8 {
        self.scheme_id
    }

    /// 更新状态（方案切换时）
    pub fn update_config(&mut self, scheme_id: u8, use_secondary: bool, burst_interval: u8, auto_back: bool) {
        self.scheme_id = scheme_id;
        self.use_secondary = use_secondary;
        self.burst_interval = burst_interval;
        self.auto_back = auto_back;
    }

    /// 空格键状态
    pub fn set_space(&mut self, held: bool) {
        self.space_held = held;
    }

    /// 处理按键输入，返回操作结果
    pub fn handle_key(&mut self, vk: u32, schemes: &SchemeManager) -> ActionResult {
        // 数字键：ASCII 48-57 ('0'-'9') 或 小键盘 96-105
        let num = if (48..=57).contains(&vk) {
            Some((vk - 48) as u8)
        } else if (96..=105).contains(&vk) {
            Some((vk - 96) as u8)
        } else {
            None
        };

        // 字母键：ASCII 65-90 ('A'-'Z')
        let alpha = if (65..=90).contains(&vk) {
            Some(vk as u8 as char)
        } else {
            None
        };

        match &self.page {
            Page::Home => {
                // 空格 + 数字 = 设置连发间隔
                if self.space_held {
                    if let Some(n) = num {
                        self.burst_interval = n;
                        return ActionResult::SetBurstInterval(n);
                    }
                    return ActionResult::None;
                }

                match num {
                    Some(0) => {
                        // 0 键 → 搜索页
                        self.page = Page::Search;
                        self.search_mode = None;
                        return ActionResult::SwitchPage(Page::Search);
                    }
                    Some(n @ 1..=9) => {
                        if self.burst_interval > 0 {
                            // 连发模式
                            return ActionResult::StartBurst(self.scheme_id, n);
                        } else if self.use_secondary {
                            // 进入二级面板
                            self.page = Page::Secondary(n);
                            return ActionResult::SwitchPage(Page::Secondary(n));
                        } else {
                            // 直接发送一级面板内容
                            if let Some(scheme) = schemes.get(self.scheme_id) {
                                let msg = scheme.primary[n as usize - 1].clone();
                                if !msg.is_empty() {
                                    return ActionResult::SendMessage(msg);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                ActionResult::None
            }

            Page::Secondary(_) => {
                if let Some(n) = num {
                    if let Some(scheme) = schemes.get(self.scheme_id) {
                        let idx = self.page_secondary_index() as usize - 1;
                        let msg = scheme.secondary[idx][n as usize].clone();
                        if !msg.is_empty() {
                            if self.auto_back {
                                self.page = Page::Home;
                            }
                            return ActionResult::SendMessage(msg);
                        }
                    }
                }
                ActionResult::None
            }

            Page::Search => {
                match &self.search_mode {
                    None => {
                        // 第一次输入，决定搜索模式
                        if let Some(n) = num {
                            // 按编号搜索
                            let tens = n;
                            self.search_mode = Some(SearchMode::ById(tens.to_string()));
                            let results = schemes.find_by_tens(tens);
                            let display: Vec<_> = results
                                .iter()
                                .map(|(id, s)| (*id, s.name.clone()))
                                .collect();
                            return ActionResult::UpdateSearch(
                                format!("{}_", tens),
                                display,
                            );
                        } else if let Some(c) = alpha {
                            // 按拼音搜索
                            let spell = c.to_uppercase().to_string();
                            self.search_mode = Some(SearchMode::BySpell(spell.clone()));
                            let results = schemes.find_by_spell(&spell);
                            let display: Vec<_> = results
                                .iter()
                                .map(|(id, s)| (*id, s.name.clone()))
                                .collect();
                            return ActionResult::UpdateSearch(spell, display);
                        }
                    }
                    Some(SearchMode::ById(input)) => {
                        // 退格 → 清空
                        if vk == 0x08 {
                            self.search_mode = None;
                            return ActionResult::UpdateSearch(String::new(), vec![]);
                        }
                        // 数字 → 选中方案
                        if let Some(n) = num {
                            let new_id = input.parse::<u8>().unwrap_or(0) * 10 + n;
                            if schemes.contains(new_id) {
                                self.scheme_id = new_id;
                                self.page = Page::Home;
                                self.search_mode = None;
                                return ActionResult::SwitchScheme(new_id);
                            }
                        }
                    }
                    Some(SearchMode::BySpell(input)) => {
                        // 退格
                        if vk == 0x08 {
                            if input.len() <= 1 {
                                self.search_mode = None;
                                return ActionResult::UpdateSearch(String::new(), vec![]);
                            } else {
                                let new_spell = input[..input.len() - 1].to_string();
                                let results = schemes.find_by_spell(&new_spell);
                                let display: Vec<_> = results
                                    .iter()
                                    .map(|(id, s)| (*id, s.name.clone()))
                                    .collect();
                                self.search_mode = Some(SearchMode::BySpell(new_spell.clone()));
                                return ActionResult::UpdateSearch(new_spell, display);
                            }
                        }
                        // 字母 → 继续拼写（限长 16，防止无限增长）
                        if let Some(c) = alpha {
                            if input.chars().count() >= 16 {
                                return ActionResult::None;
                            }
                            let new_spell = format!("{}{}", input, c);
                            let results = schemes.find_by_spell(&new_spell);
                            let display: Vec<_> = results
                                .iter()
                                .map(|(id, s)| (*id, s.name.clone()))
                                .collect();
                            self.search_mode = Some(SearchMode::BySpell(new_spell.clone()));
                            return ActionResult::UpdateSearch(new_spell, display);
                        }
                        // 数字 → 选中搜索结果（显示为 1-based，索引需 -1）
                        if let Some(n) = num {
                            if n >= 1 && n <= 9 {
                                let results = schemes.find_by_spell(input);
                                if let Some((id, _)) = results.get((n - 1) as usize) {
                                    self.scheme_id = *id;
                                    self.page = Page::Home;
                                    self.search_mode = None;
                                    return ActionResult::SwitchScheme(*id);
                                }
                            }
                        }
                    }
                }
                ActionResult::None
            }
        }
    }

    /// 关闭面板，重置状态
    pub fn reset(&mut self) {
        self.page = Page::Home;
        self.search_mode = None;
        self.space_held = false;
    }

    fn page_secondary_index(&self) -> u8 {
        match self.page {
            Page::Secondary(n) => n,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests;
