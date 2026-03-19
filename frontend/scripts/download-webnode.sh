#!/bin/bash

set -euo pipefail

MINA_BASE_URL="${MINA_BASE_URL:-https://github.com/o1-labs}"
CIRCUITS_BASE_URL="${CIRCUITS_BASE_URL:-$MINA_BASE_URL/circuit-blobs/releases/download}"
CIRCUITS_VERSION="${CIRCUITS_VERSION:-berkeley-devnet}"
SCRIPT_DIR="${SCRIPT_DIR:-$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
REPO_ROOT="${REPO_ROOT:-$(cd -- "$SCRIPT_DIR/../.." && pwd)}"
DOWNLOAD_ROOT="${DOWNLOAD_ROOT:-$REPO_ROOT/frontend/src/assets/webnode/circuit-blobs}"
DOWNLOAD_DIR="$DOWNLOAD_ROOT/$CIRCUITS_VERSION"
CURL_BIN="${CURL_BIN:-curl}"
CARGO_BIN="${CARGO_BIN:-cargo}"

DEVNET_CIRCUIT_FILES=(
    "step-step-proving-key-blockchain-snark-step-0-55f640777b6486a6fd3fdbc3fcffcc60_gates.json"
    "step-step-proving-key-blockchain-snark-step-0-55f640777b6486a6fd3fdbc3fcffcc60_internal_vars.bin"
    "step-step-proving-key-blockchain-snark-step-0-55f640777b6486a6fd3fdbc3fcffcc60_rows_rev.bin"
    "step-step-proving-key-transaction-snark-merge-1-ba1d52dfdc2dd4d2e61f6c66ff2a5b2f_gates.json"
    "step-step-proving-key-transaction-snark-merge-1-ba1d52dfdc2dd4d2e61f6c66ff2a5b2f_internal_vars.bin"
    "step-step-proving-key-transaction-snark-merge-1-ba1d52dfdc2dd4d2e61f6c66ff2a5b2f_rows_rev.bin"
    "step-step-proving-key-transaction-snark-opt_signed-3-9eefed16953d2bfa78a257adece02d47_gates.json"
    "step-step-proving-key-transaction-snark-opt_signed-3-9eefed16953d2bfa78a257adece02d47_internal_vars.bin"
    "step-step-proving-key-transaction-snark-opt_signed-3-9eefed16953d2bfa78a257adece02d47_rows_rev.bin"
    "step-step-proving-key-transaction-snark-opt_signed-opt_signed-2-48925e6a97197028e1a7c1ecec09021d_gates.json"
    "step-step-proving-key-transaction-snark-opt_signed-opt_signed-2-48925e6a97197028e1a7c1ecec09021d_internal_vars.bin"
    "step-step-proving-key-transaction-snark-opt_signed-opt_signed-2-48925e6a97197028e1a7c1ecec09021d_rows_rev.bin"
    "step-step-proving-key-transaction-snark-proved-4-0cafcbc6dffccddbc82f8c2519c16341_gates.json"
    "step-step-proving-key-transaction-snark-proved-4-0cafcbc6dffccddbc82f8c2519c16341_internal_vars.bin"
    "step-step-proving-key-transaction-snark-proved-4-0cafcbc6dffccddbc82f8c2519c16341_rows_rev.bin"
    "step-step-proving-key-transaction-snark-transaction-0-c33ec5211c07928c87e850a63c6a2079_gates.json"
    "step-step-proving-key-transaction-snark-transaction-0-c33ec5211c07928c87e850a63c6a2079_internal_vars.bin"
    "step-step-proving-key-transaction-snark-transaction-0-c33ec5211c07928c87e850a63c6a2079_rows_rev.bin"
    "wrap-wrap-proving-key-blockchain-snark-bbecaf158ca543ec8ac9e7144400e669_gates.json"
    "wrap-wrap-proving-key-blockchain-snark-bbecaf158ca543ec8ac9e7144400e669_internal_vars.bin"
    "wrap-wrap-proving-key-blockchain-snark-bbecaf158ca543ec8ac9e7144400e669_rows_rev.bin"
    "wrap-wrap-proving-key-transaction-snark-b9a01295c8cc9bda6d12142a581cd305_gates.json"
    "wrap-wrap-proving-key-transaction-snark-b9a01295c8cc9bda6d12142a581cd305_internal_vars.bin"
    "wrap-wrap-proving-key-transaction-snark-b9a01295c8cc9bda6d12142a581cd305_rows_rev.bin"
)

GENERATED_VERIFIER_FILES=(
    "block_verifier_index.postcard"
    "transaction_verifier_index.postcard"
)

download_release_asset() {
    local file="$1"
    echo "Downloading $file to $DOWNLOAD_DIR..."
    "$CURL_BIN" -fsSL --retry 3 --retry-delay 5 \
        -o "$DOWNLOAD_DIR/$file" \
        "$CIRCUITS_BASE_URL/$CIRCUITS_VERSION/$file"
    echo "$file downloaded successfully to $DOWNLOAD_DIR"
}

ensure_file_exists() {
    local file="$1"
    if [[ ! -s "$DOWNLOAD_DIR/$file" ]]; then
        echo "Expected asset missing or empty: $DOWNLOAD_DIR/$file" >&2
        exit 1
    fi
}

generate_verifier_postcards() {
    local network
    case "$CIRCUITS_VERSION" in
        berkeley-devnet)
            network="devnet"
            ;;
        3.0.0mainnet)
            network="mainnet"
            ;;
        *)
            echo "Unsupported circuits version for local verifier generation: $CIRCUITS_VERSION" >&2
            exit 1
            ;;
    esac

    echo "Generating verifier postcards locally for $network..."
    (
        cd "$REPO_ROOT"
        "$CARGO_BIN" run --quiet -p generate-webnode-circuit-blobs -- \
            --network "$network" \
            --out-root "$DOWNLOAD_ROOT"
    )
}

download_circuit_files() {
    mkdir -p "$DOWNLOAD_DIR"

    for file in "${DEVNET_CIRCUIT_FILES[@]}"; do
        if [[ -f "$DOWNLOAD_DIR/$file" ]]; then
            echo "$file already exists in $DOWNLOAD_DIR, skipping download."
        else
            download_release_asset "$file"
        fi
        ensure_file_exists "$file"
    done

    generate_verifier_postcards

    for file in "${GENERATED_VERIFIER_FILES[@]}"; do
        ensure_file_exists "$file"
    done
}

download_circuit_files
