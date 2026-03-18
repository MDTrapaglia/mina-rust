use std::{
    cell::Cell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use ark_ec::{short_weierstrass::Affine, AffineRepr, CurveConfig};
use ark_poly::{univariate::DensePolynomial, Radix2EvaluationDomain};
use kimchi::{
    alphas::Alphas,
    circuits::{
        argument::{Argument, ArgumentType},
        berkeley_columns::{BerkeleyChallengeTerm, Column},
        expr::{ConstantTerm, Linearization, PolishToken},
        gate::GateType,
        polynomials::{permutation, varbasemul::VarbaseMul},
        wires::{COLUMNS, PERMUTS},
    },
    mina_curves::pasta::Pallas,
    verifier_index::LookupVerifierIndex,
};
use mina_curves::pasta::Fq;
use mina_p2p_messages::bigint::{BigInt, InvalidBigInt};
use once_cell::sync::OnceCell;
use poly_commitment::{
    commitment::CommitmentCurve, hash_map_cache::HashMapCache, ipa::SRS, PolyComm,
};
use serde::{Deserialize, Serialize};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

use super::VerifierIndex;

#[cfg(target_family = "wasm")]
#[wasm_bindgen(inline_js = r#"
export function codex_trace_caching(stage) {
  const url = '/wasm-smoke/trace?stage=' + encodeURIComponent(stage);
  self.fetch(url, { cache: 'no-store' }).catch(() => undefined);
}"#)]
extern "C" {
    fn codex_trace_caching(stage: &str);
}

#[cfg(target_family = "wasm")]
fn trace_stage(stage: &str) {
    let stage = format!("ledger-caching:{stage}");
    codex_trace_caching(&stage);
}

fn into<'a, U, T>(slice: &'a [U]) -> Vec<T>
where
    T: From<&'a U>,
{
    slice.iter().map(T::from).collect()
}

fn try_into<'a, U, T>(slice: &'a [U]) -> Result<Vec<T>, InvalidBigInt>
where
    T: TryFrom<&'a U, Error = InvalidBigInt>,
{
    slice.iter().map(T::try_from).collect()
}

// Make it works with other containers, and non-From types
fn into_with<U, T, F, C, R>(container: C, fun: F) -> R
where
    F: Fn(U) -> T,
    C: IntoIterator<Item = U>,
    R: std::iter::FromIterator<T>,
{
    container.into_iter().map(fun).collect()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Radix2EvaluationDomainCached {
    size: u64,
    log_size_of_group: u32,
    size_as_field_element: BigInt,
    size_inv: BigInt,
    group_gen: BigInt,
    group_gen_inv: BigInt,
    offset: BigInt,
    offset_inv: BigInt,
    offset_pow_size: BigInt,
}

impl From<&Radix2EvaluationDomainCached> for Radix2EvaluationDomain<Fq> {
    fn from(domain: &Radix2EvaluationDomainCached) -> Self {
        Self {
            size: domain.size,
            log_size_of_group: domain.log_size_of_group,
            size_as_field_element: domain.size_as_field_element.to_field().unwrap(), // We trust cached data
            size_inv: domain.size_inv.to_field().unwrap(), // We trust cached data
            group_gen: domain.group_gen.to_field().unwrap(), // We trust cached data
            group_gen_inv: domain.group_gen_inv.to_field().unwrap(), // We trust cached data
            offset: domain.offset.to_field().unwrap(),
            offset_inv: domain.offset_inv.to_field().unwrap(),
            offset_pow_size: domain.offset_pow_size.to_field().unwrap(),
        }
    }
}

impl From<&Radix2EvaluationDomain<Fq>> for Radix2EvaluationDomainCached {
    fn from(domain: &Radix2EvaluationDomain<Fq>) -> Self {
        Self {
            size: domain.size,
            log_size_of_group: domain.log_size_of_group,
            size_as_field_element: domain.size_as_field_element.into(),
            size_inv: domain.size_inv.into(),
            group_gen: domain.group_gen.into(),
            group_gen_inv: domain.group_gen_inv.into(),
            offset: domain.offset.into(),
            offset_inv: domain.offset_inv.into(),
            offset_pow_size: domain.offset_pow_size.into(),
        }
    }
}

// Note: This should be an enum but bincode encode the discriminant in 8 bytes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAffineCached {
    x: BigInt,
    y: BigInt,
    infinity: bool,
}

