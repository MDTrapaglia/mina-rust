#!/usr/bin/env bash

set -euo pipefail

SCRIPT_UNDER_TEST="${SCRIPT_UNDER_TEST:-$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/download-webnode.sh}"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

assert_file_exists() {
    local path="$1"
    [[ -s "$path" ]] || fail "expected file missing or empty: $path"
}

assert_contains() {
    local path="$1"
    local needle="$2"
    grep -Fq "$needle" "$path" || fail "expected '$needle' in $path"
}

make_fake_curl() {
    local path="$1"
    cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

out=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o)
            out="$2"
            shift 2
            ;;
        --retry|--retry-delay)
            shift 2
            ;;
        -fsSL)
            shift
            ;;
        *)
            shift
            ;;
    esac
done

[[ -n "$out" ]] || exit 1
mkdir -p "$(dirname "$out")"
printf 'stub-asset\n' >"$out"
EOF
    chmod +x "$path"
}

make_fake_cargo() {
    local path="$1"
    cat >"$path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

network=""
out_root=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --network)
            network="$2"
            shift 2
            ;;
        --out-root)
            out_root="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

[[ -n "$network" && -n "$out_root" ]] || exit 1

case "$network" in
    devnet)
        dir="berkeley-devnet"
        ;;
    mainnet)
        dir="3.0.0mainnet"
        ;;
    *)
        exit 1
        ;;
esac

mkdir -p "$out_root/$dir"
printf '%s\n' "$network" >"$out_root/invoked-network.txt"
printf 'block\n' >"$out_root/$dir/block_verifier_index.postcard"

if [[ "${FAKE_CARGO_MODE:-ok}" == "ok" ]]; then
    printf 'tx\n' >"$out_root/$dir/transaction_verifier_index.postcard"
fi
EOF
    chmod +x "$path"
}

run_download_script() {
    local tmp="$1"
    local circuits_version="$2"
    local cargo_mode="${3:-ok}"
    local stub_dir="$tmp/stubs"

    mkdir -p "$stub_dir"
    mkdir -p "$tmp/repo-root"
    make_fake_curl "$stub_dir/curl"
    make_fake_cargo "$stub_dir/cargo"

    PATH="$stub_dir:$PATH" \
        DOWNLOAD_ROOT="$tmp/downloads" \
        REPO_ROOT="$tmp/repo-root" \
        CIRCUITS_VERSION="$circuits_version" \
        FAKE_CARGO_MODE="$cargo_mode" \
        bash "$SCRIPT_UNDER_TEST"
}

test_generates_postcards_for_berkeley_devnet() {
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    run_download_script "$tmp" "berkeley-devnet"

    assert_contains "$tmp/downloads/invoked-network.txt" "devnet"
    assert_file_exists "$tmp/downloads/berkeley-devnet/block_verifier_index.postcard"
    assert_file_exists "$tmp/downloads/berkeley-devnet/transaction_verifier_index.postcard"
}

test_generates_postcards_for_mainnet() {
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    run_download_script "$tmp" "3.0.0mainnet"

    assert_contains "$tmp/downloads/invoked-network.txt" "mainnet"
    assert_file_exists "$tmp/downloads/3.0.0mainnet/block_verifier_index.postcard"
    assert_file_exists "$tmp/downloads/3.0.0mainnet/transaction_verifier_index.postcard"
}

test_fails_fast_when_generator_misses_postcard() {
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    if run_download_script "$tmp" "berkeley-devnet" "missing-tx"; then
        fail "download script should fail when a generated postcard is missing"
    fi
}

test_generates_postcards_for_berkeley_devnet
test_generates_postcards_for_mainnet
test_fails_fast_when_generator_misses_postcard

echo "download-webnode.sh tests: ok"
