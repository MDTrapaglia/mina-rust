# WebAssembly Runtime Repro Plan

## Scope

This plan splits the active investigation away from Mina startup logic and focuses on a narrower question:

- why large SHA-256 work items inside a browser worker on `wasm32-unknown-unknown` show unstable progress in Chromium headless
- why the observed cutoff changes with chunk size and scheduling strategy
- whether the remaining failure is inside Mina logic or in the browser/wasm runtime stack

The current conclusion is:

- no tested chunk size has eliminated the failure mode in a stable way
- `16 KiB` with cooperative yielding is the best variant observed so far
- that variant can complete the `block_verifier_index` payload digest in some runs
- the same variant still fails or stalls earlier in other equivalent runs

Because of that variability, chunk tuning alone is not an acceptable fix.

## Confirmed advances to date

The runtime line already established several facts that were not clear when this branch was created:

- the large-digest instability can reproduce before entering Mina verifier parsing
- pure JS `crypto.subtle.digest("SHA-256", ...)` inside the same worker can stall on a `Uint8Array` of roughly `3317098` bytes
- the same JS probe completes reliably at smaller sizes such as `1048576` bytes
- sizes around `2 MiB`, `2.5 MiB`, and `3 MiB` are not cleanly monotonic: some runs complete and others time out with large timing variance
- copying bytes into a fresh `Uint8Array` does not remove the issue, so shared wasm memory is not the only plausible ingredient
- running the probe with `skip_run=1` still reproduces the failure, so the extra `run(...)` worker load is not a necessary condition
- forcing lower effective concurrency, for example `forced_hardware_concurrency=2`, can improve some runs but does not make the behavior reliable
- a wasm-side Rust fallback using chunked `sha2` plus cooperative yields changes the behavior materially
- the best Mina-side variant observed so far is Rust hashing with `16 KiB` chunks and a yield boundary every `64 KiB`
- that variant can complete `block_verifier_index` payload digest verification and reach `read_cache.decode.begin` in some runs
- the same variant still fails to do so in other equivalent runs

These results narrow the problem substantially:

- this is no longer well explained as a Mina-only bug
- this is no longer well explained as a `wasm-bindgen` bridge bug alone
- this is no longer well explained as "a single bad payload offset"
- the remaining problem is strongly consistent with browser-worker runtime variability under large hashing work

## Refined hypotheses

The strongest working hypothesis is still not "a specific bad chunk offset" and not "a single broken SHA implementation".

The evidence currently fits a runtime interaction involving:

- wasm worker scheduling
- shared memory / multithreaded wasm runtime behavior
- long-running CPU work inside a worker
- Chromium headless nondeterminism under load
- browser task scheduling around large async crypto work

The leading hypotheses are now:

### H1. Worker scheduling sensitivity is primary

The outcome changes materially when long work is segmented differently:

- one-shot `WebCrypto` can stall
- Rust chunked hashing can progress further
- adding cooperative yields can help
- `16 KiB` chunks outperform larger chunks in the current Mina path

This suggests that scheduler granularity and worker responsiveness are part of the causal chain, even if they are not the whole explanation.

### H2. Shared memory is aggravating, not strictly required

The issue reproducing with a copied JS buffer means shared wasm memory is not required.

Shared memory may still worsen timing or contention once Mina runs, but it is no longer a sufficient root-cause explanation by itself.

### H3. Contention changes probability, not necessity

Lower effective concurrency sometimes improves completion time, but:

- failure still occurs with `skip_run=1`
- failure still occurs with reduced apparent concurrency

So extra workers and contention matter, but they do not appear to be necessary preconditions.

### H4. Headless Chromium runtime behavior is now a first-class suspect

Because the failure reproduces in pure JS `WebCrypto` inside the same worker environment, a browser/runtime issue is now a stronger explanation than a Mina-specific logic error.

This still needs one more step of confirmation:

- determine whether the same minimal probe behaves differently in non-headless Chromium or another browser
- determine whether the repro persists with a wasm-only harness that does not import Mina code at all

Chunk size matters because it changes the shape of the work:

