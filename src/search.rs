// 拼音搜索模块
// 20902 汉字拼音首字母映射表（Unicode 0x4E00-0x9FA5）

/// 汉字转拼音首字母
/// 参数: 中文字符串（可含非中文字符和非 CJK 字符）
/// 返回: 拼音首字母大写字符串（忽略非 CJK 字符）
pub fn to_spell(text: &str) -> String {
    text.chars().filter_map(|c| pinyin_first(c)).collect()
}

/// 单字转拼音首字母
fn pinyin_first(c: char) -> Option<char> {
    let code = c as u32;
    if (0x4E00..=0x9FA5).contains(&code) {
        let idx = (code - 0x4E00) as usize;
        if idx < PINYIN_TABLE.len() {
            Some(PINYIN_TABLE.as_bytes()[idx] as char)
        } else {
            None
        }
    } else {
        None
    }
}

/// 20902 汉字拼音首字母表
const PINYIN_TABLE: &str = include_str!("pinyin_table.txt");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_length() {
        assert_eq!(PINYIN_TABLE.len(), 20902);
    }

    #[test]
    fn test_to_spell_common() {
        // 常见汉字
        assert_eq!(to_spell("中国"), "ZG");
        assert_eq!(to_spell("北京"), "BJ");
        assert_eq!(to_spell("上海"), "SH");
        assert_eq!(to_spell("深圳"), "SZ"); // 圳 -> Z
        assert_eq!(to_spell("英雄"), "YX");
        assert_eq!(to_spell("三百"), "SB");
    }

    #[test]
    fn test_to_spell_mixed() {
        // 含非中文字符
        assert_eq!(to_spell("300英雄"), "YX");
        assert_eq!(to_spell("LOL"), "");
        assert_eq!(to_spell("测试123方案"), "CSFA");
    }

    #[test]
    fn test_to_spell_empty() {
        assert_eq!(to_spell(""), "");
        assert_eq!(to_spell("abc"), "");
        assert_eq!(to_spell("123"), "");
    }

    #[test]
    fn test_pinyin_first() {
        assert_eq!(pinyin_first('中'), Some('Z'));
        assert_eq!(pinyin_first('国'), Some('G'));
        assert_eq!(pinyin_first('A'), None);
        assert_eq!(pinyin_first('1'), None);
    }
}