use js_sys::Promise;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r#"
export function codex_runtime_trace(stage, details) {
  const url = new URL("/wasm-smoke/trace", self.location.origin);
  url.searchParams.set("stage", `runtime-repro:${stage}`);
  if (details !== undefined && details !== null && details !== "") {
    url.searchParams.set("details", String(details));
  }
  self.fetch(url, { cache: "no-store" }).catch(() => undefined);
}

export function codex_runtime_yield() {
  return new Promise(resolve => setTimeout(resolve, 0));
}
"#)]
extern "C" {
    fn codex_runtime_trace(stage: &str, details: &str);
    fn codex_runtime_yield() -> Promise;
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn sha256_one_shot_js_input(bytes: Vec<u8>) -> Vec<u8> {
    trace_bytes("wasm.sha256.one_shot_js_input.begin", bytes.len());
    let digest = Sha256::digest(bytes);
    trace_bytes("wasm.sha256.one_shot_js_input.complete", digest.len());
    digest.to_vec()
}

#[wasm_bindgen]
pub fn sha256_chunked_js_input(bytes: Vec<u8>, chunk_size: usize, progress_step: usize) -> Vec<u8> {
    hash_chunked(
        "wasm.sha256.chunked_js_input",
        &bytes,
        sanitize_chunk_size(chunk_size),
        sanitize_progress_step(progress_step, bytes.len()),
    )
}

#[wasm_bindgen]
pub async fn sha256_chunked_yielding_js_input(
    bytes: Vec<u8>,
    chunk_size: usize,
    yield_every_chunks: usize,
    progress_step: usize,
) -> Result<Vec<u8>, JsValue> {
    hash_chunked_yielding(
        "wasm.sha256.chunked_yielding_js_input",
        &bytes,
        sanitize_chunk_size(chunk_size),
        yield_every_chunks,
        sanitize_progress_step(progress_step, bytes.len()),
    )
    .await
}

#[wasm_bindgen]
pub fn sha256_one_shot_generated(len: usize) -> Vec<u8> {
    trace_bytes("wasm.sha256.one_shot_generated.generate.begin", len);
    let bytes = pattern_bytes(len);
    trace_bytes("wasm.sha256.one_shot_generated.generate.complete", bytes.len());
    sha256_one_shot_generated_impl(bytes)
}

#[wasm_bindgen]
pub fn sha256_chunked_generated(len: usize, chunk_size: usize, progress_step: usize) -> Vec<u8> {
    trace_bytes("wasm.sha256.chunked_generated.generate.begin", len);
    let bytes = pattern_bytes(len);
    trace_bytes("wasm.sha256.chunked_generated.generate.complete", bytes.len());
    hash_chunked(
        "wasm.sha256.chunked_generated",
        &bytes,
        sanitize_chunk_size(chunk_size),
        sanitize_progress_step(progress_step, bytes.len()),
    )
}

#[wasm_bindgen]
pub async fn sha256_chunked_yielding_generated(
    len: usize,
    chunk_size: usize,
    yield_every_chunks: usize,
    progress_step: usize,
) -> Result<Vec<u8>, JsValue> {
    trace_bytes("wasm.sha256.chunked_yielding_generated.generate.begin", len);
    let bytes = pattern_bytes(len);
    trace_bytes(
        "wasm.sha256.chunked_yielding_generated.generate.complete",
        bytes.len(),
    );
    hash_chunked_yielding(
        "wasm.sha256.chunked_yielding_generated",
        &bytes,
        sanitize_chunk_size(chunk_size),
        yield_every_chunks,
        sanitize_progress_step(progress_step, bytes.len()),
    )
    .await
}

fn sha256_one_shot_generated_impl(bytes: Vec<u8>) -> Vec<u8> {
    trace_bytes("wasm.sha256.one_shot_generated.begin", bytes.len());
    let digest = Sha256::digest(bytes);
    trace_bytes("wasm.sha256.one_shot_generated.complete", digest.len());
    digest.to_vec()
}

fn hash_chunked(label: &str, bytes: &[u8], chunk_size: usize, progress_step: usize) -> Vec<u8> {
    trace_hash_begin(label, bytes.len(), chunk_size, progress_step);

    let mut progress = ProgressReporter::new(label, bytes.len(), progress_step);
    let mut hasher = Sha256::new();
    let mut processed = 0usize;

    for chunk in bytes.chunks(chunk_size) {
        hasher.update(chunk);
        processed += chunk.len();
        progress.observe(processed);
    }

    trace_stage(&format!("{label}.hash.complete"));
    let digest = hasher.finalize();
    trace_bytes(&format!("{label}.digest.complete"), digest.len());
    digest.to_vec()
}

async fn hash_chunked_yielding(
    label: &str,
    bytes: &[u8],
    chunk_size: usize,
    yield_every_chunks: usize,
    progress_step: usize,
) -> Result<Vec<u8>, JsValue> {
    trace_hash_begin(label, bytes.len(), chunk_size, progress_step);
    trace_value(
        &format!("{label}.yield_every_chunks"),
        yield_every_chunks.max(1),
    );

    let mut progress = ProgressReporter::new(label, bytes.len(), progress_step);
    let mut hasher = Sha256::new();
    let mut processed = 0usize;
    let effective_yield_every_chunks = yield_every_chunks.max(1);

    for (index, chunk) in bytes.chunks(chunk_size).enumerate() {
        hasher.update(chunk);
        processed += chunk.len();
        progress.observe(processed);

        if processed < bytes.len() && (index + 1) % effective_yield_every_chunks == 0 {
            JsFuture::from(codex_runtime_yield()).await?;
        }
    }

    trace_stage(&format!("{label}.hash.complete"));
    let digest = hasher.finalize();
    trace_bytes(&format!("{label}.digest.complete"), digest.len());
    Ok(digest.to_vec())
}

fn pattern_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = (index & 0xff) as u8;
    }
    bytes
}

