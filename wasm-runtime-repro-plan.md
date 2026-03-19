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

## Update 2026-03-19: real single-threaded build and new split point

### Result C. A true single-threaded package now exists

The runtime-repro harness now has two wasm package variants:

- `tools/wasm-runtime-repro/pkg`
  - threaded/shared-memory build aligned with the repo's wasm profile
- `tools/wasm-runtime-repro/pkg-single`
  - built from a temporary standalone crate outside the repo's `.cargo/config.toml`
  - no shared-memory `WebAssembly.Memory` creation in generated JS
  - generated glue only calls `wasm.__wbindgen_start()`

Important implementation detail:

- trying to neutralize the threaded profile from inside the workspace was not enough
- both `--config ... rustflags=[]` and `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=''` still inherited the repo's threaded linker flags when Cargo ran with the repo worktree as current directory
- the working solution was to generate a tiny standalone crate in `/tmp`, `cd` into that directory, compile there, and copy the resulting `pkg-single` back into the harness

This means the single-threaded comparison is now real, not a fake variant still contaminated by shared-memory linker args.

### Result D. Single-threaded worker no longer stalls at `wasm.init(shared)`, but hangs even earlier

Headless Chromium run:

- backend: `wasm_chunked_yielding`
- `wasm_profile=single`
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
  - `runtime-worker:wasm.bindings.import.begin`
  - fetch of `pkg-single/wasm_runtime_repro.js`
- traces did not reach:
  - `runtime-worker:wasm.bindings.import.complete`
  - `runtime-worker:wasm.init.begin`
  - `runtime-worker:wasm.init.complete`
  - any Rust-side hashing marker

In the latest run, the worker did not even reach the `import()` completion for the single-threaded package. Earlier single-profile runs fetched the snippet file as well, but the last traced run still timed out before `import.complete`.

### What changed in the interpretation

The comparison is now cleaner:

1. The threaded minimal wasm variant still stalls after creating shared memory and before `wasm.init.complete`.
2. The true single-threaded variant does not reproduce that exact stall. Instead, it currently hangs at dynamic module import of `pkg-single`.

So the minimal runtime repro has already disproven one simplistic theory:

- this is not a single, stable failure mode that survives unchanged across threaded and single-threaded wasm packaging

Instead, the runtime path is now split:

- threaded profile: first visible stall is after `WebAssembly.Memory({ shared: true })` and before `init.complete`
- single profile: first visible stall is at `await import("/tools/wasm-runtime-repro/pkg-single/wasm_runtime_repro.js")`

That does not explain the original Mina behavior yet, but it does show that the worker/runtime sensitivity changes substantially with the packaging and initialization path.

## New working hypotheses after the single-threaded build

### H8. The threaded and single-threaded failures are different runtime pathologies

The latest evidence suggests we are not looking at one bug with one location.

Current split:

- threaded build: shared-memory initialization path
- single-threaded build: worker module loader or module evaluation path

### H9. The single-threaded package may be hanging during worker-side ESM resolution or evaluation

Because the last traced run reached `wasm.bindings.import.begin` but not `import.complete`, the active frontier for the single-threaded path is now:

- ESM loading
- nested import resolution
- top-level evaluation of the generated package

This is earlier than wasm instantiation and earlier than hashing.

### H10. The minimal harness is now good enough to keep deconvolving runtime layers

Even though the latest single-threaded run did not reach hashing, the harness is now useful in a stronger way:

- it can separate module-loader issues
- it can separate shared-memory init issues
- it can separate later hashing issues once import/init become stable

## Revised next steps after the single-threaded comparison

1. Instrument around the single-threaded `import()` boundary until we can tell whether the stall is in:
   - fetching nested module dependencies
   - module evaluation
   - or the transition from `import()` to `init()`
2. Add a direct single-threaded import probe outside the hashing path, ideally:
   - page main thread import
   - worker import without calling any exported function
3. Re-run the threaded variant with the same extra `import()/init()` traces for symmetry.
4. Only after both minimal profiles can reliably reach `init.complete`, resume the backend comparison (`WebCrypto` vs wasm `sha2`).

## Result E. Main-thread imports resolve, but worker-side dynamic imports stall even for a trivial ESM module

To separate package-specific evaluation from worker-module loading itself, the harness was extended with:

- `page_import_probe=<single|threaded|trivial>`
- `skip_worker=1`
- `backend=esm_import_only`
- `esm_target=<single|threaded|trivial>`
- a trivial ESM module at `tools/wasm-runtime-repro/probe-module.js`

This allowed three direct comparisons without rebuilding wasm:

1. Main thread import of `pkg-single`, skipping the worker.
2. Main thread import of the trivial module, skipping the worker.
3. Worker-side dynamic import only, without calling `init()` or any hash function.

Observed artifacts:

