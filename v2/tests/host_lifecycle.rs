//! Actual frozen/patched dispatch/remove/annotate methods. Only process transport
//! and registry ownership inputs are injected; full host is not built.
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tezbar_extension_sdk::CoreResult;
struct Process {
    result: Value,
    sessions: Mutex<HashMap<String, ()>>,
    changed: tokio::sync::Notify,
    generation: u64,
    disposed: AtomicUsize,
}
impl Process {
    async fn call(&self, _: &str, _: Value, _: Duration) -> CoreResult {
        Ok(self.result.clone())
    }
    async fn dispose(&self, _: &str) {
        tokio::task::yield_now().await;
        self.disposed.fetch_add(1, Ordering::SeqCst);
    }
}
struct Entry {
    process: Arc<Process>,
    cancel: tokio::sync::watch::Sender<bool>,
    search: Mutex<tokio::sync::watch::Sender<()>>,
    serial: tokio::sync::Mutex<()>,
    active: AtomicBool,
    policy: Value,
    canonical: String,
    command: String,
}
#[derive(Default)]
struct State {
    sessions: HashMap<String, Arc<Entry>>,
}
struct NativeSessions(Arc<Mutex<State>>);
include!("../fixtures/host-lifecycle-functions.rs");
fn setup(mode: &str) -> (NativeSessions, Arc<Process>) {
    let p = Arc::new(Process {
        result: json!({"ok":true,"mode":mode}),
        sessions: Mutex::new(HashMap::from([("s".into(), ())])),
        changed: tokio::sync::Notify::new(),
        generation: 1,
        disposed: AtomicUsize::new(0),
    });
    let e = Arc::new(Entry {
        process: p.clone(),
        cancel: tokio::sync::watch::channel(false).0,
        search: Mutex::new(tokio::sync::watch::channel(()).0),
        serial: tokio::sync::Mutex::new(()),
        active: AtomicBool::new(true),
        policy: json!({"concurrency":"serialize","refreshMs":null}),
        canonical: "com.tezbar.native.clipboard-sequential-paste".into(),
        command: "paste-next-item".into(),
    });
    (
        NativeSessions(Arc::new(Mutex::new(State {
            sessions: HashMap::from([("s".into(), e)]),
        }))),
        p,
    )
}
#[tokio::test]
async fn original_no_view_action_leaks_reference_but_patch_disposes() {
    let (s, p) = setup("no-view");
    assert_eq!(
        original_dispatch::run(&s, "extension:invoke-action")
            .await
            .unwrap()["mode"],
        "no-view"
    );
    assert_eq!(s.0.lock().unwrap().sessions.len(), 1);
    assert_eq!(p.disposed.load(Ordering::SeqCst), 0);
    let (s, p) = setup("no-view");
    assert_eq!(
        patched_dispatch::run(&s, "extension:invoke-action")
            .await
            .unwrap()["mode"],
        "no-view"
    );
    assert!(s.0.lock().unwrap().sessions.is_empty());
    assert!(p.sessions.lock().unwrap().is_empty());
    assert_eq!(p.disposed.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn patch_retains_real_view_and_unchanged_sessions() {
    for mode in ["view", "unchanged"] {
        let (s, p) = setup(mode);
        let r = patched_dispatch::run(&s, "extension:invoke-action")
            .await
            .unwrap();
        assert_eq!(r["mode"], mode);
        assert_eq!(s.0.lock().unwrap().sessions.len(), 1);
        assert_eq!(p.disposed.load(Ordering::SeqCst), 0);
    }
    let (s, p) = setup("view");
    assert_eq!(
        patched_dispatch::run(&s, "extension:dispose-session")
            .await
            .unwrap(),
        true
    );
    assert!(s.0.lock().unwrap().sessions.is_empty());
    assert_eq!(p.disposed.load(Ordering::SeqCst), 1);
}