fn sanitize_chunk_size(chunk_size: usize) -> usize {
    chunk_size.max(1)
}

fn sanitize_progress_step(progress_step: usize, total_len: usize) -> usize {
    let fallback = total_len.max(1);
    progress_step.max(1).min(fallback)
}

fn trace_stage(stage: &str) {
    codex_runtime_trace(stage, "");
}

fn trace_value(stage: &str, value: usize) {
    codex_runtime_trace(stage, &value.to_string());
}

fn trace_bytes(stage: &str, byte_len: usize) {
    trace_value(stage, byte_len);
}

fn trace_hash_begin(label: &str, total_len: usize, chunk_size: usize, progress_step: usize) {
    let details = format!(
        "len={total_len},chunk_size={chunk_size},progress_step={progress_step}"
    );
    codex_runtime_trace(&format!("{label}.begin"), &details);
}

struct ProgressReporter<'a> {
    label: &'a str,
    total_len: usize,
    progress_step: usize,
    next_boundary: usize,
    emitted_first: bool,
}

impl<'a> ProgressReporter<'a> {
    fn new(label: &'a str, total_len: usize, progress_step: usize) -> Self {
        Self {
            label,
            total_len,
            progress_step,
            next_boundary: progress_step,
            emitted_first: false,
        }
    }

    fn observe(&mut self, processed: usize) {
        if !self.emitted_first && processed > 0 {
            trace_bytes(&format!("{}.progress.first", self.label), processed);
            self.emitted_first = true;
        }

        while processed >= self.next_boundary && self.next_boundary < self.total_len {
            trace_bytes(&format!("{}.progress.boundary", self.label), self.next_boundary);
            self.next_boundary = self.next_boundary.saturating_add(self.progress_step);
            if self.next_boundary == 0 {
                break;
            }
        }

        if processed >= self.total_len {
            trace_bytes(&format!("{}.progress.final", self.label), self.total_len);
        }
    }
}