impl<'a, T> From<&'a Affine<T>> for GroupAffineCached
where
    T: ark_ec::short_weierstrass::SWCurveConfig,
    BigInt: From<&'a <T as CurveConfig>::BaseField>,
{
    fn from(pallas: &'a Affine<T>) -> Self {
        Self {
            x: (&pallas.x).into(),
            y: (&pallas.y).into(),
            infinity: pallas.infinity,
        }
    }
}

impl<T> From<&GroupAffineCached> for ark_ec::models::short_weierstrass::Affine<T>
where
    T: ark_ec::short_weierstrass::SWCurveConfig,
    <T as CurveConfig>::BaseField: From<ark_ff::BigInteger256>,
{
    // This is copy of old `GroupAffine::new` function
    fn from(pallas: &GroupAffineCached) -> Self {
        let point = Self {
            x: pallas.x.to_field().unwrap(), // We trust cached data
            y: pallas.y.to_field().unwrap(), // We trust cached data
            infinity: pallas.infinity,
        };
        assert!(point.is_on_curve());
        assert!(point.is_in_correct_subgroup_assuming_on_curve());
        point
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PolyCommCached {
    elems: Vec<GroupAffineCached>,
}

impl<'a, A> From<&'a PolyComm<A>> for PolyCommCached
where
    GroupAffineCached: From<&'a A>,
{
    fn from(value: &'a PolyComm<A>) -> Self {
        let PolyComm { chunks } = value;

        Self {
            elems: into(chunks),
        }
    }
}

impl<'a, A> From<&'a PolyCommCached> for PolyComm<A>
where
    A: From<&'a GroupAffineCached>,
{
    fn from(value: &'a PolyCommCached) -> Self {
        let PolyCommCached { elems } = value;

        Self {
            chunks: into(elems),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SRSCached {
    g: Vec<GroupAffineCached>,
    h: GroupAffineCached,
    lagrange_bases: HashMap<usize, Vec<PolyCommCached>>,
}
impl<'a, G> From<&'a SRS<G>> for SRSCached
where
    G: CommitmentCurve,
    GroupAffineCached: for<'b> From<&'b G>,
    PolyCommCached: for<'x> From<&'x PolyComm<G>>,
    BigInt: From<&'a <G as AffineRepr>::ScalarField>,
    BigInt: From<&'a <G as AffineRepr>::BaseField>,
{
    fn from(srs: &'a SRS<G>) -> Self {
        Self {
            g: into(&srs.g),
            h: (&srs.h).into(),
            lagrange_bases: {
                let cloned = srs.lagrange_bases.clone();
                let map = HashMap::from(cloned);
                map.into_iter()
                    .map(|(key, value)| {
                        (
                            key,
                            value
                                .into_iter()
                                .map(|pc| PolyCommCached::from(&pc))
                                .collect(),
                        )
                    })
                    .collect()
            },
        }
    }
}

impl<'a, G> From<&'a SRSCached> for SRS<G>
where
    G: CommitmentCurve + From<&'a GroupAffineCached>,
{
    fn from(srs: &'a SRSCached) -> Self {
        Self {
            g: into(&srs.g),
            h: (&srs.h).into(),
            lagrange_bases: {
                let lagrange_bases = srs
                    .lagrange_bases
                    .iter()
                    .map(|(key, value)| (*key, value.iter().map(PolyComm::from).collect()))
                    .collect();

                HashMapCache::new_from_hashmap(lagrange_bases)
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DensePolynomialCached {
    coeffs: Vec<BigInt>, // Fq
}

impl From<&DensePolynomialCached> for DensePolynomial<Fq> {
    fn from(value: &DensePolynomialCached) -> Self {
        Self {
            coeffs: try_into(&value.coeffs).unwrap(), // We trust cached data
        }
    }
}

impl From<&DensePolynomial<Fq>> for DensePolynomialCached {
    fn from(value: &DensePolynomial<Fq>) -> Self {
        Self {
            coeffs: into(&value.coeffs),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerifierIndexCached {
    domain: Radix2EvaluationDomainCached,
    max_poly_size: usize,
    srs: SRSCached,
    public: usize,
    prev_challenges: usize,
    sigma_comm: [PolyComm<Pallas>; PERMUTS],
    coefficients_comm: [PolyComm<Pallas>; COLUMNS],
    generic_comm: PolyComm<Pallas>,
    psm_comm: PolyComm<Pallas>,
    complete_add_comm: PolyComm<Pallas>,
    mul_comm: PolyComm<Pallas>,
    emul_comm: PolyComm<Pallas>,
    endomul_scalar_comm: PolyComm<Pallas>,
    range_check0_comm: Option<PolyComm<Pallas>>,
    range_check1_comm: Option<PolyComm<Pallas>>,
    foreign_field_add_comm: Option<PolyComm<Pallas>>,
    foreign_field_mul_comm: Option<PolyComm<Pallas>>,
    xor_comm: Option<PolyComm<Pallas>>,
    rot_comm: Option<PolyComm<Pallas>>,
    shift: [BigInt; PERMUTS], // Fq
    permutation_vanishing_polynomial_m: DensePolynomialCached,
    w: BigInt,    // Fq
    endo: BigInt, // Fq
    lookup_index: Option<LookupVerifierIndex<Pallas>>,
    linearization: Linearization<Vec<PolishToken<BigInt, Column, BerkeleyChallengeTerm>>, Column>, // Fq
    zk_rows: u64,
}

type LinearizationCached =
    Linearization<Vec<PolishToken<BigInt, Column, BerkeleyChallengeTerm>>, Column>;

struct CountingSlice<'de> {
    inner: postcard::de_flavors::Slice<'de>,
    bytes_read: Rc<Cell<usize>>,
}

impl<'de> CountingSlice<'de> {
    fn new(bytes: &'de [u8], bytes_read: Rc<Cell<usize>>) -> Self {
        Self {
            inner: postcard::de_flavors::Slice::new(bytes),
            bytes_read,
        }
    }
}

impl<'de> postcard::de_flavors::Flavor<'de> for CountingSlice<'de> {
    type Remainder = &'de [u8];
    type Source = &'de [u8];

    fn pop(&mut self) -> Result<u8, postcard::Error> {
        let byte = self.inner.pop()?;
        self.bytes_read.set(self.bytes_read.get() + 1);
        Ok(byte)
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }

    fn try_take_n(&mut self, ct: usize) -> Result<&'de [u8], postcard::Error> {
        let bytes = self.inner.try_take_n(ct)?;
        self.bytes_read.set(self.bytes_read.get() + bytes.len());
        Ok(bytes)
    }

    fn finalize(self) -> Result<&'de [u8], postcard::Error> {
        self.inner.finalize()
    }
}

fn trace_offset(stage: &str, bytes_read: &Cell<usize>) {
    #[cfg(target_family = "wasm")]
    trace_stage(&format!(
        "verifier_index_from_bytes.postcard.{stage}.offset.{}",
        bytes_read.get()
    ));
}

fn deserialize_next<'de, T, F>(
    deserializer: &mut postcard::Deserializer<'de, F>,
    bytes_read: &Cell<usize>,
    stage: &str,
) -> Result<T, postcard::Error>
where
    T: Deserialize<'de>,
    F: postcard::de_flavors::Flavor<'de>,
{
    #[cfg(target_family = "wasm")]
    trace_stage(&format!("verifier_index_from_bytes.postcard.{stage}.begin"));
    let value = T::deserialize(deserializer)?;
    trace_offset(stage, bytes_read);
    #[cfg(target_family = "wasm")]
    trace_stage(&format!("verifier_index_from_bytes.postcard.{stage}.complete"));
    Ok(value)
}

fn deserialize_usize_next<'de, F>(
    deserializer: &mut postcard::Deserializer<'de, F>,
    bytes_read: &Cell<usize>,
    stage: &str,
) -> Result<usize, postcard::Error>
where
    F: postcard::de_flavors::Flavor<'de>,
{
    let value = deserialize_next::<usize, F>(deserializer, bytes_read, stage)?;
    #[cfg(target_family = "wasm")]
    trace_stage(&format!(
        "verifier_index_from_bytes.postcard.{stage}.value.{value}"
    ));
    Ok(value)
}

fn deserialize_array_next<'de, T, F, const N: usize>(
    deserializer: &mut postcard::Deserializer<'de, F>,
    bytes_read: &Cell<usize>,
    stage: &str,
) -> Result<[T; N], postcard::Error>
where
    T: Deserialize<'de>,
    F: postcard::de_flavors::Flavor<'de>,
{
    #[cfg(target_family = "wasm")]
    trace_stage(&format!("verifier_index_from_bytes.postcard.{stage}.begin"));

    let mut values = Vec::with_capacity(N);
    for index in 0..N {
        #[cfg(target_family = "wasm")]
        trace_stage(&format!(
            "verifier_index_from_bytes.postcard.{stage}.{index}.begin"
        ));
        let value = T::deserialize(&mut *deserializer)?;
        #[cfg(target_family = "wasm")]
        {
            trace_stage(&format!(
                "verifier_index_from_bytes.postcard.{stage}.{index}.offset.{}",
                bytes_read.get()
            ));
            trace_stage(&format!(
                "verifier_index_from_bytes.postcard.{stage}.{index}.complete"
            ));
        }
        values.push(value);
    }

    trace_offset(stage, bytes_read);
    #[cfg(target_family = "wasm")]
    trace_stage(&format!("verifier_index_from_bytes.postcard.{stage}.complete"));

    Ok(values.try_into().unwrap_or_else(|_| unreachable!()))
}

fn verifier_index_cached_from_bytes(bytes: &[u8]) -> Result<VerifierIndexCached, postcard::Error> {
    let bytes_read = Rc::new(Cell::new(0usize));
    let flavor = CountingSlice::new(bytes, Rc::clone(&bytes_read));
    let mut deserializer = postcard::Deserializer::from_flavor(flavor);

    let domain = deserialize_next::<Radix2EvaluationDomainCached, _>(
        &mut deserializer,
        bytes_read.as_ref(),
        "domain",
    )?;
    let max_poly_size = deserialize_usize_next(&mut deserializer, bytes_read.as_ref(), "max_poly_size")?;
    let srs = deserialize_next::<SRSCached, _>(&mut deserializer, bytes_read.as_ref(), "srs")?;
    let public = deserialize_usize_next(&mut deserializer, bytes_read.as_ref(), "public")?;
    let prev_challenges =
        deserialize_usize_next(&mut deserializer, bytes_read.as_ref(), "prev_challenges")?;
    let sigma_comm = deserialize_array_next::<PolyComm<Pallas>, _, PERMUTS>(
        &mut deserializer,
        bytes_read.as_ref(),
        "sigma_comm",
    )?;
    let coefficients_comm =
        deserialize_next::<[PolyComm<Pallas>; COLUMNS], _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "coefficients_comm",
        )?;
    let generic_comm =
        deserialize_next::<PolyComm<Pallas>, _>(&mut deserializer, bytes_read.as_ref(), "generic_comm")?;
    let psm_comm =
        deserialize_next::<PolyComm<Pallas>, _>(&mut deserializer, bytes_read.as_ref(), "psm_comm")?;
    let complete_add_comm =
        deserialize_next::<PolyComm<Pallas>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "complete_add_comm",
        )?;
    let mul_comm =
        deserialize_next::<PolyComm<Pallas>, _>(&mut deserializer, bytes_read.as_ref(), "mul_comm")?;
    let emul_comm =
        deserialize_next::<PolyComm<Pallas>, _>(&mut deserializer, bytes_read.as_ref(), "emul_comm")?;
    let endomul_scalar_comm =
        deserialize_next::<PolyComm<Pallas>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "endomul_scalar_comm",
        )?;
    let range_check0_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "range_check0_comm",
        )?;
    let range_check1_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "range_check1_comm",
        )?;
    let foreign_field_add_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "foreign_field_add_comm",
        )?;
    let foreign_field_mul_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "foreign_field_mul_comm",
        )?;
    let xor_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(&mut deserializer, bytes_read.as_ref(), "xor_comm")?;
    let rot_comm =
        deserialize_next::<Option<PolyComm<Pallas>>, _>(&mut deserializer, bytes_read.as_ref(), "rot_comm")?;
    let shift =
        deserialize_next::<[BigInt; PERMUTS], _>(&mut deserializer, bytes_read.as_ref(), "shift")?;
    let permutation_vanishing_polynomial_m =
        deserialize_next::<DensePolynomialCached, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "permutation_vanishing_polynomial_m",
        )?;
    let w = deserialize_next::<BigInt, _>(&mut deserializer, bytes_read.as_ref(), "w")?;
    let endo = deserialize_next::<BigInt, _>(&mut deserializer, bytes_read.as_ref(), "endo")?;
    let lookup_index =
        deserialize_next::<Option<LookupVerifierIndex<Pallas>>, _>(
            &mut deserializer,
            bytes_read.as_ref(),
            "lookup_index",
        )?;
    let linearization =
        deserialize_next::<LinearizationCached, _>(&mut deserializer, bytes_read.as_ref(), "linearization")?;
    let zk_rows = deserialize_next::<u64, _>(&mut deserializer, bytes_read.as_ref(), "zk_rows")?;

    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.postcard.finalize.begin");
    let _ = deserializer.finalize()?;
    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.postcard.finalize.complete");

    Ok(VerifierIndexCached {
        domain,
        max_poly_size,
        srs,
        public,
        prev_challenges,
        sigma_comm,
        coefficients_comm,
        generic_comm,
        psm_comm,
        complete_add_comm,
        mul_comm,
        emul_comm,
        endomul_scalar_comm,
        range_check0_comm,
        range_check1_comm,
        foreign_field_add_comm,
        foreign_field_mul_comm,
        xor_comm,
        rot_comm,
        shift,
        permutation_vanishing_polynomial_m,
        w,
        endo,
        lookup_index,
        linearization,
        zk_rows,
    })
}

