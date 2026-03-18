use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use mina_core::{info, log::system_time, warn};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use kimchi::{
    circuits::{
        constraints::FeatureFlags,
        expr::Linearization,
        lookup::lookups::{LookupFeatures, LookupPatterns},
        polynomials::permutation::{permutation_vanishing_polynomial, zk_w},
    },
    linearization::{constraints_expr, expr_linearization, linearization_columns},
    mina_curves::pasta::Pallas,
};
use mina_curves::pasta::{Fp, Fq};
use poly_commitment::{ipa::SRS, SRS as _};

use crate::{proofs::BACKEND_TOCK_ROUNDS_N, VerificationKey};

use super::{
    transaction::{endos, InnerCurve},
    wrap::{Domain, Domains},
    VerifierIndex,
};

#[derive(Clone, Copy)]
enum Kind {
    BlockVerifier,
    TransactionVerifier,
}

impl Kind {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::BlockVerifier => "block_verifier_index",
            Self::TransactionVerifier => "transaction_verifier_index",
        }
    }

    pub fn filename(self) -> String {
        format!("{}.postcard", self.to_str())
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(inline_js = r#"
export function codex_trace_verifiers(stage) {
  const url = '/wasm-smoke/trace?stage=' + encodeURIComponent(stage);
  self.fetch(url, { cache: 'no-store' }).catch(() => undefined);
}

export function codex_trace_verifiers_now() {
  if (self.performance && typeof self.performance.now === 'function') {
    return self.performance.now();
  }
  return Date.now();
}"#)]
extern "C" {
    fn codex_trace_verifiers(stage: &str);
    fn codex_trace_verifiers_now() -> f64;
}

#[cfg(target_family = "wasm")]
fn trace_stage(kind: Kind, stage: &str) {
    let stage = format!("ledger-verifiers:{}:{stage}", kind.to_str());
    codex_trace_verifiers(&stage);
}

#[cfg(target_family = "wasm")]
fn trace_now_ms() -> u64 {
    codex_trace_verifiers_now().round() as u64
}

fn cache_filename(kind: Kind) -> PathBuf {
    let circuits_config = mina_core::NetworkConfig::global().circuits_config;
    Path::new(circuits_config.directory_name).join(kind.filename())
}

#[cfg(not(target_family = "wasm"))]
fn verify_payload_digest(_kind: Kind, expected: &[u8; 32], slice: &[u8]) -> anyhow::Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(slice);
    let digest = hasher.finalize();
    if expected != digest.as_slice() {
        anyhow::bail!("verifier index digest verification failed");
    }
    Ok(())
}

#[cfg(target_family = "wasm")]
fn verify_payload_digest(kind: Kind, expected: &[u8; 32], slice: &[u8]) -> anyhow::Result<()> {
    const HASH_CHUNK_SIZE: usize = 8 * 1024;

    let mut hasher = Sha256::new();
    let hash_started_at_ms = trace_now_ms();
    trace_stage(kind, "read_cache.payload_digest.verify.begin");
    trace_stage(
        kind,
        &format!("read_cache.payload_digest.len.{}", slice.len()),
    );
    trace_stage(
        kind,
        &format!("read_cache.payload_digest.hash.chunk_size.{HASH_CHUNK_SIZE}"),
    );
    trace_stage(kind, "read_cache.payload_digest.hash.begin");

    let probe_len = slice.len().min(8 * 1024);
    if probe_len > 1 {
        let mut probe = Sha256::new();
        let split = probe_len / 2;
        trace_stage(
            kind,
            &format!("read_cache.payload_digest.probe.begin.{probe_len}"),
        );
        trace_stage(
            kind,
            &format!("read_cache.payload_digest.probe.chunk.0.begin.{split}"),
        );
        probe.update(&slice[..split]);
        trace_stage(
            kind,
            &format!("read_cache.payload_digest.probe.chunk.0.complete.{split}"),
        );
        trace_stage(
            kind,
            &format!(
                "read_cache.payload_digest.probe.chunk.1.begin.{}",
                probe_len - split
            ),
        );
        probe.update(&slice[split..probe_len]);
        trace_stage(
            kind,
            &format!(
                "read_cache.payload_digest.probe.chunk.1.complete.{}",
                probe_len
            ),
        );
        let _ = probe.finalize();
        trace_stage(kind, "read_cache.payload_digest.probe.complete");
    }

    let mut processed = 0usize;
    for (chunk_idx, chunk) in slice.chunks(HASH_CHUNK_SIZE).enumerate() {
        let next_processed = processed + chunk.len();
        let chunk_started_at_ms = trace_now_ms();
        if chunk_idx < 4 || next_processed == slice.len() || chunk_idx % 16 == 0 {
            trace_stage(
                kind,
                &format!(
                    "read_cache.payload_digest.hash.chunk.{chunk_idx}.begin.{processed}.{}.t{}",
                    chunk.len(),
                    chunk_started_at_ms.saturating_sub(hash_started_at_ms)
                ),
            );
        }
        hasher.update(chunk);
        let chunk_completed_at_ms = trace_now_ms();
        processed = next_processed;

        let chunk_elapsed_ms = chunk_completed_at_ms.saturating_sub(chunk_started_at_ms);
        if chunk_idx < 4 || processed == slice.len() || chunk_idx % 16 == 0 || chunk_elapsed_ms >= 250
        {
            trace_stage(
                kind,
                &format!(
                    "read_cache.payload_digest.hash.chunk.{chunk_idx}.complete.{processed}.dt{}.t{}",
                    chunk_elapsed_ms,
                    chunk_completed_at_ms.saturating_sub(hash_started_at_ms)
                ),
            );
        }

        if processed == chunk.len() || processed == slice.len() || processed % (1024 * 1024) == 0
        {
            trace_stage(
                kind,
                &format!(
                    "read_cache.payload_digest.hash.progress.{processed}.t{}",
                    chunk_completed_at_ms.saturating_sub(hash_started_at_ms)
                ),
            );
        }
    }

    trace_stage(
        kind,
        &format!(
            "read_cache.payload_digest.hash.complete.t{}",
            trace_now_ms().saturating_sub(hash_started_at_ms)
        ),
    );
    trace_stage(kind, "read_cache.payload_digest.finalize.begin");
    let digest = hasher.finalize();
    trace_stage(kind, "read_cache.payload_digest.finalize.complete");
    trace_stage(kind, "read_cache.payload_digest.compare.begin");
    if expected != digest.as_slice() {
        anyhow::bail!("verifier index digest verification failed");
    }
    trace_stage(kind, "read_cache.payload_digest.compare.complete");
    trace_stage(kind, "read_cache.payload_digest.verify.complete");
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn cache_path(kind: Kind) -> Option<PathBuf> {
    super::circuit_blobs::home_base_dir().map(|p| p.join(cache_filename(kind)))
}

macro_rules! read_cache {
    ($kind: expr, $digest: expr) => {{
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.begin");
        #[cfg(not(target_family = "wasm"))]
        let data = super::circuit_blobs::fetch_blocking(&cache_filename($kind))
            .context("fetching verifier index failed")?;
        #[cfg(target_family = "wasm")]
        let data = {
            trace_stage($kind, "read_cache.fetch.begin");
            let data = super::circuit_blobs::fetch(&cache_filename($kind))
                .await
                .context("fetching verifier index failed")?;
            trace_stage($kind, "read_cache.fetch.complete");
            data
        };
        let mut slice = data.as_slice();
        let mut d = [0; 32];
        // source digest
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.source_digest.read.begin");
        slice.read_exact(&mut d).context("reading source digest")?;
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.source_digest.read.complete");
        if d != $digest {
            anyhow::bail!("source digest verification failed");
        }

        // index digest
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.index_digest.read.begin");
        slice.read_exact(&mut d).context("reading index digest")?;
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.index_digest.read.complete");
        verify_payload_digest($kind, &d, slice)?;
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.decode.begin");
        let verifier_index = super::caching::verifier_index_from_bytes(slice)?;
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "read_cache.decode.complete");
        Ok(verifier_index)
    }};
}

#[cfg(not(target_family = "wasm"))]
fn read_cache(kind: Kind, digest: &[u8]) -> anyhow::Result<VerifierIndex<Fq>> {
    read_cache!(kind, digest)
}

#[cfg(target_family = "wasm")]
async fn read_cache(kind: Kind, digest: &[u8]) -> anyhow::Result<VerifierIndex<Fq>> {
    read_cache!(kind, digest)
}

#[cfg(not(target_family = "wasm"))]
fn write_cache(kind: Kind, index: &VerifierIndex<Fq>, digest: &[u8]) -> anyhow::Result<()> {
    use std::{fs::File, io::Write};

    let path = cache_path(kind)
        .ok_or_else(|| anyhow::anyhow!("$HOME env not set, so can't cache verifier index"))?;
    let bytes = super::caching::verifier_index_to_bytes(index)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let Some(parent) = path.parent() else {
        anyhow::bail!("cannot get parent for {path:?}");
    };
    std::fs::create_dir_all(parent).context("creating cache file parent directory")?;
    let mut file = File::create(path).context("creating cache file")?;
    file.write_all(digest).context("storing source digest")?;
    file.write_all(&hasher.finalize())
        .context("storing verifier index digest")?;
    file.write_all(&bytes)
        .context("storing verifier index into cache file")?;
    Ok(())
}

macro_rules! make_with_ext_cache {
    ($kind: expr, $data: expr) => {{
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "parse.begin");
        let verifier_index: VerifierIndex<Fq> = serde_json::from_str($data).unwrap();
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "parse.complete");
        let mut hasher = Sha256::new();
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "src_digest.begin");
        hasher.update($data);
        let src_index_digest = hasher.finalize();
        #[cfg(target_family = "wasm")]
        trace_stage($kind, "src_digest.complete");

        #[cfg(not(target_family = "wasm"))]
        let cache = read_cache($kind, &src_index_digest);
        #[cfg(target_family = "wasm")]
        let cache = {
            trace_stage($kind, "read_cache.call.begin");
            let cache = read_cache($kind, &src_index_digest).await;
            trace_stage($kind, "read_cache.call.complete");
            cache
        };

        match cache {
            Ok(verifier_index) => {
                #[cfg(target_family = "wasm")]
                trace_stage($kind, "cache.hit");
                info!(system_time(); "Verifier index is loaded");
                verifier_index
            }
            Err(err) => {
                #[cfg(target_family = "wasm")]
                trace_stage($kind, "cache.miss");
                warn!(system_time(); "Cannot load verifier index: {err}");
                #[cfg(target_family = "wasm")]
                trace_stage($kind, "make_verifier_index.begin");
                let index = make_verifier_index($kind, verifier_index);
                #[cfg(target_family = "wasm")]
                trace_stage($kind, "make_verifier_index.complete");
                #[cfg(not(target_family = "wasm"))]
                if let Err(err) = write_cache($kind, &index, &src_index_digest) {
                    warn!(system_time(); "Cannot store verifier index to cache file: {err}");
                } else {
                    info!(system_time(); "Stored verifier index to cache file");
                }
                index
            }
        }
    }}
}