- larger chunks reduce loop overhead but increase uninterrupted work per update
- smaller chunks increase per-chunk overhead and the number of updates
- yielding inserts scheduler boundaries that can help or hurt depending on timing

So chunk size is influencing runtime behavior, but no result so far proves that chunk size is the root cause.

## What is already ruled out

At the current confidence level, this branch should treat the following explanations as insufficient or already weakened:

- "the issue is only inside Mina verifier decoding"
- "the issue only happens after `run(...)` starts extra runtime work"
- "the issue requires wasm memory-backed input"
- "there is one deterministic failing chunk offset in the postcard payload"
- "switching from `sha2` to `WebCrypto` alone fixes the problem"
- "a smaller chunk size fully removes the failure mode"

## Investigation goals

1. Build a minimal browser-worker repro outside Mina verifier setup.
2. Determine whether instability reproduces with:
   - pure JS `crypto.subtle.digest`
   - pure JS chunked hashing
   - wasm-exported hashing over a large buffer
   - wasm hashing with and without cooperative yields
3. Separate these dimensions:
   - payload size
   - chunk size
   - number of yields
   - shared vs copied input buffer
   - worker count / effective concurrency
4. Establish whether the remaining variability is:
   - Mina-specific
   - wasm-bindgen bridge specific
   - Chromium worker/runtime specific

5. Produce a repro and evidence quality high enough to justify one of:
   - an upstream Chromium / wasm runtime issue
   - a product guardrail in Mina wasm
   - both

## Non-goals

- Do not continue tuning Mina startup behavior as if it were a stable product fix.
- Do not merge a chunk-size workaround as a final fix without a runtime explanation or a strong operational guardrail.
- Do not add more broad tracing inside verifier startup unless it answers a specific branch of this plan.

## Deliverables

### 1. Minimal harness inside the existing smoke app

Add a dedicated mode to the local harness that can run without Mina verifier loading:

- one worker
- one large input buffer
- controlled hashing strategy
- controlled chunk size
- controlled yield cadence
- controlled copy/shared-memory mode

The mode should emit only a small set of milestones:

- start
- first progress marker
- last progress marker
- complete
- timeout

### 2. Repro matrix

Run and record a matrix across:

- hashing backend:
  - `crypto.subtle.digest`
  - Rust `sha2` in one shot
  - Rust `sha2` chunked
  - Rust `sha2` chunked + yield
- payload size:
  - `1 MiB`
  - `2 MiB`
  - `3 MiB`
  - `3.317098 MiB`
- chunk size:
  - `8 KiB`
  - `16 KiB`
  - `32 KiB`
  - `64 KiB`
- yield cadence:
  - none
  - every `64 KiB`
  - every `256 KiB`
- concurrency:
  - single worker repro only
  - with the existing extra worker/runtime load

Status:

- partially complete
- pure JS `WebCrypto` probe exists in the local harness
- size sweep and basic concurrency sweep were already exercised
- Rust-only minimal repro outside Mina code is still missing

Each row should record:

- completed or timed out
- total elapsed time
- highest progress marker reached
- whether behavior is reproducible across at least 3 runs

### 3. Decision memo

At the end of the matrix, write a short conclusion that answers:

- can the issue be reproduced outside Mina verifier parsing
- is WebCrypto part of the problem, or only one manifestation
- does shared memory materially change the outcome
- does cooperative yielding improve stability enough to matter
- is there any parameter set that is reliable enough for product use

## Phases

### Phase 1. Minimalize the reproducer

Implement a new harness path that does not call `run(...)` and does not touch verifier cache files.

Use the existing local app under:

- `wasm-smoke/index.html`
- `wasm-smoke/mina-worker-module.js`

Goal:

- reproduce the instability with the smallest possible moving surface while staying in the same browser/worker environment

Exit criteria:

- either the failure still reproduces without Mina verifier loading
- or it disappears, in which case Mina-side logic becomes suspect again

Status:

- partially complete
- JS-only worker probe already reproduces the failure without entering verifier parsing
- a stricter repro that does not load Mina wasm at all is still pending

### Phase 2. Separate backend from scheduler

