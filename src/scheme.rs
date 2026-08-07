// 方案数据模型
// 99个方案槽位，每个最多 9 条一级 + 90 条二级发言

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// 单个方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheme {
    pub name: String,                        // 方案名称
    pub use_secondary: bool,                 // 是否启用二级面板
    pub primary: [String; 9],                // 一级面板 1-9
    pub secondary: [[String; 10]; 9],        // 二级面板 1-9, 每面板 0-9
}

impl Default for Scheme {
    fn default() -> Self {
        Self {
            name: String::new(),
            use_secondary: false,
            primary: Default::default(),
            secondary: Default::default(),
        }
    }
}

impl Scheme {
    /// 计算拼音首字母（用于搜索），由 search 模块提供
    pub fn spell(&self) -> String {
        crate::search::to_spell(&self.name)
    }
}

/// 全局方案数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemeData {
    active: u8,                              // 当前激活方案编号
    schemes: BTreeMap<u8, Scheme>,           // 编号 -> 方案
}

/// 方案管理器
pub struct SchemeManager {
    dir: PathBuf,
    data: SchemeData,
}

impl SchemeManager {
    /// 初始化，从目录读取所有方案
    pub fn init(dir: PathBuf) -> Self {
        let path = dir.join("schemes.json");

        let data = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| SchemeData {
                    active: 0,
                    schemes: BTreeMap::new(),
                })
        } else {
            SchemeData {
                active: 0,
                schemes: BTreeMap::new(),
            }
        };

        crate::logger::info(&format!(
            "加载方案: {} 个, 激活: {}",
            data.schemes.len(),
            data.active
        ));

        Self { dir, data }
    }

    /// 持久化到文件
    fn save(&self) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            if let Err(e) = std::fs::write(self.dir.join("schemes.json"), &json) {
                crate::logger::error(&format!("保存方案失败: {}", e));
            }
        }
    }

    // === 查询 ===

    pub fn get(&self, id: u8) -> Option<&Scheme> {
        self.data.schemes.get(&id)
    }

    pub fn contains(&self, id: u8) -> bool {
        self.data.schemes.contains_key(&id)
    }

    pub fn active(&self) -> u8 {
        self.data.active
    }

    pub fn is_empty(&self) -> bool {
        self.data.schemes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.schemes.len()
    }

    /// 获取最小编号
    pub fn front_id(&self) -> Option<u8> {
        self.data.schemes.keys().next().copied()
    }

    /// 获取一个未使用的编号（0-99），None 表示已满
    pub fn blank_id(&self) -> Option<u8> {
        (0..100).find(|id| !self.data.schemes.contains_key(id))
    }

    /// 所有方案引用
    pub fn list(&self) -> impl Iterator<Item = (u8, &Scheme)> {
        self.data.schemes.iter().map(|(k, v)| (*k, v))
    }

    // === 修改 ===

    /// 新增方案
    pub fn append(&mut self, id: u8) -> &mut Scheme {
        crate::logger::info(&format!("新增方案: {}", id));
        self.data.schemes.entry(id).or_default();
        self.save();
        self.data.schemes.get_mut(&id).unwrap()
    }

    /// 更新方案（覆盖）
    pub fn update(&mut self, id: u8, scheme: Scheme) {
        crate::logger::info(&format!("更新方案: {}", id));
        self.data.schemes.insert(id, scheme);
        self.save();
    }

    /// 删除方案
    pub fn remove(&mut self, id: u8) -> bool {
        let existed = self.data.schemes.remove(&id).is_some();
        if existed {
            crate::logger::info(&format!("删除方案: {}", id));
            if self.data.active == id {
                self.data.active = self.front_id().unwrap_or(0);
            }
            self.save();
        }
        existed
    }

    /// 重命名方案（改编号）
    pub fn rename(&mut self, old_id: u8, new_id: u8) -> bool {
        if old_id == new_id || !self.contains(old_id) || self.contains(new_id) {
            return false;
        }
        if let Some(scheme) = self.data.schemes.remove(&old_id) {
            crate::logger::info(&format!("重命名方案: {} -> {}", old_id, new_id));
            self.data.schemes.insert(new_id, scheme);
            if self.data.active == old_id {
                self.data.active = new_id;
            }
            self.save();
            true
        } else {
            false
        }
    }

    /// 设置激活方案
    pub fn set_active(&mut self, id: u8) -> bool {
        if self.contains(id) {
            self.data.active = id;
            self.save();
            true
        } else {
            false
        }
    }

    // === 搜索 ===

    /// 按编号十位数筛选（搜索用）
    pub fn find_by_tens(&self, tens: u8) -> Vec<(u8, &Scheme)> {
        self.data
            .schemes
            .iter()
            .filter(|(id, _)| (*id / 10) == tens)
            .map(|(k, v)| (*k, v))
            .collect()
    }

    /// 按拼音首字母前缀搜索
    pub fn find_by_spell(&self, spell: &str) -> Vec<(u8, &Scheme)> {
        let upper = spell.to_uppercase();
        self.data
            .schemes
            .iter()
            .filter(|(_, s)| s.spell().starts_with(&upper))
            .map(|(k, v)| (*k, v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_dir() -> PathBuf {
        env::temp_dir().join("fcd-test-schemes")
    }

    fn setup() -> SchemeManager {
        let dir = test_dir();
        let _ = std::fs::remove_dir_all(&dir);
        SchemeManager::init(dir)
    }

    #[test]
    fn test_append_and_get() {
        let mut mgr = setup();
        assert!(mgr.is_empty());

        let s = mgr.append(0);
        s.name = "测试方案".into();
        s.primary[0] = "你好".into();
        drop(s);

        let s = mgr.get(0).unwrap();
        assert_eq!(s.name, "测试方案");
        assert_eq!(s.primary[0], "你好");
    }

    #[test]
    fn test_blank_id() {
        let mut mgr = setup();
        assert_eq!(mgr.blank_id(), Some(0));

        mgr.append(0);
        mgr.append(99);
        assert_eq!(mgr.blank_id(), Some(1));

        // 填满 0-99
        for i in 0..100 {
            mgr.append(i);
        }
        assert_eq!(mgr.blank_id(), None);
    }

    #[test]
    fn test_remove_and_front() {
        let mut mgr = setup();
        mgr.append(5);
        mgr.append(3);
        mgr.append(7);

        assert_eq!(mgr.front_id(), Some(3)); // BTreeMap 有序

        mgr.remove(3);
        assert!(!mgr.contains(3));
        assert_eq!(mgr.front_id(), Some(5));
    }

    #[test]
    fn test_rename() {
        let mut mgr = setup();
        mgr.append(1);
        mgr.set_active(1);

        assert!(mgr.rename(1, 10));
        assert!(!mgr.contains(1));
        assert!(mgr.contains(10));
        assert_eq!(mgr.active(), 10);

        // 不能重名到已存在的编号
        mgr.append(20);
        assert!(!mgr.rename(10, 20));
    }

    #[test]
    fn test_find_by_tens() {
        let mut mgr = setup();
        for id in [10, 12, 15, 20, 21] {
            let s = mgr.append(id);
            s.name = format!("方案{}", id);
        }

        let found = mgr.find_by_tens(1);
        assert_eq!(found.len(), 3);
        let ids: Vec<u8> = found.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&10));
        assert!(ids.contains(&12));
        assert!(ids.contains(&15));
    }

    #[test]
    fn test_persistence() {
        let dir = test_dir();
        let _ = std::fs::remove_dir_all(&dir);

        {
            let mut mgr = SchemeManager::init(dir.clone());
            let s = mgr.append(0);
            s.name = "持久化测试".into();
            s.primary[0] = "数据".into();
            mgr.set_active(0);
        } // mgr 析构，数据已保存

        {
            let mgr = SchemeManager::init(dir.clone());
            assert_eq!(mgr.len(), 1);
            let s = mgr.get(0).unwrap();
            assert_eq!(s.name, "持久化测试");
            assert_eq!(s.primary[0], "数据");
            assert_eq!(mgr.active(), 0);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}