#[cfg(not(target_family = "wasm"))]
fn make_with_ext_cache(kind: Kind, data: &str) -> VerifierIndex<Fq> {
    make_with_ext_cache!(kind, data)
}

#[cfg(target_family = "wasm")]
async fn make_with_ext_cache(kind: Kind, data: &str) -> VerifierIndex<Fq> {
    make_with_ext_cache!(kind, data)
}

/// Verifier index for block proofs (consensus layer / block selection).
/// Lazily initialized and cached globally.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockVerifier(Arc<VerifierIndex<Fq>>);

/// Verifier index for transaction proofs (execution layer / transaction confirmation).
/// Lazily initialized and cached globally.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionVerifier(Arc<VerifierIndex<Fq>>);

static BLOCK_VERIFIER: OnceCell<BlockVerifier> = OnceCell::new();
static TX_VERIFIER: OnceCell<TransactionVerifier> = OnceCell::new();

impl BlockVerifier {
    fn kind() -> Kind {
        Kind::BlockVerifier
    }

    fn src_json() -> &'static str {
        let network_name = mina_core::NetworkConfig::global().name;
        match network_name {
            "mainnet" => include_str!("data/mainnet_blockchain_verifier_index.json"),
            "devnet" => include_str!("data/devnet_blockchain_verifier_index.json"),
            other => panic!("get_verifier_index: unknown network '{other}'"),
        }
    }
}