fn conv_token<'a, T, U, F>(
    token: &'a PolishToken<T, Column, BerkeleyChallengeTerm>,
    fun: F,
) -> PolishToken<U, Column, BerkeleyChallengeTerm>
where
    T: 'a,
    F: Fn(&T) -> U,
{
    match token {
        PolishToken::Constant(constant_term) => match constant_term {
            ConstantTerm::EndoCoefficient => PolishToken::Constant(ConstantTerm::EndoCoefficient),
            &ConstantTerm::Mds { row, col } => {
                PolishToken::Constant(ConstantTerm::Mds { row, col })
            }
            ConstantTerm::Literal(literal) => {
                PolishToken::Constant(ConstantTerm::Literal(fun(literal)))
            }
        },
        PolishToken::Challenge(challenge) => PolishToken::Challenge(*challenge),
        PolishToken::Cell(variable) => PolishToken::Cell(*variable),
        PolishToken::Dup => PolishToken::Dup,
        PolishToken::Pow(p) => PolishToken::Pow(*p),
        PolishToken::Add => PolishToken::Add,
        PolishToken::Mul => PolishToken::Mul,
        PolishToken::Sub => PolishToken::Sub,
        PolishToken::VanishesOnZeroKnowledgeAndPreviousRows => {
            PolishToken::VanishesOnZeroKnowledgeAndPreviousRows
        }
        PolishToken::UnnormalizedLagrangeBasis(row_offset) => {
            PolishToken::UnnormalizedLagrangeBasis(*row_offset)
        }
        PolishToken::Store => PolishToken::Store,
        PolishToken::Load(load) => PolishToken::Load(*load),
        PolishToken::SkipIf(feature_flag, value) => PolishToken::SkipIf(*feature_flag, *value),
        PolishToken::SkipIfNot(feature_flag, value) => {
            PolishToken::SkipIfNot(*feature_flag, *value)
        }
    }
}

