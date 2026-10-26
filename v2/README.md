# Clipboard Sequential Paste — standalone Rust V2

Feature-branch implementation for parent review. Nothing is installed or published.
The only command is `paste-next-item` (`no-view`): an ordinary launch automatically
pastes the next item. There is no delivery screen, queue editor, history, polling,
background lease, app dependency, local SDK path, or JavaScript product runtime.
Error and ambiguity recovery alone uses a static native `Detail` view.

Build and reproduce the isolated checks from the repository root:

```sh
sh v2/scripts/qa.sh
```

Requires existing stable and Rust 1.85.0 toolchains with rustfmt/Clippy, Node 26,
Python 3.9+, `patch`, Git and HTTPS access to the pinned SDK/npm/crates registries.
The script installs only test/build dependencies into ignored `v2/.qa`; it does
not install the extension or alter an application/profile/toolchain. Outputs are
ignored: `v2/target`, `.qa` (including dev ZIP and extracted package), and `dist`.
`artifacts: {}` is development metadata, not a release reference.

## Exact provenance

| Input | Immutable pin |
| --- | --- |
| Own repository, fresh `origin/main` | `dd58c5e7b4c1213abd645ed69f30c0479b6f56f0` |
| Raycast extensions origin declared by own manifest | `6734d85992290066fd31a7f5bee3cca3d3545a1a`, `extensions/clipboard-sequential-paste` |
| Independent `tezbar-extension-sdk` 0.1.1 HTTPS Git dependency | `e955911f0bf5b558f9d3eb5183df0adc95c22af2` |
| Read-only Tezbar host contract | `ea100cee3f69711de12485ba061950c5fc65258f` |
| Original upstream icon | 24,883 bytes, SHA256 `fcffd51e4ec206a3684a2f6bcabc7b4082964942a14fa97983b1f9c293360368` |