- main-thread `pkg-single` import:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-page-import-single-v1.html`
- main-thread trivial import:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-page-import-trivial-v1.html`
- worker trivial import:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-import-trivial-v2.html`
- worker `pkg-single` import:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-import-single-esm-v2.html`
- shared server trace:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-server-8152-v1.log`

Observed behavior:

- main-thread import of `pkg-single` resolved in ~`36 ms`
- main-thread import of the trivial module resolved in ~`52 ms`
- worker-side `await import("/tools/wasm-runtime-repro/probe-module.js")` timed out
- worker-side `await import("/tools/wasm-runtime-repro/pkg-single/wasm_runtime_repro.js")` also timed out

Most importantly, the worker traces reached:

- `runtime-worker:start`
- `runtime-worker:esm.import.begin`
- `GET /tools/wasm-runtime-repro/probe-module.js` for the trivial case
- `GET /tools/wasm-runtime-repro/pkg-single/wasm_runtime_repro.js` for the generated package case

But they did not reach:

- `runtime-worker:esm.import.complete`

So the active frontier moved again. The current headless pathology is no longer specific to wasm or hashing:

- in this minimal harness, dynamic `import()` inside the worker stalls even for a trivial ESM module

## What changed in the interpretation after Result E

This is a stronger separation than before:

1. Main thread ESM loading is healthy in the same browser session.
2. Worker startup itself is healthy enough to run code and emit traces.
3. The worker stall now reproduces before wasm init and before any hash function.
4. The stall is not specific to `wasm-bindgen` output, because it also reproduces for `probe-module.js`.

That means the currently dominant runtime-repro frontier is:

- dynamic nested module import from within a module worker under headless Chromium

## New working hypotheses after Result E

### H11. The active repro is now a worker-side dynamic `import()` issue, not a wasm issue

Because both:

- `import("/tools/wasm-runtime-repro/probe-module.js")`
- `import("/tools/wasm-runtime-repro/pkg-single/wasm_runtime_repro.js")`

stall in the worker but succeed on the page main thread, the smallest current repro is no longer “wasm in worker”. It is closer to:

- “dynamic ESM import from a module worker in this headless Chromium runtime can hang after fetch and before import completion”

### H12. Static worker imports should be tested next

The next discriminant is whether the runtime only breaks on:

- dynamic `await import(...)`

or whether it also breaks on:

- static top-level `import ... from "./probe-module.js"`

inside a module worker.

If static imports work and dynamic imports do not, the issue narrows to the dynamic worker-module loader path. If both fail, the issue is broader in worker-side nested module evaluation.

## Revised next steps after Result E

1. Add worker entrypoints with static top-level imports of:
   - `probe-module.js`
   - `pkg-single/wasm_runtime_repro.js`
2. Compare static-import worker behavior against the current dynamic-import worker behavior.
3. If static imports succeed, minimize further around `await import(...)` in workers and prepare an upstream Chromium-style repro.
4. If static imports also fail, test the same minimal harness outside headless mode or in another browser to separate “Chromium headless” from “worker module loader” more cleanly.

## Result F. Static worker imports resolve, dynamic worker imports still hang after removing the listener race

After Result E, the harness was refined in two ways:

1. The page now sends an explicit `{ type: "start" }` message after installing worker listeners.
2. The workers wait for that start message before posting completion, removing the possibility that a very fast worker could beat the page-side listener setup.

Additional worker entrypoints were added:

- `tools/wasm-runtime-repro/runtime-worker-static-trivial.js`
- `tools/wasm-runtime-repro/runtime-worker-static-single.js`

These entrypoints use static top-level imports instead of `await import(...)`.

Observed artifacts:

- dynamic worker, trivial target:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-dynamic-trivial-v4.html`
- dynamic worker, single target:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-dynamic-single-v4.html`
- static worker, trivial target:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-static-trivial-v1.html`
- static worker, single target:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-browser-worker-static-single-v1.html`
- shared server trace:
  - `/home/mtrapaglia/mina/logs/wasm-runtime-repro-server-8153-v1.log`

Observed behavior after the handshake fix:

- `worker_entry=static_trivial` resolved
- `worker_entry=static_single` resolved
- `worker_entry=dynamic&backend=esm_import_only&esm_target=trivial` timed out
- `worker_entry=dynamic&backend=esm_import_only&esm_target=single` timed out

Most importantly:

- static worker imports reached `runtime-worker:static.import.complete`
- dynamic worker imports reached `runtime-worker:esm.import.begin`
- dynamic worker imports did not reach `runtime-worker:esm.import.complete`

That means the old “maybe the page missed a very fast worker message” explanation is no longer enough to explain the active failure. Once the start-race is removed, the split remains:

- static imports in workers: healthy
- dynamic `await import(...)` in workers: hanging

## What changed in the interpretation after Result F

This is the strongest separation produced by the minimal repro so far.

It narrows the active runtime issue to:

- dynamic nested ESM import from inside a module worker under headless Chromium

and simultaneously weakens several broader theories:

- not “workers cannot import modules”
- not “all nested worker imports fail”
- not “wasm-bindgen package loading always fails in workers”
- not “the page-side listener race fully explained the earlier timeouts”

## New working hypotheses after Result F

### H13. The dominant repro is specific to dynamic `import()` inside a module worker

Because both static workers succeed and both dynamic workers hang, the minimal runtime bug is now best described as:

- “`await import(...)` inside a module worker can hang in this Chromium headless runtime, even when the same target loads via static import”

### H14. The issue is now small enough for an upstream-quality repro

The current harness no longer needs wasm hashing or Mina logic to reproduce the active frontier.

The smallest meaningful variants are now:

- dynamic worker import of `probe-module.js` -> hangs
- static worker import of `probe-module.js` -> resolves

That is a much cleaner upstream repro than anything we had before.

## Revised next steps after Result F

1. Extract the static-vs-dynamic worker import comparison into an even smaller standalone repro, ideally with:
   - one HTML file
   - one worker with dynamic import
   - one worker with static import
   - one trivial imported module
2. Run that tiny repro in:
   - Chromium headless
   - a regular browser session if available
   - optionally Firefox for contrast
3. If the split persists, prepare an upstream issue centered on:
   - worker module
   - dynamic import
   - headless Chromium
   - trivial ESM target