If the minimal repro still fails, compare:

- JS `WebCrypto`
- Rust `sha2`
- Rust `sha2` with chunking
- Rust `sha2` with chunking and yield

Goal:

- determine whether the instability follows the hashing backend or the worker scheduling pattern

Exit criteria:

- identify at least one pair of runs where changing only backend or yield strategy changes the outcome materially

Status:

- partially complete
- outcome already changes materially between:
  - JS `WebCrypto`
  - Rust chunked hashing
  - Rust chunked hashing plus yield
- missing piece: a minimal Rust-only wasm repro outside Mina crates

### Phase 3. Shared-memory sensitivity

Repeat the minimal repro with:

- copied `Uint8Array`
- wasm memory-backed slice
- worker-local copied buffer before hashing

Goal:

- determine whether shared memory is a necessary ingredient

Exit criteria:

- confirm or reject "shared memory is required to reproduce"

Status:

- partially complete
- copied JS buffer still reproduces the issue
- this is strong evidence against shared memory being required
- still missing: a minimal wasm-side copy-vs-shared comparison outside Mina

### Phase 4. Concurrency sensitivity

Run the same minimal repro:

- alone in one worker
- with extra workers alive
- with forced lower effective concurrency when possible

Goal:

- quantify whether contention is causal or merely aggravating

Exit criteria:

- confirm whether single-worker repro is sufficient

Status:

- partially complete
- `skip_run=1` indicates the extra Mina worker path is not necessary
- reduced effective concurrency can help but does not eliminate failures
- still missing: a completely isolated one-worker repro with no Mina runtime loaded

### Phase 5. Product decision

Only after the runtime line is understood enough, choose one of:

- keep a guarded workaround in Mina wasm
- bypass expensive digest verification for packaged assets under strict assumptions
- move verifier-cache verification off the critical startup path
- report a Chromium/wasm issue with a minimal external repro

This phase should not start until the branch can answer two questions cleanly:

- does a minimal wasm worker repro fail without Mina code
- is the failure materially different between headless and non-headless Chromium

## Success criteria

This branch is successful if it produces one of these outcomes:

1. A minimal repro outside Mina that still fails.
2. A clear proof that the issue is Mina-specific after all.
3. A reliable workaround with measured stability across repeated runs.
4. A compact external issue reportable upstream with evidence.

## Current operational position

Until this line is resolved, treat the current chunk-size workaround as exploratory only.

The evidence today supports this statement:

- no chunk size tested so far removes the failure mode with enough confidence to rely on it operationally

That is the reason for splitting this work into a dedicated runtime-repro branch.

## Immediate next steps

1. Add a minimal wasm worker module under the local smoke harness that exports hashing helpers but does not load Mina node logic.
2. Re-run the existing JS probe and the new wasm-only probe with the same payload sizes:
   - `1 MiB`
   - `2 MiB`
   - `3 MiB`
   - `3317098` bytes
3. Compare three backends in the same harness:
   - JS `WebCrypto`
   - wasm `sha2` one shot
   - wasm `sha2` chunked plus yield
4. Record each configuration across at least 3 runs and classify it as:
   - reliable
   - variable
   - failing
5. If the minimal wasm-only probe still reproduces the instability, prepare a compact upstream-quality report with:
   - exact browser mode
   - payload sizes
   - completion variability
   - headless vs non-headless notes

## Update 2026-03-19: first minimal harness landed

The branch now has a versioned minimal harness under:

- `tools/wasm-runtime-repro/index.html`
- `tools/wasm-runtime-repro/runtime-worker.js`
- `tools/wasm-runtime-repro/src/lib.rs`
- `Makefile` target: `build-wasm-runtime-repro`

This harness does not load `mina-node-web`.

It supports:

- JS `WebCrypto`
- wasm `sha2` one shot
- wasm `sha2` chunked
- wasm `sha2` chunked plus yield
- JS-generated input copied into wasm
- wasm-generated input

The current build path for this minimal module is still aligned with the repo's threaded wasm profile:

- shared memory
- atomics
- rebuilt `std` with `-Z build-std=std,panic_abort`

