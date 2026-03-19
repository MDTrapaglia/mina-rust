use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use ledger::proofs::{
    caching::verifier_index_to_bytes,
    verifiers::{BlockVerifier, TransactionVerifier},
    VerifierIndex,
};
use mina_core::NetworkConfig;
use mina_curves::pasta::Fq;
use sha2::{Digest, Sha256};

const DEVNET_BLOCK_VERIFIER_SRC: &str =
    include_str!("../../../crates/ledger/src/proofs/data/devnet_blockchain_verifier_index.json");
const DEVNET_TRANSACTION_VERIFIER_SRC: &str =
    include_str!("../../../crates/ledger/src/proofs/data/devnet_transaction_verifier_index.json");
const MAINNET_BLOCK_VERIFIER_SRC: &str =
    include_str!("../../../crates/ledger/src/proofs/data/mainnet_blockchain_verifier_index.json");
const MAINNET_TRANSACTION_VERIFIER_SRC: &str =
    include_str!("../../../crates/ledger/src/proofs/data/mainnet_transaction_verifier_index.json");

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "devnet")]
    network: String,

    #[arg(long, default_value = "/home/mtrapaglia/mina/assets/webnode/circuit-blobs")]
    out_root: PathBuf,
}

fn source_json(network: &str, is_block: bool) -> &'static str {
    match (network, is_block) {
        ("devnet", true) => DEVNET_BLOCK_VERIFIER_SRC,
        ("devnet", false) => DEVNET_TRANSACTION_VERIFIER_SRC,
        ("mainnet", true) => MAINNET_BLOCK_VERIFIER_SRC,
        ("mainnet", false) => MAINNET_TRANSACTION_VERIFIER_SRC,
        (other, _) => panic!("unsupported network '{other}'"),
    }
}

fn blob_bytes_from_payload(src_json: &str, verifier_index_bytes: &[u8]) -> Vec<u8> {
    let src_digest = Sha256::digest(src_json.as_bytes());
    let verifier_index_digest = Sha256::digest(verifier_index_bytes);

    let mut blob = Vec::with_capacity(64 + verifier_index_bytes.len());
    blob.extend_from_slice(&src_digest);
    blob.extend_from_slice(&verifier_index_digest);
    blob.extend_from_slice(verifier_index_bytes);
    blob
}

fn blob_bytes(src_json: &str, verifier_index: &VerifierIndex<Fq>) -> Result<Vec<u8>> {
    let verifier_index_bytes = verifier_index_to_bytes(verifier_index)
        .context("serializing verifier index to postcard bytes")?;
    Ok(blob_bytes_from_payload(src_json, &verifier_index_bytes))
}

fn write_blob(path: &Path, src_json: &str, verifier_index: &VerifierIndex<Fq>) -> Result<()> {
    let blob = blob_bytes(src_json, verifier_index)?;
    fs::write(path, blob).with_context(|| format!("writing {}", path.display()))
}

fn main() -> Result<()> {
    let args = Args::parse();
    NetworkConfig::init(&args.network)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("initializing network '{}'", args.network))?;

    let circuits_dir = NetworkConfig::global().circuits_config.directory_name;
    let out_dir = args.out_root.join(circuits_dir);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating {}", out_dir.display()))?;

    let block_path = out_dir.join("block_verifier_index.postcard");
    let tx_path = out_dir.join("transaction_verifier_index.postcard");

    let block_verifier = BlockVerifier::make();
    write_blob(
        &block_path,
        source_json(NetworkConfig::global().name, true),
        &block_verifier,
    )?;

    let tx_verifier = TransactionVerifier::make();
    write_blob(
        &tx_path,
        source_json(NetworkConfig::global().name, false),
        &tx_verifier,
    )?;

    println!("{}", block_path.display());
    println!("{}", tx_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        blob_bytes_from_payload, source_json, DEVNET_BLOCK_VERIFIER_SRC,
        DEVNET_TRANSACTION_VERIFIER_SRC, MAINNET_BLOCK_VERIFIER_SRC,
        MAINNET_TRANSACTION_VERIFIER_SRC,
    };
    use sha2::{Digest, Sha256};

    #[test]
    fn source_json_selects_expected_embedded_sources() {
        assert_eq!(source_json("devnet", true), DEVNET_BLOCK_VERIFIER_SRC);
        assert_eq!(source_json("devnet", false), DEVNET_TRANSACTION_VERIFIER_SRC);
        assert_eq!(source_json("mainnet", true), MAINNET_BLOCK_VERIFIER_SRC);
        assert_eq!(source_json("mainnet", false), MAINNET_TRANSACTION_VERIFIER_SRC);
    }

    #[test]
    fn blob_bytes_prefixes_source_and_payload_digests() {
        let src_json = r#"{"kind":"block"}"#;
        let payload = b"payload-bytes";
        let blob = blob_bytes_from_payload(src_json, payload);

        assert_eq!(&blob[..32], &Sha256::digest(src_json.as_bytes())[..]);
        assert_eq!(&blob[32..64], &Sha256::digest(payload)[..]);
        assert_eq!(&blob[64..], payload);
    }
}