impl TransactionVerifier {
    fn kind() -> Kind {
        Kind::TransactionVerifier
    }

    fn src_json() -> &'static str {
        let network_name = mina_core::NetworkConfig::global().name;
        match network_name {
            "mainnet" => include_str!("data/mainnet_transaction_verifier_index.json"),
            "devnet" => include_str!("data/devnet_transaction_verifier_index.json"),
            other => panic!("get_verifier_index: unknown network '{other}'"),
        }
    }

    pub fn get() -> Option<Self> {
        TX_VERIFIER.get().cloned()
    }
}

#[cfg(not(target_family = "wasm"))]
impl BlockVerifier {
    /// Creates or returns cached block verifier index from embedded JSON data.
    /// Network-specific (mainnet/devnet). Uses external cache for faster loading.
    pub fn make() -> Self {
        BLOCK_VERIFIER
            .get_or_init(|| {
                Self(Arc::new(make_with_ext_cache(
                    Self::kind(),
                    Self::src_json(),
                )))
            })
            .clone()
    }
}

#[cfg(target_family = "wasm")]
impl BlockVerifier {
    pub async fn make() -> Self {
        if let Some(v) = BLOCK_VERIFIER.get() {
            v.clone()
        } else {
            let verifier = Self(Arc::new(
                make_with_ext_cache(Self::kind(), Self::src_json()).await,
            ));
            BLOCK_VERIFIER.get_or_init(move || verifier).clone()
        }
    }
}

