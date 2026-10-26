use super::super::*;
use tezbar_extension_sdk::{ExtensionPaths, TestContext};
fn temp() -> tempfile::TempDir {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".qa/tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}
fn context(dir: &std::path::Path) -> TestContext {
    TestContext::new(
        ExtensionPaths {
            data_dir: dir.into(),
            temp_dir: dir.into(),
            package_dir: dir.into(),
        },
        &[
            "host.clipboard.read",
            "host.clipboard.pasteText",
            "host.window.visibility",
        ],
    )
}
#[tokio::test]
async fn actual_source_oracle_flows() {
    let flows: Value =
        serde_json::from_str(include_str!("../../fixtures/source-traces.json")).unwrap();
    for flow in flows.as_array().unwrap() {
        let name = flow["name"].as_str().unwrap();
        if [
            "failed-paste",
            "clock-rollback-source",
            "host-writes-source-breakage",
        ]
        .contains(&name)
        {
            continue;
        }
        let dir = temp();
        let mut a = Adapter::default();
        let epoch = Instant::now();
        let mut clipboard = String::new();
        for row in flow["trace"].as_array().unwrap() {
            let step = &row["step"];
            if step["restart"] == true {
                a = Adapter::default();
            }
            if let Some(copy) = step["copy"].as_str() {
                clipboard = copy.into();
            }
            let c = context(dir.path());
            if row["calls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v[0] == "read")
            {
                c.reply(Ok(json!(clipboard)));
            }
            if row["calls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v[0] == "paste")
            {
                c.reply(Ok(json!({"ok":true})));
            } else {
                c.reply(Ok(Value::Null));
            }
            let result = c
                .run(a.run(
                    &c.context,
                    &json!({"sessionId":"oracle"}),
                    epoch + Duration::from_millis(step["at"].as_u64().unwrap()),
                    step["at"].as_u64().unwrap(),
                    None,
                ))
                .await;
            let expected = row["calls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c[0] == "paste")
                .map(|c| c[1].clone());
            let actual = c
                .calls()
                .iter()
                .find(|v| v["method"] == "host.clipboard.pasteText")
                .map(|v| v["params"]["text"].clone());
            if expected.as_ref().is_some_and(|v| v.is_object()) {
                assert!(
                    result.is_err(),
                    "safe rejection of source undefined: {name}"
                );
                assert!(actual.is_none());
                continue;
            }
            assert!(result.is_ok(), "{name}: {result:?}");
            assert_eq!(actual, expected, "source paste flow {name}");
            let q = Store::new(dir.path()).unwrap().load().unwrap();
            let source = &row["store"];
            assert_eq!(
                q.active,
                source.get("StorageItem").is_some(),
                "active source checkpoint {name}"
            );
            if q.active {
                assert_eq!(q.original.as_deref(), source["StorageItem"].as_str());
                assert_eq!(q.next as u64, source["StorageIndex"].as_u64().unwrap());
                let source_list: Vec<String> =
                    serde_json::from_str(source["StorageList"].as_str().unwrap()).unwrap();
                assert_eq!(
                    store::split(q.original.as_deref().unwrap()).unwrap(),
                    source_list
                );
            }
            assert_eq!(
                a.gate.lock().await.global_index as u64,
                row["globals"]["index"].as_u64().unwrap()
            );
        }
    }
}
#[tokio::test]
async fn host_written_empty_item_continues_and_restart_is_ambiguous() {
    let dir = temp();
    let c = context(dir.path());
    let a = Adapter::default();
    let now = Instant::now();
    for (n, clipboard, expected) in [
        (0, "a\n\nb\n", "a"),
        (1, "a", ""),
        (2, "", "b"),
        (3, "b", ""),
    ] {
        c.reply(Ok(json!(clipboard)));
        c.reply(Ok(json!({"ok":true})));
        let r = c
            .run(a.run(
                &c.context,
                &json!({"sessionId":"s"}),
                now + Duration::from_millis(n * 650),
                10000 + n * 650,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(r["mode"], "no-view");
        assert_eq!(c.calls().last().unwrap()["params"]["text"], expected);
    }
    let b = Adapter::default();
    c.reply(Ok(json!("")));
    let r = c
        .run(b.run(
            &c.context,
            &json!({"sessionId":"s"}),
            now + Duration::from_secs(9),
            19000,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(r["mode"], "view");
    assert_eq!(r["actions"].as_array().unwrap().len(), 2);
    // Explicit continue trusts the retained original and process-restart index 0.
    let p = json!({"sessionId":"s","actionId":r["actions"][0]["id"]});
    let ticket = b.take_ticket(&p).unwrap();
    assert!(b.take_ticket(&p).is_err());
    c.reply(Ok(json!("")));
    c.reply(Ok(json!({"ok":true})));
    c.run(b.run(
        &c.context,
        &p,
        now + Duration::from_secs(10),
        20000,
        Some(ticket),
    ))
    .await
    .unwrap();
    assert_eq!(c.calls().last().unwrap()["params"]["text"], "a");
}
#[tokio::test]
async fn failed_ack_exact_retry_and_stale_actions() {
    for reply in [
        Ok(json!(null)),
        Ok(json!(true)),
        Ok(json!({"ok":false})),
        Ok(json!({"ok":true,"error":"contradiction"})),
        Err("synthetic failure".into()),
    ] {
        let dir = temp();
        let c = context(dir.path());
        let a = Adapter::default();
        let now = Instant::now();
        c.reply(Ok(json!("α\r\nb")));
        c.reply(reply);
        let r = c
            .run(a.run(&c.context, &json!({"sessionId":"s"}), now, 10000, None))
            .await
            .unwrap();
        assert_eq!(r["mode"], "view");
        assert_eq!(
            Store::new(dir.path())
                .unwrap()
                .load()
                .unwrap()
                .pending
                .unwrap()
                .text,
            "α"
        );
        let p = json!({"sessionId":"s","actionId":r["actions"][0]["id"]});
        let ticket = a.take_ticket(&p).unwrap();
        assert!(a.take_ticket(&p).is_err());
        c.reply(Ok(json!({"ok":true})));
        c.run(a.run(
            &c.context,
            &p,
            now + Duration::from_secs(1),
            11000,
            Some(ticket),
        ))
        .await
        .unwrap();
        let methods: Vec<_> = c
            .calls()
            .iter()
            .map(|v| v["method"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            methods,
            [
                "host.clipboard.read",
                "host.clipboard.pasteText",
                "host.clipboard.pasteText"
            ]
        );
        assert_eq!(c.calls().last().unwrap()["params"]["text"], "α");
    }
}
#[tokio::test]
async fn queued_arrival_and_rollback_do_not_repeat() {
    let dir = temp();
    let c = context(dir.path());
    let a = Adapter::default();
    let now = Instant::now();
    c.reply(Ok(json!("a\nb\nc")));
    c.reply(Ok(json!({"ok":true})));
    c.run(a.run(&c.context, &json!({"sessionId":"s"}), now, 10000, None))
        .await
        .unwrap();
    // Delayed execution, original arrival still within 650ms; no read or paste.
    c.run(a.run(
        &c.context,
        &json!({"sessionId":"q"}),
        now + Duration::from_millis(649),
        20000,
        None,
    ))
    .await
    .unwrap();
    assert_eq!(c.calls().len(), 3);
    c.reply(Ok(json!("a")));
    c.reply(Ok(json!({"ok":true})));
    c.run(a.run(
        &c.context,
        &json!({"sessionId":"r"}),
        now + Duration::from_millis(650),
        9000,
        None,
    ))
    .await
    .unwrap();
    assert_eq!(c.calls().last().unwrap()["params"]["text"], "b");
}
#[tokio::test]
async fn read_failure_and_profile_fencing() {
    let dir = temp();
    let c = context(dir.path());
    let a = Adapter::default();
    c.reply(Err("synthetic read failure".into()));
    assert!(c
        .run(a.run(
            &c.context,
            &json!({"sessionId":"s"}),
            Instant::now(),
            10000,
            None
        ))
        .await
        .is_err());
    assert_eq!(c.calls().len(), 1);
    let other = temp();
    let d = context(other.path());
    assert!(d
        .run(a.run(
            &d.context,
            &json!({"sessionId":"s"}),
            Instant::now(),
            10000,
            None
        ))
        .await
        .is_err());
    assert!(d.calls().is_empty());
}
#[test]
fn persistence_bounds_corruption_and_write_failure() {
    let dir = temp();
    let store = Store::new(dir.path()).unwrap();
    let mut q = Queue::fresh();
    store.save(&mut q).unwrap();
    let path = dir.path().join("clipboard-sequential-paste-v2/queue.json");
    std::fs::write(&path, b"{broken").unwrap();
    assert!(store.load().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    std::fs::write(&path, vec![b' '; store::MAX_STORE as usize + 1]).unwrap();
    assert!(store.load().is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(store.save(&mut q).is_err());
    #[cfg(unix)]
    {
        std::fs::remove_dir(&path).unwrap();
        std::os::unix::fs::symlink(dir.path().join("elsewhere"), &path).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&mut q).is_err());
    }
}

#[tokio::test]
async fn sdk_owned_guard_cleans_unpolled_recovery_registrations() {
    let dir = temp();
    let a = Adapter::default();
    for _ in 0..140 {
        let c = context(dir.path());
        a.recover(
            &c.context,
            &json!({"sessionId":"never-disposed-open"}),
            &Queue::fresh(),
            true,
        )
        .unwrap();
        // No await between registration and dropping its SDK scope. The inert
        // future may not have been polled, yet must still own/drop its guard.
        drop(c);
        tokio::task::yield_now().await;
        assert!(a.tickets.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn restart_identical_item_can_be_explicit_new_clipboard() {
    let dir = temp();
    let c = context(dir.path());
    let a = Adapter::default();
    let epoch = Instant::now();
    c.reply(Ok(json!("a\nb")));
    c.reply(Ok(json!({"ok":true})));
    c.run(a.run(&c.context, &json!({"sessionId":"s"}), epoch, 10000, None))
        .await
        .unwrap();
    let b = Adapter::default();
    c.reply(Ok(json!("a")));
    let r = c
        .run(b.run(
            &c.context,
            &json!({"sessionId":"r"}),
            epoch + Duration::from_secs(1),
            11000,
            None,
        ))
        .await
        .unwrap();
    let p = json!({"sessionId":"r","actionId":r["actions"][1]["id"]});
    let ticket = b.take_ticket(&p).unwrap();
    c.reply(Ok(json!("a")));
    c.reply(Ok(json!({"ok":true})));
    c.run(b.run(
        &c.context,
        &p,
        epoch + Duration::from_secs(2),
        12000,
        Some(ticket),
    ))
    .await
    .unwrap();
    let q = Store::new(dir.path()).unwrap().load().unwrap();
    assert_eq!(q.original.as_deref(), Some("a"));
    assert!(!q.active);
}

#[tokio::test]
async fn retained_invalid_input_and_new_clipboard_after_restart() {
    let dir = temp();
    let c = context(dir.path());
    let a = Adapter::default();
    let epoch = Instant::now();
    c.reply(Ok(json!("a\nb\nc")));
    c.reply(Ok(json!({"ok":true})));
    c.run(a.run(&c.context, &json!({"sessionId":"s"}), epoch, 10000, None))
        .await
        .unwrap();
    c.reply(Ok(json!("x".repeat(store::MAX_LINE + 1))));
    assert!(c
        .run(a.run(
            &c.context,
            &json!({"sessionId":"bad"}),
            epoch + Duration::from_secs(1),
            11000,
            None
        ))
        .await
        .is_err());
    assert_eq!(
        Store::new(dir.path())
            .unwrap()
            .load()
            .unwrap()
            .original
            .as_deref(),
        Some("a\nb\nc")
    );
    let b = Adapter::default();
    c.reply(Ok(json!("new\nlist")));
    c.reply(Ok(json!({"ok":true})));
    c.run(b.run(
        &c.context,
        &json!({"sessionId":"new"}),
        epoch + Duration::from_secs(2),
        9000,
        None,
    ))
    .await
    .unwrap();
    assert_eq!(c.calls().last().unwrap()["params"]["text"], "new");
}
