//! Compile exact functions extracted from the frozen/patched host, with only the
//! surrounding registry and platform inputs injected. No app/Tauri build.
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
#[derive(Clone)]
pub struct Installed {
    id: String,
    slug: String,
    path: PathBuf,
    manifest: Value,
    package_dir: Option<PathBuf>,
}
pub struct CoreContext {
    rows: Vec<Installed>,
}
fn collect(ctx: &CoreContext) -> Vec<Installed> {
    ctx.rows.clone()
}
fn platform() -> &'static str {
    "macOS"
}
include!("../fixtures/host-functions.rs");
fn item(dir: &Path) -> Installed {
    Installed {
        id: "com.tezbar.native.clipboard-sequential-paste".into(),
        slug: "clipboard-sequential-paste".into(),
        path: dir.into(),
        package_dir: None,
        manifest: serde_json::from_str(include_str!("../extension.json")).unwrap(),
    }
}
#[test]
fn exact_verified_aliases_and_coverage() {
    let mut row = item(Path::new("/synthetic"));
    let mut ctx = CoreContext {
        rows: vec![row.clone()],
    };
    for id in [
        "com.tezbar.clipboard-sequential-paste",
        "raycast.clipboard-sequential-paste",
    ] {
        assert_eq!(
            verified_replacement(&ctx, id, "paste-next-item"),
            Some(row.id.clone())
        );
    }
    for id in [
        "com.tezbar.native.clipboard-sequential-paste",
        "com.tezbar.clipboard-sequential-paste-evil",
        "raycast.other",
        "clipboard-sequential-paste",
    ] {
        assert!(verified_replacement(&ctx, id, "paste-next-item").is_none());
    }
    assert!(verified_replacement(&ctx, "raycast.clipboard-sequential-paste", "unknown").is_none());
    ctx.rows.push(row.clone());
    assert!(verified_replacement(
        &ctx,
        "raycast.clipboard-sequential-paste",
        "paste-next-item"
    )
    .is_none());
    ctx.rows.pop();
    row.manifest["healthError"] = json!("tampered");
    ctx.rows = vec![row.clone()];
    assert!(verified_replacement(
        &ctx,
        "raycast.clipboard-sequential-paste",
        "paste-next-item"
    )
    .is_none());
    row.manifest["healthError"] = Value::Null;
    row.manifest["commands"][0]["runtime"] = json!("javascript");
    ctx.rows = vec![row.clone()];
    assert!(verified_replacement(
        &ctx,
        "raycast.clipboard-sequential-paste",
        "paste-next-item"
    )
    .is_none());
    row.manifest["commands"][0]["runtime"] = json!("rust");
    row.manifest["commands"][0]["platforms"] = json!(["Windows"]);
    ctx.rows = vec![row];
    assert!(verified_replacement(
        &ctx,
        "raycast.clipboard-sequential-paste",
        "paste-next-item"
    )
    .is_none());
}
#[test]
fn active_version_resources_are_selected() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".qa/tmp");
    std::fs::create_dir_all(&base).unwrap();
    let d = tempfile::tempdir_in(base).unwrap();
    let mut row = item(
        &d.path()
            .join("com.tezbar.native.clipboard-sequential-paste"),
    );
    let active = row.path.join("versions/0.2.0-sha");
    std::fs::create_dir_all(active.join("resources")).unwrap();
    let icon = active.join("resources/extension-icon.png");
    std::fs::write(&icon, include_bytes!("../resources/extension-icon.png")).unwrap();
    row.package_dir = Some(active);
    assert!(original_icon_path(&row).is_none());
    assert_eq!(patched_icon_path(&row), Some(icon.to_string_lossy().into()));
    row.manifest["icon"] = json!("../secret.png");
    assert!(patched_icon_path(&row).is_none());
}

#[test]
fn pinned_host_text_helpers_preserve_empty_and_exact_bytes() {
    // Both real typed router and clipboard worker argument helpers accept empty
    // strings and preserve bytes. Native arboard/focus/key delivery is untested.
    for text in ["", "a\rb", "α", "\r", "a\n"] {
        let typed = json!({"text": text});
        assert_eq!(typed_string(&typed, "text").unwrap(), text);
        let args = vec![json!(text)];
        assert_eq!(text_arg(&args, 0).unwrap(), text);
    }
    assert!(typed_string(&json!({"text": null}), "text").is_err());
    assert!(text_arg(&[json!(null)], 0).is_err());
    let platform = include_str!("../fixtures/host/src-tauri/src/native_core/platform.rs");
    let paste = platform
        .split("\"core:platform:paste\" | \"core:platform:paste-text\" => {")
        .nth(1)
        .unwrap()
        .split("\"core:platform:clipboard-write\" =>")
        .next()
        .unwrap();
    assert!(
        paste.find("clipboard:write").unwrap() < paste.find("set_backend_app_visibility").unwrap()
    );
    assert!(paste.find("set_backend_app_visibility").unwrap() < paste.find("press_key").unwrap());
    assert!(paste.contains("Ok(json!({\"ok\":true,\"message\":\"Pasted text\"}))"));
    assert!(!paste.contains("is_empty"));
}
