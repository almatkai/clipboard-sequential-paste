# Local search coverage — incomplete for V2

Canonical root: `/Users/almat/Desktop/code/Tezbar APP/rust-extension-ports/clipboard-sequential-paste`.
SHA256-derived namespace: `ax-248e0eda4ed3` (first 12 hex of canonical root hash).
Own state: `.git/xerj-state`. No other checkout or shared namespace was indexed.

Initial autoindex used `--no-graph --workers 2 --disable-feedback`; generation 1
committed 53 records, all four source code files indexed, two images skipped as
junk (reported completion-with-junk). Initial `clipboard paste storage debounce`
search returned the actual own command, manifest, README and source smoke test.
This happened before full source/lockfile reads and design.

Final V2 autoindex using the SAME root, prefix and state directory failed:

```
error: project new file v2/Cargo.toml: no frozen dataset accepts family docs,
group None, and fields ["end_line", "start_line", "text"]; this requires
unsupported dataset/schema evolution (use a new prefix)
```

No new prefix, namespace bypass, schema mutation or global server restart was
attempted. The final index does not cover V2. A subsequent search for `pending queue
checkpoint` in the original prefix returned no match; that stale lexical result is
not evidence that the implementation is absent. No 429 or RSS failure was observed
in this attempt; the observed blocker is the frozen schema.

Exact/full `rg`, direct full-file reads, executed production/domain/host fixture
checks, byte-for-byte V1 audit, immutable fixture hashes and the full Git-visible
`INDEX.sha256` inventory provide the fallback coverage. See QA for executed tests.
