# UNAPPLIED host prerequisites

All patches target host ea100cee3f69711de12485ba061950c5fc65258f. No host checkout
was modified, stashed, checked out or built. These are generic, narrow fixes needed
by this port; they contain no extension-specific catalog entry or artifact pins.

1. `01-detail-primary-click.patch`: wraps only Detail's Enter button callback so
   MouseEvent cannot become an action ID. No Forms or List patch is included.
2. `02-active-version-icon.patch`: retains the already verified `Package.dir` in
   discovery and checks its resource path before package-root/legacy fallbacks.
   Installed package path/preferences ownership stays unchanged. Only the used
   extension icon path is patched; command icon behavior is not expanded.
3. `03-verified-exact-alias-routing.patch`: derives exact replacement ownership
   from verified installed manifests for the requested covered Rust command.
   Both own and upstream IDs work without guessing a com.tezbar prefix. Duplicate
   owners, damaged/unsupported packages and uncovered commands keep legacy routing.
   Canonical failures retain existing repair semantics.


4. `04-no-view-action-lifecycle.patch`: after a successful action-call returns
   no-view, removes the host reference and awaits SDK dispose outside its serial
   lock/cancellation select. The renderer returns from the recovery surface through
   onBack. The original host only disposed no-view **opens**, leaving completed
   recovery action sessions retained. Real dispatch/remove/annotate functions are
   extracted and executed with injected process transport; original retention is
   reproduced, patched cleanup is awaited, and view/unchanged sessions stay alive.
   The actual renderer onInvokeAction closure is also executed: original leaves
   stale state, patched completion calls onBack exactly once.

`prepare-host-patches.py` regenerates diffs from immutable Git fixtures.
`host-fixtures.py --check` dry-runs/applies each patch only to a fresh ignored fixture
copy, extracts exact real functions and checks committed fixture bodies.
`tests/host_contract.rs` compiles and executes extracted icon/alias functions with
injected registry/platform inputs. `renderer-qa.cjs` executes frozen original and
patched real Detail/surface/action-overlay functions with locked TypeScript 5.9.3
and React 19.0.0. Original mouse failure is reproduced; patched button, keyboard
Enter and secondary recovery-action button pass. Hook lifecycle/state is injected;
these are React element callback tests, not DOM/browser/desktop GUI tests.

The complete host and actual package verifier are not built in this task. Parent
review and full host integration tests remain required before using the patches.
