    use super::*;
    use std::path::PathBuf;

    fn setup() -> (StateMachine, SchemeManager) {
        let dir = PathBuf::from("fcd-test-state");
        let _ = std::fs::remove_dir_all(&dir);
        let mut mgr = SchemeManager::init(dir);
        let s = mgr.append(0);
        s.name = "测试".into();
        s.primary[0] = "你好".into(); // 1键
        s.primary[1] = "大家好".into(); // 2键
        s.use_secondary = true;
        s.secondary[0][0] = "二级消息0".into();
        s.secondary[0][1] = "二级消息1".into();
        mgr.set_active(0);

        let sm = StateMachine::new(0, true, 0, true);
        (sm, mgr)
    }

    #[test]
    fn test_home_direct_send() {
        let (mut sm, mgr) = setup();
        let mut sm = StateMachine::new(0, false, 0, true); // 关二级面板
        match sm.handle_key('1' as u32, &mgr) {
            ActionResult::SendMessage(msg) => assert_eq!(msg, "你好"),
            _ => panic!("应该发送消息"),
        }
    }

    #[test]
    fn test_home_to_secondary() {
        let (mut sm, mgr) = setup();
        // 按 1 进入二级面板
        match sm.handle_key('1' as u32, &mgr) {
            ActionResult::SwitchPage(Page::Secondary(1)) => {}
            other => panic!("应该进入二级面板，实际: {:?}", other),
        }
        assert_eq!(*sm.page(), Page::Secondary(1));
    }

    #[test]
    fn test_secondary_send() {
        let (mut sm, mgr) = setup();
        sm.handle_key('1' as u32, &mgr); // 进入二级面板
        match sm.handle_key('0' as u32, &mgr) {
            ActionResult::SendMessage(msg) => assert_eq!(msg, "二级消息0"),
            other => panic!("应该发送二级消息，实际: {:?}", other),
        }
        // 自动回首页
        assert_eq!(*sm.page(), Page::Home);
    }

    #[test]
    fn test_home_to_search() {
        let (mut sm, mgr) = setup();
        match sm.handle_key('0' as u32, &mgr) {
            ActionResult::SwitchPage(Page::Search) => {}
            other => panic!("应该进入搜索页，实际: {:?}", other),
        }
    }

    #[test]
    fn test_search_by_id() {
        let (mut sm, mgr) = setup();
        sm.handle_key('0' as u32, &mgr); // 进入搜索
        // 按数字 1 = 十位数为 1
        let result = sm.handle_key('1' as u32, &mgr);
        match result {
            ActionResult::UpdateSearch(_, _) => {}
            _ => panic!("应该更新搜索结果"),
        }
    }

    #[test]
    fn test_search_by_spell() {
        let (mut sm, mgr) = setup();
        sm.handle_key('0' as u32, &mgr); // 进入搜索
        // 按字母 C = 拼音首字母 C
        let result = sm.handle_key('C' as u32, &mgr);
        match result {
            ActionResult::UpdateSearch(spell, _) => {
                assert!(spell.starts_with('C'));
            }
            _ => panic!("应该更新搜索结果"),
        }
    }

    #[test]
    fn test_burst_shortcut() {
        let (mut sm, _mgr) = setup();
        sm.set_space(true);
        match sm.handle_key('3' as u32, &_mgr) {
            ActionResult::SetBurstInterval(3) => {}
            other => panic!("应该设置连发间隔为3，实际: {:?}", other),
        }
    }

    #[test]
    fn test_burst_mode() {
        let (mut sm, mgr) = setup();
        let mut sm = StateMachine::new(0, true, 3, true); // 连发间隔3秒
        match sm.handle_key('1' as u32, &mgr) {
            ActionResult::StartBurst(0, 1) => {}
            other => panic!("应该触发连发，实际: {:?}", other),
        }
    }
