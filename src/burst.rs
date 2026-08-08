// 连发模式模块
// 按间隔逐条发送二级面板中的消息

use crate::scheme::SchemeManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 连发控制器
pub struct BurstController {
    running: Arc<AtomicBool>,
}

impl BurstController {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 是否正在连发
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 停止连发
    pub fn stop(&mut self) {
        if self.is_running() {
            self.running.store(false, Ordering::SeqCst);
            crate::logger::info("连发已停止");
        }
    }

    /// 开始连发
    /// scheme_id: 方案编号
    /// secondary_index: 二级面板索引 (1-9)
    /// interval_secs: 间隔秒数 (1-9)
    /// public_chat: 是否全体频道
    /// chat_mode: 是否聊天模式
    pub fn start(
        &mut self,
        scheme_id: u8,
        secondary_index: u8,
        interval_secs: u8,
        public_chat: bool,
        chat_mode: bool,
        // 需要从外部传入方案数据（因为自引用问题）
        secondary_items: Vec<String>,
    ) {
        if self.is_running() {
            crate::logger::warn("连发已在运行中，忽略新请求");
            return;
        }

        if interval_secs == 0 || secondary_items.is_empty() {
            return;
        }

        crate::logger::info(&format!(
            "开始连发: 方案{}, 二级面板{}, 共{}条, 间隔{}秒",
            scheme_id,
            secondary_index,
            secondary_items.len(),
            interval_secs
        ));

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        thread::spawn(move || {
            for (i, item) in secondary_items.iter().enumerate() {
                if !running.load(Ordering::SeqCst) {
                    crate::logger::info("连发被中断");
                    return;
                }

                if item.is_empty() {
                    continue;
                }

                crate::logger::debug(&format!("连发 [{}/{}]: {}", i + 1, secondary_items.len(), item));

                crate::input::send_message(item, public_chat, chat_mode);

                // 检查是否还有下一条非空消息
                let has_next = secondary_items[i + 1..]
                    .iter()
                    .any(|s| !s.is_empty());

                if has_next {
                    // 等待间隔
                    for _ in 0..(interval_secs as u64 * 10) {
                        if !running.load(Ordering::SeqCst) {
                            return;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            running.store(false, Ordering::SeqCst);
            crate::logger::info("连发完成");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_not_running() {
        let bc = BurstController::new();
        assert!(!bc.is_running());
    }

    #[test]
    fn test_stop_when_not_running() {
        let mut bc = BurstController::new();
        bc.stop(); // 不应 panic
        assert!(!bc.is_running());
    }

    #[test]
    fn test_start_empty() {
        let mut bc = BurstController::new();
        bc.start(0, 1, 1, false, false, vec![]);
        assert!(!bc.is_running()); // 空列表不启动
    }

    #[test]
    fn test_start_zero_interval() {
        let mut bc = BurstController::new();
        bc.start(0, 1, 0, false, false, vec!["test".into()]);
        assert!(!bc.is_running()); // 间隔 0 不启动
    }

    #[test]
    fn test_start_and_stop() {
        let mut bc = BurstController::new();
        bc.start(0, 1, 1, false, false, vec!["msg1".into(), "msg2".into()]);
        assert!(bc.is_running());
        bc.stop();
        // 给线程一点时间停止
        std::thread::sleep(Duration::from_millis(100));
        assert!(!bc.is_running());
    }
}