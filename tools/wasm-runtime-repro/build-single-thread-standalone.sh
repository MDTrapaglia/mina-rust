#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SOURCE_LIB="${SCRIPT_DIR}/src/lib.rs"
OUT_DIR="${SCRIPT_DIR}/pkg-single"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/mina-wasm-runtime-repro-single-XXXXXX")"

cleanup() {
  rm -rf "${TMP_ROOT}"
}
trap cleanup EXIT

mkdir -p "${TMP_ROOT}/src" "${OUT_DIR}"
cp "${SOURCE_LIB}" "${TMP_ROOT}/src/lib.rs"

cat > "${TMP_ROOT}/Cargo.toml" <<'EOF'
[package]
name = "wasm-runtime-repro"
version = "0.19.0"
edition = "2021"
license = "Apache-2.0"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
console_error_panic_hook = "0.1"
js-sys = "=0.3.83"
sha2 = "0.10"
wasm-bindgen = "=0.2.106"
wasm-bindgen-futures = "=0.4.56"
EOF

(
  cd "${TMP_ROOT}"
  cargo +nightly build \
    --release \
    --target wasm32-unknown-unknown \
    --target-dir "${TMP_ROOT}/target"
)

wasm-bindgen --keep-debug --target web \
  --out-dir "${OUT_DIR}" \
  "${TMP_ROOT}/target/wasm32-unknown-unknown/release/wasm_runtime_repro.wasm"