#[cfg(not(target_family = "wasm"))]
impl TransactionVerifier {
    /// Creates or returns cached transaction verifier index from embedded JSON.
    /// Network-specific (mainnet/devnet). Uses external cache for faster loading.
    pub fn make() -> Self {
        TX_VERIFIER
            .get_or_init(|| {
                Self(Arc::new(make_with_ext_cache(
                    Self::kind(),
                    Self::src_json(),
                )))
            })
            .clone()
    }
}

#[cfg(target_family = "wasm")]
impl TransactionVerifier {
    pub async fn make() -> Self {
        if let Some(v) = TX_VERIFIER.get() {
            v.clone()
        } else {
            let verifier = Self(Arc::new(
                make_with_ext_cache(Self::kind(), Self::src_json()).await,
            ));
            TX_VERIFIER.get_or_init(move || verifier).clone()
        }
    }
}

impl std::ops::Deref for BlockVerifier {
    type Target = VerifierIndex<Fq>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for TransactionVerifier {
    type Target = VerifierIndex<Fq>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BlockVerifier> for Arc<VerifierIndex<Fq>> {
    fn from(value: BlockVerifier) -> Self {
        value.0
    }
}

impl From<TransactionVerifier> for Arc<VerifierIndex<Fq>> {
    fn from(value: TransactionVerifier) -> Self {
        value.0
    }
}

fn make_verifier_index(kind: Kind, index: VerifierIndex<Fq>) -> VerifierIndex<Fq> {
    let domain = index.domain;
    let max_poly_size: usize = index.max_poly_size;
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.endo.begin");
    let (endo, _) = endos::<Fq>();
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.endo.complete");

    let feature_flags = FeatureFlags {
        range_check0: false,
        range_check1: false,
        foreign_field_add: false,
        foreign_field_mul: false,
        xor: false,
        rot: false,
        lookup_features: LookupFeatures {
            patterns: LookupPatterns {
                xor: false,
                lookup: false,
                range_check: false,
                foreign_field_mul: false,
            },
            joint_lookup_used: false,
            uses_runtime_tables: false,
        },
    };

    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.columns.begin");
    let evaluated_cols = linearization_columns::<Fq>(Some(&feature_flags));
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.columns.complete");

    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.constraints.begin");
    let (expr, powers_of_alpha) = constraints_expr(Some(&feature_flags), true);
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.constraints.complete");

    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.linearize.begin");
    let mut linearization = expr
        .linearize(evaluated_cols)
        .unwrap()
        .map(|e| e.to_polish());
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.linearize.complete");

    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.index_terms.assert.begin");
    assert_eq!(linearization.index_terms.len(), 0);
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.index_terms.assert.complete");

    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.sort.begin");
    let linearization = Linearization {
        constant_term: linearization.constant_term,
        index_terms: {
            // Make the verifier index deterministic
            linearization
                .index_terms
                .sort_by_key(|&(columns, _)| columns);
            linearization.index_terms
        },
    };
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.linearization.sort.complete");

    // <https://github.com/o1-labs/proof-systems/blob/2702b09063c7a48131173d78b6cf9408674fd67e/kimchi/src/verifier_index.rs#L310-L314>
    let srs = {
        #[cfg(target_family = "wasm")]
        trace_stage(kind, "make_verifier_index.srs.create.begin");
        let srs = SRS::create(max_poly_size);
        #[cfg(target_family = "wasm")]
        trace_stage(kind, "make_verifier_index.srs.create.complete");
        #[cfg(target_family = "wasm")]
        trace_stage(kind, "make_verifier_index.srs.lagrange.begin");
        srs.get_lagrange_basis(domain);
        #[cfg(target_family = "wasm")]
        trace_stage(kind, "make_verifier_index.srs.lagrange.complete");
        Arc::new(srs)
    };

    // <https://github.com/o1-labs/proof-systems/blob/2702b09063c7a48131173d78b6cf9408674fd67e/kimchi/src/verifier_index.rs#L319>
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.permutation.begin");
    let permutation_vanishing_polynomial_m =
        permutation_vanishing_polynomial(domain, index.zk_rows);
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.permutation.complete");

    // <https://github.com/o1-labs/proof-systems/blob/2702b09063c7a48131173d78b6cf9408674fd67e/kimchi/src/verifier_index.rs#L324>
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.zk_w.begin");
    let w = zk_w(domain, index.zk_rows);
    #[cfg(target_family = "wasm")]
    trace_stage(kind, "make_verifier_index.zk_w.complete");

    VerifierIndex::<Fq> {
        srs,
        permutation_vanishing_polynomial_m: OnceCell::from(permutation_vanishing_polynomial_m),
        w: OnceCell::from(w),
        endo,
        linearization,
        powers_of_alpha,
        ..index
    }
}

/// <https://github.com/MinaProtocol/mina/blob/bfd1009abdbee78979ff0343cc73a3480e862f58/src/lib/crypto/kimchi_bindings/stubs/src/pasta_fq_plonk_verifier_index.rs#L213>
/// <https://github.com/MinaProtocol/mina/blob/bfd1009abdbee78979ff0343cc73a3480e862f58/src/lib/pickles/common.ml#L16C1-L25C58>
pub fn make_shifts(
    domain: &Radix2EvaluationDomain<Fq>,
) -> kimchi::circuits::polynomials::permutation::Shifts<Fq> {
    // let value = 1 << log2_size;
    // let domain = Domain::<Fq>::new(value).unwrap();
    kimchi::circuits::polynomials::permutation::Shifts::new(domain)
}

// <https://github.com/MinaProtocol/mina/blob/bfd1009abdbee78979ff0343cc73a3480e862f58/src/lib/pickles/common.ml#L27>
pub fn wrap_domains(proofs_verified: usize) -> Domains {
    let h = match proofs_verified {
        0 => 13,
        1 => 14,
        2 => 15,
        _ => unreachable!(),
    };

    Domains {
        h: Domain::Pow2RootsOfUnity(h),
    }
}

/// <https://github.com/MinaProtocol/mina/blob/bfd1009abdbee78979ff0343cc73a3480e862f58/src/lib/pickles/side_loaded_verification_key.ml#L206>
pub fn make_zkapp_verifier_index(vk: &VerificationKey) -> VerifierIndex<Fq> {
    let d = wrap_domains(vk.actual_wrap_domain_size.to_int());
    let log2_size = d.h.log2_size();

    let public = 40; // Is that constant ?

    let domain: Radix2EvaluationDomain<Fq> =
        Radix2EvaluationDomain::new(1 << log2_size as u64).unwrap();

    let srs = {
        let degree = 1 << BACKEND_TOCK_ROUNDS_N;
        let srs = SRS::<Pallas>::create(degree);
        srs.get_lagrange_basis(domain);
        srs
    };

    let make_poly = |poly: &InnerCurve<Fp>| poly_commitment::PolyComm {
        chunks: vec![poly.to_affine()],
    };

    let feature_flags = FeatureFlags {
        range_check0: false,
        range_check1: false,
        foreign_field_add: false,
        foreign_field_mul: false,
        rot: false,
        xor: false,
        lookup_features: LookupFeatures {
            patterns: LookupPatterns {
                xor: false,
                lookup: false,
                range_check: false,
                foreign_field_mul: false,
            },
            joint_lookup_used: false,
            uses_runtime_tables: false,
        },
    };

    let (endo_q, _endo_r) = endos::<Fq>();
    let (linearization, powers_of_alpha) = expr_linearization(Some(&feature_flags), true);

    let shift = make_shifts(&domain);

    // <https://github.com/MinaProtocol/mina/blob/047375688f93546d4bdd58c75674394e3faae1f4/src/lib/pickles/side_loaded_verification_key.ml#L232>
    let zk_rows = 3;

    // Note: Verifier index is converted from OCaml here:
    // <https://github.com/MinaProtocol/mina/blob/bfd1009abdbee78979ff0343cc73a3480e862f58/src/lib/crypto/kimchi_bindings/stubs/src/pasta_fq_plonk_verifier_index.rs#L58>

    VerifierIndex::<Fq> {
        domain,
        max_poly_size: 1 << BACKEND_TOCK_ROUNDS_N,
        srs: Arc::new(srs),
        public,
        prev_challenges: 2,
        sigma_comm: vk.wrap_index.sigma.each_ref().map(make_poly),
        coefficients_comm: vk.wrap_index.coefficients.each_ref().map(make_poly),
        generic_comm: make_poly(&vk.wrap_index.generic),
        psm_comm: make_poly(&vk.wrap_index.psm),
        complete_add_comm: make_poly(&vk.wrap_index.complete_add),
        mul_comm: make_poly(&vk.wrap_index.mul),
        emul_comm: make_poly(&vk.wrap_index.emul),
        endomul_scalar_comm: make_poly(&vk.wrap_index.endomul_scalar),
        range_check0_comm: None,
        range_check1_comm: None,
        foreign_field_add_comm: None,
        foreign_field_mul_comm: None,
        xor_comm: None,
        rot_comm: None,
        shift: *shift.shifts(),
        permutation_vanishing_polynomial_m: OnceCell::with_value(permutation_vanishing_polynomial(
            domain, zk_rows,
        )),
        w: { OnceCell::with_value(zk_w(domain, zk_rows)) },
        endo: endo_q,
        lookup_index: None,
        linearization,
        powers_of_alpha,
        zk_rows,
    }
}