fn conv_linearization<'a, T, U, F>(
    linearization: &'a Linearization<Vec<PolishToken<T, Column, BerkeleyChallengeTerm>>, Column>,
    fun: F,
) -> Linearization<Vec<PolishToken<U, Column, BerkeleyChallengeTerm>>, Column>
where
    T: 'a,
    F: Fn(&T) -> U,
{
    let constant_term = &linearization.constant_term;
    let index_terms = &linearization.index_terms;

    let conv_token =
        |token: &PolishToken<T, Column, BerkeleyChallengeTerm>| conv_token(token, &fun);

    Linearization {
        constant_term: into_with(constant_term, conv_token),
        index_terms: into_with(index_terms, |(col, term)| {
            (*col, into_with(term, conv_token))
        }),
    }
}

impl From<&VerifierIndex<Fq>> for VerifierIndexCached {
    fn from(v: &VerifierIndex<Fq>) -> Self {
        let VerifierIndex::<Fq> {
            domain,
            max_poly_size,
            srs,
            public,
            prev_challenges,
            sigma_comm,
            coefficients_comm,
            generic_comm,
            psm_comm,
            complete_add_comm,
            mul_comm,
            emul_comm,
            endomul_scalar_comm,
            range_check0_comm,
            range_check1_comm,
            foreign_field_add_comm,
            foreign_field_mul_comm,
            xor_comm,
            rot_comm,
            shift,
            w,
            endo,
            lookup_index,
            linearization,
            zk_rows,
            permutation_vanishing_polynomial_m,
            powers_of_alpha: _, // ignored
        } = v;

        Self {
            domain: domain.into(),
            max_poly_size: *max_poly_size,
            srs: {
                let s = srs.as_ref();
                SRSCached::from(s)
            },
            public: *public,
            prev_challenges: *prev_challenges,
            sigma_comm: sigma_comm.clone(),
            coefficients_comm: coefficients_comm.clone(),
            generic_comm: generic_comm.clone(),
            psm_comm: psm_comm.clone(),
            complete_add_comm: complete_add_comm.clone(),
            mul_comm: mul_comm.clone(),
            emul_comm: emul_comm.clone(),
            endomul_scalar_comm: endomul_scalar_comm.clone(),
            range_check0_comm: range_check0_comm.clone(),
            range_check1_comm: range_check1_comm.clone(),
            foreign_field_add_comm: foreign_field_add_comm.clone(),
            foreign_field_mul_comm: foreign_field_mul_comm.clone(),
            xor_comm: xor_comm.clone(),
            rot_comm: rot_comm.clone(),
            shift: shift.each_ref().map(|s| s.into()),
            permutation_vanishing_polynomial_m: permutation_vanishing_polynomial_m
                .get()
                .unwrap()
                .into(),
            w: (*w.get().unwrap()).into(),
            endo: endo.into(),
            lookup_index: lookup_index.clone(),
            linearization: conv_linearization(linearization, |v| v.into()),
            zk_rows: *zk_rows,
        }
    }
}