Icon bytes came from the [immutable upstream URL](https://raw.githubusercontent.com/raycast/extensions/6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/clipboard-sequential-paste/assets/extension-icon.png).
They are not recompressed and are deliberately different from the current own
repository icon. Frozen own source/lock, selected SDK and host Git objects,
upstream command and actual locked API declarations are in `fixtures/`.
`FIXTURES.json` hashes them; `INDEX.sha256` hashes every Git-visible file in this
repository except the index itself. No required fixtures are ignored.

Canonical ID: `com.tezbar.native.clipboard-sequential-paste`; exact replacements:
`com.tezbar.clipboard-sequential-paste` and `raycast.clipboard-sequential-paste`.
The pinned host's recommended entry is the latter, owned by `almatkai` and pointing
to this own repository. Manifest schema 3, Rust/process execution, protocol 1,
MSRV 1.85, SDK 0.1.1, one serialized command and `refreshMs: null` are fixed.
There are no fabricated release URLs, artifacts, hashes or catalog entries.

## Behavior verified against source

`fixtures/source-traces.json` contains 8 flows/31 invocations of the actual own TS
command, compiled by its exact locked TypeScript 5.9.3 and executed in a VM with
injected storage, clock and clipboard producer. Only mocked `@raycast/api` imports
are accepted; process/fs/OS/network/browser/real clipboard are unavailable in the
VM. Rust tests compare real adapter paste bytes, active source checkpoint, list,
next index and in-process global index to those executable traces. The oracle
includes failures and deliberately different host-overwrite/clock behavior.
`@raycast/api` is locked at 1.104.25; `@raycast/utils` 2.3.0 is not imported by the
command and supplies no queue algorithm. The API native implementation is not
available as a normal Node module; these tests do not claim Raycast GUI behavior.

- Split is exactly `/\r?\n/`: LF and CRLF delimit, empty interior/final items stay,
  and a lone CR remains data. Empty clipboard is a no-op, but an empty queued item
  is sent as `pasteText {text:""}`. Host's real empty-text OS behavior is untested.
- 650 ms debounce is inclusive at 650 (649 suppressed, 650 admitted). It records
  admission before clipboard read, like source. In-process gating uses monotonic
  invocation arrival before waiting, preventing a queued hotkey from becoming a
  later automatic paste. Across restart it uses persisted wall time; a backwards
  wall clock admits instead of source's potentially indefinite lockout. This is
  a deliberate bounded-clock adaptation. A forward jump can end the restart
  debounce early. Wall timestamp storage is bounded through year 9999.
- Source keeps its global `index` when it clears storage or sees a new clipboard.
  After successful paste, the persisted next index increments; the global still
  holds the index just used. This quirk is preserved while the process lives.
  After list completion, a repeated launch in the same process can keep pasting
  the final item, contrary to V1 README's simple wrap claim. After restart the
  global resets to zero; a completed retained list can then start at the top.
  An active persisted list resumes its next index after explicit ownership recovery.
- A new list inherits the global index. If that index exceeds its length, source
  passes `undefined` to paste. Rust stops visibly and retains the old queue; it
  never fabricates a line, truncates, or silently resets the index. Closing the
  process/relaunching (host idle exit) resets that in-memory quirk.
- The source's failed paste does not increment. V2 also keeps the item unresolved,
  but explicitly requires retry/advance; ordinary launch never retries it.
- Source toasts become protocol messages. Successful paste hides the window via
  host pasteText. Empty/debounced launches use source's close-window behavior via
  `host.window.visibility {visible:false}`; the pinned visibility RPC returns
  JSON `null`. This is separate from strict structured paste acknowledgement.

## Clipboard ownership and recovery

Pinned host pasteText writes the item to the clipboard, hides the app, waits 80 ms
and emits Cmd+V/Ctrl+V. It does not restore the original list. A simple comparison
against the original would therefore destroy queue continuation on every launch.
V2 keeps the original plus `produced` text and a random process owner UUID.

Within the same process, a clipboard value exactly equal to the last acknowledged
produced text is interpreted as extension-owned and uses the retained original.
Any different nonempty valid clipboard is a source new-list transition, retaining
the source global-index quirk. A same-byte user copy is indistinguishable from
owned text and can continue the old queue. This limitation is explicit; the typed
API supplies neither clipboard generation/token nor compare-and-swap. A user may
also change the clipboard between read and paste, or between the host write and
keyboard paste. An ACK cannot identify the destination or prove insertion there.
No automatic restoration/write-back attempts to overwrite such user changes.

After restart, equality with the previous produced item is ambiguous and shows
**Continue saved list** or **Use current clipboard as a new list**. Other valid
clipboard values follow source new-list semantics. The new-list choice rereads
clipboard at click time; continue deliberately uses the saved original even if
clipboard changed, and its paste will write the selected line. The view never
embeds the clipboard contents in markdown or action descriptors.

Every paste has a durable pending checkpoint before dispatch. Failure, error ACK,
malformed ACK, timeout, cancellation, dropped session, EOF or crash can leave it
pending. Reopen shows **Retry exact saved item (may duplicate)** and **Mark item
handled and advance (may skip)**. Retry sends the saved exact bytes, with no
clipboard reread. Advance records the user's decision, closes the view's window,
and does not paste the next line until another command launch. All choices are
one-use, bound to the SDK session and exact persisted revision; stale/duplicate
choices are rejected before another await/effect. Enter invokes the first choice;
Cmd+K exposes the second. The Detail mouse fix in `host-patches/` is required.

ACK is accepted only as `{ok:true}` with optional string `message`; null, boolean,
extra fields, errors, contradictory fields and non-string messages are rejected.
Host ACK and filesystem writes are not a distributed transaction. Cancellation
can occur after a real paste; persistence can fail after ACK. A checkpoint may
also remain pending although cancellation prevented dispatch. This conservative
policy provides neither exactly-once delivery nor an automatic replay guarantee.

## Storage, bounds and privacy

One Tokio mutex serializes the whole transition/effect across sessions of one
host-owned process/profile. No std mutex crosses await. The adapter refuses a
changed data directory in that process. The host must enforce one process per
extension/profile; this port does not coordinate independent concurrent processes.
SDK session state is not authoritative because SDK staging can discard it while
external effects persist. Recovery registrations live outside staged state, are
consumed synchronously, and are owned by an SDK-scoped inert drop guard, including
when cancellation occurs before its first poll. No detached domain tasks/threads,
timers, polling or background leases are created. The SDK's own bounded stdin
pump/runner tasks remain SDK-owned.

Store: `<host-private-dataDir>/clipboard-sequential-paste-v2/queue.json`. Source
already persisted plaintext clipboard lists. V2 likewise stores plaintext original,
last produced line and unresolved exact item; it adds no credentials/encryption
API. Unlike source's completion clear, V2 retains the original after completion so
host-overwritten clipboard text can continue/recreate source behavior. A successful
new valid list replaces that original; an empty/invalid input or storage/read error
retains it. Failed paste retains a pending item. There is no automatic V1 migration,
history, source-store cleanup, or UI clear control. To clear deliberately, stop the
extension process and remove only this private V2 directory offline; doing so loses
unresolved delivery evidence and is not performed by this task. Other source/profile
entries are never read or removed.

Clipboard input <=256 KiB UTF-8, <=4096 lines, <=64 KiB per item; store <=4 MiB,
128 recovery registrations (bounded by SDK sessions), 10-second per-host-call
timeout, command host timeout 30 seconds. Filesystem writes are bounded in size,
not by a guaranteed kernel fsync deadline. Invalid input is rejected without silent
truncation. Absolute private paths, every existing symlink component, regular file
kind, store schema, field types, queue/pending/index invariants, revision/timestamp
limits and size are checked. Atomic private temporary file + fsync + replacement
is used, with directory fsync on Unix. Failed replacement cannot dispatch paste;
a failure after replacement remains conservative and requires reopening. Private
subdirectory mode is 0700 on Unix; temp/checkpoint files are 0600. Host private-path
and process ownership are trusted; this is not a sandbox against a malicious same-user
process racing path checks. Windows durability/ACL behavior remains unverified.
Corruption/unreadability yields an actionable view and never an automatic wipe.
No clipboard data is logged or used in evidence: all fixtures/tests are synthetic.

See [QA.md](QA.md), [INTEGRATION.md](INTEGRATION.md), [NOTICE](NOTICE) and
[host-patches/README.md](host-patches/README.md). Real macOS and Windows clipboard,
focus, permissions, rendering and paste acceptance are **not run**.
