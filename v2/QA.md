# Executed QA and limits

Development evidence on this macOS ARM64 machine, 2026-10-26. No real clipboard,
selection, browser, settings, Downloads, profile secrets or live application APIs
were exercised. All clipboard values are synthetic. Required inputs are visible
under `v2`; generated state, dependency caches, logs, builds and archives are ignored.

## Reproduction

From repository root: `sh v2/scripts/qa.sh`. The script uses child-process HOME and
TMPDIR under `.qa`, an isolated Cargo home, and an exact-lock disposable V1 copy.
It uses existing Rust stable/1.85 toolchains and does not install toolchains. An
initial plain npm ci failed on the source lock's optional peer conflict; the exact
unchanged lock succeeds with `--legacy-peer-deps --ignore-scripts`. No source
package/lockfile was modified. Any npm audit summary is not a security clearance;
no automatic audit fixes or dependency updates were applied.

| Check | Result and actual coverage |
| --- | --- |
| Fresh clone/fetch/branch | `origin/main` = dd58c5e7b4c1213abd645ed69f30c0479b6f56f0; task branch `feat/native-v2-rust` |
| V1 tests/typecheck | 1 source smoke test passed, exact-lock disposable-copy typecheck passed |
| Source oracle | 8 deterministic actual-TS flows, 31 steps; `--check` reproduces frozen traces |
| Production Rust tests | 11 unit/domain tests + 5 actual host-function fixture tests pass on stable and 1.85.0 |
| Formatting/lints | rustfmt check and Clippy `--all-targets -- -D warnings` pass on stable and 1.85.0 |
| SDK HTTPS independence | Fresh isolated Cargo home fetched SDK e955911f… over HTTPS; lock/crate version 0.1.1 verified; no sibling dependency |
| Manifest | Actual pinned draft2020 host schema valid via locked Ajv 8.17.1; 14 negative mutations rejected |
| Renderer recovery | Original MouseEvent bug reproduced; patched primary button, keyboard Enter, second menu button IDs and no-view onBack lifecycle pass on exact renderer functions + locked React/TS |
| Host prerequisites | Four patches dry-run/apply only to ignored frozen fixture copies; exact extracted function bodies compile and execute |
| Real process pipes | Stable and 1.85 production executable: 21 scenario checks plus 140 no-view dispose/reopens per run; synthetic reverse RPC only |
| Native dev archive | ARM64 Mach-O release binary, ignored ZIP, extraction, exact complete inventory and unchanged icon; extracted production executable passes same domain pipes suite |
| Git audit | HEAD unchanged, index empty; every V1 tracked byte unchanged except exact short README link; whole Git-visible SHA256 index excludes itself |
| Windows MSVC | Attempted `cargo check --target x86_64-pc-windows-msvc`; failed E0463 because required std/core target is absent; no installation attempted |
| Real desktop/OS | macOS GUI clipboard/focus/paste and Windows build/execution/clipboard/focus/paste: **not-run** |

`--self-test` / `--protocol-info` are metadata entrypoints, not evidence of domain
behavior. The process suite actually opens the production command, checks durable
pending state before replying to paste RPC, and drives SDK JSONL pipes.

## Scenario coverage

The source oracle includes LF/CRLF/empty/trailing/lone-CR, 649/650 ms boundary,
new-list global-index carry, completion's retained-global quirk, restart, empty
clipboard, paste rejection, wall rollback and the source failure under a
clipboard-writing host. Five compatible flow traces are compared against the real
Rust adapter's effects, persisted active list/index and process global index.
Out-of-range `undefined`, rollback, unknown paste outcomes and host-overwrite are
explicit documented adaptations, not silently asserted identical.

Domain/pipe tests exercise ordinary automatic paste, empty item `text:""`, retained
original continuation after host overwrite, active/restarted/completed queues,
explicit continue/new choices for restart identical-item ambiguity, unrelated new
clipboard after restart, input/line/store bounds without truncation, read failure,
corrupt storage retained unchanged, symlink rejection, process profile fencing,
source short-new-list global-index rejection, exact-byte retry without reread,
strict malformed/failed ACKs and one-use/stale choices.

Frozen original/patched host dispatch methods also reproduce and fix retained
no-view action references, await disposal despite cancellation fencing, and retain
view/unchanged sessions. The actual parent renderer action closure returns via
onBack after no-view completion. The full host process remains unbuilt.

Concurrent opens are sent while the first clipboard read is blocked for >650 ms;
the queued <650 ms arrival stays suppressed instead of becoming a later paste.
Monotonic arrival and backwards wall time are tested with controlled clocks at
the real adapter. The production pipe suite checks cancellation **after dispatch**,
late ACK rejection, real 10-second reverse timeout, EOF during effect, restart with
pending effect, explicit mark handled, failed checkpoint write before dispatch,
and failed completion persistence after host ACK. Pending state is checked before
host effect ACK. It is never automatically repeated or advanced.

140 no-view open/dispose cycles in one actual SDK process test slot lifecycle.
A separate 140-cycle unit test constructs recovery registration, drops its SDK
scope before first poll with no explicit dispose callback, and checks authoritative
registration cleanup. Consumed guard futures complete through a one-shot signal;
there are no detached domain tasks, polling timers or background leases.

## Interpretation limits

The full app is neither checked out nor compiled by these tests. Host registry
collection/package resolution/platform inputs are injected around exact extracted
functions. Renderer hook state/lifecycle is injected and real element callbacks
are invoked directly: no browser DOM or real desktop rendering is claimed.
The real typed router and clipboard worker text argument helpers are extracted
and executed for empty/exact strings. The frozen source contracts show the host's empty-text call path but cannot prove
native arboard behavior, focus restoration, destination insertion or permissions.
Windows std/core/tooling is missing; a GNU target already present on the machine
does not satisfy the required MSVC build. All real OS verification remains not-run.

A successful paste ACK plus durable checkpoint is not exactly-once delivery.
Clipboard changes can race read/write/paste. Same-byte copying during one process
is not observable through the typed API; after restart it gets explicit recovery.
Filesystem path checks trust host-private directory ownership and do not claim
protection against hostile same-user TOCTOU races or guaranteed fsync latency.
Windows atomic replacement durability/ACL behavior is not verified here.

Authentic own/upstream/SDK/host and crate/tooling notices are retained. The own
LICENSE's original literal backslash-newline formatting is preserved. Missing
standalone license evidence is explicit for r-efi and some npm tooling/target
packages in license inventories; no grants were substituted or invented. No
icon-specific license beyond upstream's root MIT grant was found.

## XERJ coverage

Initial root-scoped index/search used canonical root SHA256 prefix
`ax-248e0eda4ed3` and `.git/xerj-state`, with no global schema/server changes.
Initial indexing completed with all four source code files indexed; two image
files were skipped as junk. Final coverage is recorded in `SEARCH.md`; exact
byte, Git and full-file checks cover evidence that lexical indexing cannot prove.