impl From<&VerifierIndexCached> for VerifierIndex<Fq> {
    fn from(v: &VerifierIndexCached) -> Self {
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.enter");

        let VerifierIndexCached {
            domain,
            max_poly_size,
            srs,
            public,
            prev_challenges,
            sigma_comm,
            coefficients_comm,
            generic_comm,
            psm_comm,
            complete_add_comm,
            mul_comm,
            emul_comm,
            endomul_scalar_comm,
            range_check0_comm,
            range_check1_comm,
            foreign_field_add_comm,
            foreign_field_mul_comm,
            xor_comm,
            rot_comm,
            shift,
            permutation_vanishing_polynomial_m,
            w,
            endo,
            lookup_index,
            linearization,
            zk_rows,
        } = v;

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.domain.begin");
        let domain = domain.into();
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.domain.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.srs.begin");
        let srs = {
            let s: SRS<_> = SRS::from(srs);
            Arc::new(s)
        };
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.srs.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.commitments.begin");
        let sigma_comm = sigma_comm.clone();
        let coefficients_comm = coefficients_comm.clone();
        let generic_comm = generic_comm.clone();
        let psm_comm = psm_comm.clone();
        let complete_add_comm = complete_add_comm.clone();
        let mul_comm = mul_comm.clone();
        let emul_comm = emul_comm.clone();
        let endomul_scalar_comm = endomul_scalar_comm.clone();
        let range_check0_comm = range_check0_comm.clone();
        let range_check1_comm = range_check1_comm.clone();
        let foreign_field_add_comm = foreign_field_add_comm.clone();
        let foreign_field_mul_comm = foreign_field_mul_comm.clone();
        let xor_comm = xor_comm.clone();
        let rot_comm = rot_comm.clone();
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.commitments.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.shift.begin");
        let shift = shift.each_ref().map(|s: &BigInt| s.to_field().unwrap());
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.shift.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.permutation_poly.begin");
        let permutation_vanishing_polynomial_m =
            OnceCell::with_value(permutation_vanishing_polynomial_m.into());
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.permutation_poly.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.scalars.begin");
        let w = OnceCell::with_value(w.to_field().unwrap());
        let endo = endo.to_field().unwrap();
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.scalars.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.linearization.begin");
        let linearization = conv_linearization(linearization, |v: &BigInt| v.to_field().unwrap());
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.linearization.complete");

        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.powers_of_alpha.begin");
        let powers_of_alpha = {
            let mut powers_of_alpha = Alphas::<Fq>::default();
            powers_of_alpha.register(
                ArgumentType::Gate(GateType::Zero),
                VarbaseMul::<Fq>::CONSTRAINTS,
            );
            powers_of_alpha.register(ArgumentType::Permutation, permutation::CONSTRAINTS);
            powers_of_alpha
        };
        #[cfg(target_family = "wasm")]
        trace_stage("verifier_index_from_cached.powers_of_alpha.complete");

        Self {
            domain,
            max_poly_size: *max_poly_size,
            srs,
            public: *public,
            prev_challenges: *prev_challenges,
            sigma_comm,
            coefficients_comm,
            generic_comm,
            psm_comm,
            complete_add_comm,
            mul_comm,
            emul_comm,
            endomul_scalar_comm,
            foreign_field_add_comm,
            xor_comm,
            shift, // We trust cached data
            permutation_vanishing_polynomial_m,
            w,    // We trust cached data
            endo, // We trust cached data
            lookup_index: lookup_index.clone(),
            linearization,
            powers_of_alpha,
            range_check0_comm,
            range_check1_comm,
            foreign_field_mul_comm,
            rot_comm,
            zk_rows: *zk_rows,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Error writing verifier index to bytes: {0}")]
pub struct VerifierIndexToBytesError(#[from] postcard::Error);

pub fn verifier_index_to_bytes(
    verifier: &VerifierIndex<Fq>,
) -> Result<Vec<u8>, VerifierIndexToBytesError> {
    let verifier: VerifierIndexCached = verifier.into();
    Ok(postcard::to_stdvec(&verifier)?)
}

#[derive(Debug, thiserror::Error)]
#[error("Error reading verifier index from bytes: {0}")]
pub struct VerifierIndexFromBytesError(#[from] postcard::Error);

pub fn verifier_index_from_bytes(
    bytes: &[u8],
) -> Result<VerifierIndex<Fq>, VerifierIndexFromBytesError> {
    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.postcard.begin");
    let verifier = verifier_index_cached_from_bytes(bytes)?;
    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.postcard.complete");
    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.into.begin");
    let verifier = VerifierIndex::<Fq>::from(&verifier);
    #[cfg(target_family = "wasm")]
    trace_stage("verifier_index_from_bytes.into.complete");
    Ok(verifier)
}

pub fn srs_to_bytes<'a, G>(srs: &'a SRS<G>) -> Vec<u8>
where
    G: CommitmentCurve,
    GroupAffineCached: for<'y> From<&'y G>,
    BigInt: From<&'a <G as AffineRepr>::ScalarField>,
    BigInt: From<&'a <G as AffineRepr>::BaseField>,
{
    let srs: SRSCached = srs.into();

    postcard::to_stdvec(&srs).unwrap()
}

pub fn srs_from_bytes<G>(bytes: &[u8]) -> SRS<G>
where
    G: CommitmentCurve,
    G: for<'a> From<&'a GroupAffineCached>,
{
    let srs: SRSCached = postcard::from_bytes(bytes).unwrap();
    (&srs).into()
}

pub fn mina_cache_path<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache/mina").join(path))
}

pub fn ensure_path_exists<P: AsRef<Path> + Clone>(path: P) -> Result<(), std::io::Error> {
    match std::fs::metadata(path.clone()) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Path exists but is not a directory",
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(path)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
