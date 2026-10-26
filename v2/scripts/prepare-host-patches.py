#!/usr/bin/env python3
"""Generate narrowly scoped, UNAPPLIED patches only from immutable Git fixtures."""
import difflib,pathlib
root=pathlib.Path(__file__).resolve().parents[1]
base=root/'fixtures/host'
out=root/'host-patches';out.mkdir(exist_ok=True)
def patch(name, edits):
 parts=[]
 for file,transform in edits:
  old=(base/file).read_text();new=transform(old);assert old!=new
  parts.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),'a/'+file,'b/'+file))
 (out/name).write_text(''.join(parts))
patch('01-detail-primary-click.patch', [('src/renderer/src/raycast-api/detail-runtime.tsx',lambda s:s.replace('onClick={onRunPrimaryAction}','onClick={() => onRunPrimaryAction()}'))])
def icons(s):
 s=s.replace('    manifest: Value,\n    data_dir:', '    manifest: Value,\n    package_dir: Option<PathBuf>,\n    data_dir:')
 s=s.replace('            let mut manifest = if native {','            let mut package_dir = None;\n            let mut manifest = if native {')
 s=s.replace('                    Ok(package) => package.manifest,','                    Ok(package) => {\n                        package_dir = Some(package.dir);\n                        package.manifest\n                    },')
 s=s.replace('                manifest,\n                data_dir:', '                manifest,\n                package_dir,\n                data_dir:')
 s=s.replace('        manifest: Value::Null,','        manifest: Value::Null,\n        package_dir: None,')
 s=s.replace('    if extension.id.starts_with("com.tezbar.native.") {\n        if let Some(packages)', '    if let Some(package_dir) = &extension.package_dir {\n        candidates.insert(0, package_dir.join(relative));\n    }\n    if extension.id.starts_with("com.tezbar.native.") {\n        if let Some(packages)',1)
 return s
patch('02-active-version-icon.patch',[('src-tauri/src/native_core/extensions/registry.rs',icons)])
helper='''// Exact aliases from verified installed manifests only. Ambiguous owners and
// uncovered commands keep legacy routing. No inferred com.tezbar.* prefix strip.
pub(super) fn verified_replacement(ctx: &CoreContext, id: &str, command: &str) -> Option<String> {
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

'''
def registry(s):
 s=s.replace('/// Native implementations use the same discovery and platform rules as the launcher.',helper+'/// Native implementations use the same discovery and platform rules as the launcher.')
 s=s.replace('    let native = if id.starts_with("raycast.") {','    let native = if let Some(canonical) = verified_replacement(ctx, id, command) {\n        installed(ctx, &canonical).ok()\n    } else if id.starts_with("raycast.") {')
 return s
def native(s):
 s=s.replace('if requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()','if requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()\n                || registry::verified_replacement(ctx, requested, body["commandName"].as_str().unwrap_or("")).is_some()')
 old='''        } else {
            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)
        };'''
 new='''        } else if let Some(canonical) = registry::verified_replacement(ctx, requested, command) {
            canonical
        } else {
            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)
        };'''
 assert old in s
 return s.replace(old,new)
patch('03-verified-exact-alias-routing.patch',[('src-tauri/src/native_core/extensions/registry.rs',registry),('src-tauri/src/native_core/extensions/native.rs',native)])
def lifecycle_native(s):
 old='''        tokio::select! {biased;_=cancel.changed()=>Err("Extension session disposed".into()),_=async{if let Some(s)=&mut search{let _=s.changed().await;}else{std::future::pending::<()>().await;}}=>Err("Search superseded by a newer query".into()),result=async{'''
 assert old in s
 s=s.replace(old,old.replace('        tokio::select!', '        let result = tokio::select!'),1)
 old='''        }=>result}
    }
}
fn deadline'''
 new='''        }=>result};
        let value = result?;
        if value["mode"] == "no-view" {
            // Finish outside the cancellation select and serial lock: remove
            // signals entry.cancel and disposal must still be awaited.
            self.remove(id);
            entry.process.dispose(id).await;
        }
        Ok(value)
    }
}
fn deadline'''
 assert old in s
 return s.replace(old,new,1)
def lifecycle_renderer(s):
 old='''              if (result.mode === 'no-view') {
                setState((prev) => ({ ...prev, message: undefined }))
                return
              }'''
 new='''              if (result.mode === 'no-view') {
                onBack()
                return
              }'''
 assert old in s
 return s.replace(old,new,1)
patch('04-no-view-action-lifecycle.patch',[('src-tauri/src/native_core/extensions/native.rs',lifecycle_native),('src/renderer/ExtensionRuntimeView.tsx',lifecycle_renderer)])
