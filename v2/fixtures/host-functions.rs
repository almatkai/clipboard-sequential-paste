// Generated exact function extraction by scripts/host-fixtures.py.

fn fallback<'a>(value: &'a Value, key: &str, default: &'a str) -> &'a str {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
}

fn compatible(manifest: &Value, command: bool) -> bool {
    if command && manifest.get("platforms").is_none() {
        return true;
    }
    let Some(platforms) = manifest["platforms"]
        .as_array()
        .or_else(|| manifest["tezbar"]["platforms"].as_array())
    else {
        return platform() == "macOS";
    };
    platforms
        .iter()
        .filter_map(Value::as_str)
        .any(|value| match value.to_lowercase().as_str() {
            "macos" | "darwin" | "mac" => platform() == "macOS",
            "windows" | "win32" | "win" => platform() == "Windows",
            "linux" => platform() == "Linux",
            _ => false,
        })
}

pub fn original_icon_path(extension: &Installed) -> Option<String> {
    let name = fallback(&extension.manifest, "icon", "icon.png");
    if name.starts_with("https://") || name.starts_with("http://") {
        return Some(name.into());
    }
    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut candidates = vec![
        extension.path.join(relative),
        extension.path.join("assets").join(relative),
    ];
    if extension.id.starts_with("com.tezbar.native.") {
        if let Some(packages) = extension.path.parent() {
            if let Some(extensions) = packages.parent() {
                let legacy = extensions.join(&extension.slug);
                candidates.push(legacy.join(relative));
                candidates.push(legacy.join("assets").join(relative));
            }
            let legacy = packages.join(format!("raycast.{}", extension.slug));
            candidates.push(legacy.join(relative));
            candidates.push(legacy.join("assets").join(relative));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into())
}

pub fn patched_icon_path(extension: &Installed) -> Option<String> {
    let name = fallback(&extension.manifest, "icon", "icon.png");
    if name.starts_with("https://") || name.starts_with("http://") {
        return Some(name.into());
    }
    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let mut candidates = vec![
        extension.path.join(relative),
        extension.path.join("assets").join(relative),
    ];
    if let Some(package_dir) = &extension.package_dir {
        candidates.insert(0, package_dir.join(relative));
    }
    if extension.id.starts_with("com.tezbar.native.") {
        if let Some(packages) = extension.path.parent() {
            if let Some(extensions) = packages.parent() {
                let legacy = extensions.join(&extension.slug);
                candidates.push(legacy.join(relative));
                candidates.push(legacy.join("assets").join(relative));
            }
            let legacy = packages.join(format!("raycast.{}", extension.slug));
            candidates.push(legacy.join(relative));
            candidates.push(legacy.join("assets").join(relative));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into())
}

pub fn verified_replacement(ctx: &CoreContext, id: &str, command: &str) -> Option<String> {
    if id.starts_with("com.tezbar.native.") { return None; }
    let mut matches = collect(ctx).into_iter().filter(|e| {
        e.id.starts_with("com.tezbar.native.") && e.manifest["healthError"].is_null()
            && e.manifest["replaces"].as_array().is_some_and(|ids| ids.iter().any(|v| v == id))
            && compatible(&e.manifest, false)
            && e.manifest["commands"].as_array().is_some_and(|commands| commands.iter().any(|c| {
                c["name"] == command && c["runtime"] == "rust" && compatible(c, true)
            }))
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first.id)
}

pub fn typed_string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| s.len() <= 1024 * 1024)
        .ok_or_else(|| format!("Missing/invalid {key}"))
}

pub fn text_arg(args: &[Value], n: usize) -> Result<&str, String> {
    args.get(n)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Argument {} must be a string", n + 1))
}
