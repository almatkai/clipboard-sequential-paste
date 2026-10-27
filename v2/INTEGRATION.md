# Parent integration review

This clone remains intact on `feat/native-v2-rust`, based on freshly fetched own
`origin/main` dd58c5e7b4c1213abd645ed69f30c0479b6f56f0. HEAD is unchanged and the
Git index is empty. V1 is byte-identical except the root README discovery link.
All implementation, docs, tests, fixtures and licenses are under `v2/`. No app,
SDK, profile, catalog, sibling repository, ledger, root package/lock/workflow,
release, installation, commit or push is part of this task.

Frozen inputs: SDK e955911f0bf5b558f9d3eb5183df0adc95c22af2, host
ea100cee3f69711de12485ba061950c5fc65258f, upstream
6734d85992290066fd31a7f5bee3cca3d3545a1a. The host and SDK fixtures were read
only with exact `git show <pin>:<path>` (tree inventory used Git object metadata).
The host working checkout was neither read for implementation nor built/modified.
The isolated Cargo home obtained the SDK over HTTPS; Cargo.lock contains the exact
Git source, version 0.1.1 and revision, with no path or app/Tauri dependency.

## Domain transition contract

| Phase | Durable state before effect | On success | On error/drop/unknown |
| --- | --- | --- | --- |
| Admission | Last wall timestamp; prior original/queue retained | Read clipboard | Read/validation error view, no paste |
| Ownership ambiguity | Original + produced + prior process owner retained | Explicit continue/new decision only | No automatic paste |
| Prepare | Original, selected index, exact pending text, revision | Dispatch one pasteText | Pending may conservatively mean not dispatched |
| Paste RPC | Same pending checkpoint | Accept strict ACK; atomic advance checkpoint | Pending; retry/advance view on reopen |
| Advance checkpoint | Completion/next + produced + current owner | Return no-view | Old pending or new completion may be present; reopen |
| Explicit retry | New revision, identical pending bytes | One pasteText, no read | Remains pending; no automatic replay |
| Explicit mark handled | Atomic next/completed and produced ownership | Hide window, no paste | Inspect persisted checkpoint on reopen |

No exactly-once claim: a host paste cannot be rolled back and an unknown delivery
cannot be inferred from clipboard equality. Same-byte user copies and interleaved
clipboard changes have the limits documented in README. There is no clipboard
restoration. Failed/cancelled staged SDK state cannot erase pending storage.

## Pinned host contracts

- `process_host.rs` routes clipboard read to `clipboard:read`, which returns a JSON
  string, and pasteText `{text}` to `core:platform:paste-text`.
- `platform.rs` writes through `clipboard:write`, hides the window/app, waits 80 ms,
  emits Cmd+V/Ctrl+V, and returns `{ok:true,message:"Pasted text"}`. It has no
  nonempty guard. `clipboard.rs` passes empty strings to arboard `set_text`.
  Real native clipboard behavior is not inferred from this source inspection.
- `host.window.visibility {visible:false}` maps to platform visibility and returns
  null. It is used only where source calls closeMainWindow without a paste and
  after explicit mark-handled. A paste ACK still rejects null.
- Host native process launch disposes no-view sessions. SDK failed/cancelled opens
  remove their registration and stop session scope. Port recovery guard also owns
  registrations if the future never got polled; tests exercise this explicitly.
- SDK state is staged and committed only on successful/current operation. Port
  queue and effect ownership are deliberately outside that state. One-use actions
  are removed synchronously before waiting for the process mutex.
- Frozen renderer `DetailRuntime` can show the recovery markdown and action menu;
  action kind is `action`. Full schema + negatives and executed renderer callback
  tests prevent invented Forms/List/button contracts. Only Detail is used.

## Unapplied prerequisites

Four separate generic patches live in `host-patches/`. They are not applied to
any application checkout. Exact frozen fixture tests reproduce the primary click
bug and resource lookup gap, execute patched functions, and test verified exact
own/upstream replacement routing, uncovered commands, damaged packages, platform
mismatch, lookalikes and ambiguous owners. Surrounding host package resolution is
injected in fixture tests; the complete application is not compiled or tested here.
The fourth patch closes completed no-view action sessions and returns from the
recovery surface; the pinned host otherwise only disposes no-view opens. Exact
dispatch/remove/annotate functions and the actual renderer action closure are
executed with injected process transport/state. No catalog publication or release
pin is included. Parent should review and validate
these narrow prerequisites against its chosen host branch before app-level acceptance.

## Packaging and platform status

`package-dev.py` makes an ignored ARM64 dev ZIP from the release binary, original
PNG, authentic license/notice files and a complete file inventory. It extracts and
rehashes every member and runs the full production domain JSONL suite on the
extracted executable. This is development QA, never installation or publication.
Source artifacts remain `{}`; there are no target URLs or trusted catalog entries.
No ZIP, executable, target, node_modules or private QA output is Git-visible.

Windows MSVC check was attempted and failed because `x86_64-pc-windows-msvc`
std/core is absent. No tools were installed. A GNU Windows target happens to exist
but does not replace the required MSVC target. No Windows binary/execution claim.
Real macOS/Windows clipboard/focus/paste, desktop renderer and clean-profile host
acceptance are all not-run. Self-test/protocol-info are metadata only and are not
domain QA. See QA for executed production tests and remaining limits.