That means the harness is already useful to separate "Mina logic" from "threaded wasm runtime", even though it does not yet provide the single-threaded comparison.

## Update 2026-03-19: first results from the minimal harness

### Result A. Pure JS `WebCrypto` did not reproduce the old stall

Headless Chromium run:

- page: `tools/wasm-runtime-repro/index.html`
- backend: `js_webcrypto`
- size: `3317098` bytes
- timeout budget: `60000 ms`

Observed result:

- `run.state = resolved`
- `digestByteLength = 32`
- no JS-visible errors
- server traces reached:
  - `runtime-worker:js-webcrypto.begin`
  - `runtime-worker:js-webcrypto.complete`
  - `runtime-worker:complete`

This is a meaningful change from the earlier Mina-adjacent harness behavior, where a large `crypto.subtle.digest(...)` in the worker could hang.

### Result B. Minimal threaded wasm did not reach hashing yet

Headless Chromium run:

- backend: `wasm_chunked_yielding`
- input mode: `js_copy`
- size: `3317098` bytes
- chunk size: `16384`
- yield every `4` chunks
- timeout budget: `60000 ms`

Observed result:

- `run.state = timeout`
- no JS-visible errors
- server traces reached:
  - `runtime-worker:start`
  - `runtime-worker:wasm.memory.created`
  - fetch of `pkg/wasm_runtime_repro_bg.wasm`
- traces did not reach:
  - `runtime-worker:wasm.init.complete`
  - any Rust-side hashing marker

This remained true even after removing our own `fetch('/wasm-smoke/trace')` call from the module `#[wasm_bindgen(start)]` hook, which was a plausible source of self-inflicted initialization interference.

## Updated interpretation

The current minimal harness produced two important clarifications:

1. Large-buffer `WebCrypto` does not automatically fail in the stripped-down worker harness.
2. The first minimal threaded wasm attempt is currently blocked at module initialization, before hashing begins.

That means the first runtime-repro pass did not yet isolate the original hash pathology cleanly. Instead, it split the problem into two new branches:

- branch A: the old JS `WebCrypto` instability may have depended on Mina-side runtime load, scheduling, or integration details that are absent in the minimal harness
- branch B: the minimal threaded wasm module may still be missing part of the runtime contract expected by a shared-memory `wasm-bindgen` module in this worker setup

## New working hypotheses after the first minimal harness

### H5. The earlier JS `WebCrypto` stall was environment-sensitive, not size-only

Because the minimal `js_webcrypto` probe now resolves cleanly at `3317098` bytes, payload size alone is not enough to reproduce the prior behavior.

Plausible missing ingredients include:

- additional Mina runtime load
- different worker lifecycle
- different module initialization ordering
- previous instrumentation side effects

### H6. The current minimal threaded wasm harness is not yet semantically equivalent to Mina wasm init

The timeout before `wasm.init.complete` suggests the new harness may still be wrong or incomplete at initialization time.

Candidates:

- missing `thread_stack_size`
- wrong or incomplete `init(...)` contract for the shared-memory module
- a `wasm-bindgen` threading expectation that Mina satisfies differently
- a remaining interaction between shared memory and the generated glue code

### H7. Single-threaded minimal wasm is still needed

Because this first minimal wasm harness is forced through the repo's threaded wasm profile, it cannot yet answer:

- whether the same minimal module works when built without shared memory
- whether the current timeout is caused by threading setup rather than hashing

That comparison remains a high-priority missing piece.

## Revised immediate next steps

1. Inspect the generated `pkg/wasm_runtime_repro.js` contract and compare it with the known-good Mina wasm init path.
2. Test whether the minimal threaded module needs explicit `thread_stack_size` or a different initialization object.
3. Build a truly single-threaded version of the minimal wasm module outside the workspace's forced shared-memory config.
4. Re-run the same `3317098`-byte matrix on:
   - minimal JS `WebCrypto`
   - minimal threaded wasm
   - minimal single-threaded wasm
5. Only after the minimal wasm init path is trustworthy, return to the large-digest backend comparison.
