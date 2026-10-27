#!/bin/sh
set -eu
port_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
qa_home="$port_root/.qa/home"
qa_tmp="$port_root/.qa/tmp"
qa_cargo="$port_root/.qa/cargo-home"
rustup_root="${RUSTUP_HOME:-$HOME/.rustup}"
mkdir -p "$qa_home" "$qa_tmp" "$port_root/.qa/v1" "$port_root/.qa/schema"
run() { env HOME="$qa_home" TMPDIR="$qa_tmp" RUSTUP_HOME="$rustup_root" CARGO_HOME="$qa_cargo" CARGO_TARGET_DIR="$port_root/target" "$@"; }
# Frozen source and lock copy, never npm install in V1 checkout.
cp -R "$port_root/fixtures/own/." "$port_root/.qa/v1/"
run npm ci --prefix "$port_root/.qa/v1" --ignore-scripts --legacy-peer-deps --cache "$port_root/.qa/npm-cache" --no-audit --no-fund
cp "$port_root/tests/schema/package.json" "$port_root/tests/schema/package-lock.json" "$port_root/.qa/schema/"
run npm ci --prefix "$port_root/.qa/schema" --ignore-scripts --cache "$port_root/.qa/npm-cache" --no-audit --no-fund
run npm --prefix "$port_root/.qa/v1" test
run npm --prefix "$port_root/.qa/v1" run typecheck
run node "$port_root/scripts/source-oracle.cjs" --check
run python3 "$port_root/scripts/prepare-host-patches.py"
run python3 "$port_root/scripts/host-fixtures.py" --check
run node "$port_root/scripts/renderer-qa.cjs"
run node "$port_root/scripts/schema-qa.cjs"
run cargo fmt --manifest-path "$port_root/Cargo.toml" --check
for toolchain in stable 1.85.0; do
 run cargo "+$toolchain" test --manifest-path "$port_root/Cargo.toml" --locked
 run cargo "+$toolchain" clippy --manifest-path "$port_root/Cargo.toml" --locked --all-targets -- -D warnings
 run cargo "+$toolchain" build --manifest-path "$port_root/Cargo.toml" --locked
 run python3 "$port_root/scripts/process-qa.py" "$port_root/target/debug/clipboard-sequential-paste"
done
run cargo +stable build --release --manifest-path "$port_root/Cargo.toml" --locked
run python3 "$port_root/scripts/package-dev.py"
run python3 "$port_root/scripts/audit.py"
