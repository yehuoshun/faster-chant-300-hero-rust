    use super::*;

    fn test_dir() -> PathBuf {
        PathBuf::from("fcd-test-schemes")
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
        let dir = PathBuf::from("fcd-test-persist");
        let _ = std::fs::remove_dir_all(&dir);

        {
            let mut mgr = SchemeManager::init(dir.clone());
            let s = mgr.append(0);
            s.name = "持久化测试".into();
            s.primary[0] = "数据".into();
            mgr.set_active(0);
        }

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
