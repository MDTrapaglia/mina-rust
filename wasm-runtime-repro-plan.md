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

## Working hypothesis

The strongest working hypothesis is not "a specific bad chunk offset" and not "a single broken SHA implementation".

The evidence currently fits a runtime interaction involving:

- wasm worker scheduling
- shared memory / multithreaded wasm runtime behavior
- long-running CPU work inside a worker
- Chromium headless nondeterminism under load

Chunk size matters because it changes the shape of the work:

- larger chunks reduce loop overhead but increase uninterrupted work per update
- smaller chunks increase per-chunk overhead and the number of updates
- yielding inserts scheduler boundaries that can help or hurt depending on timing

So chunk size is influencing runtime behavior, but no result so far proves that chunk size is the root cause.

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

### Phase 3. Shared-memory sensitivity

Repeat the minimal repro with:

- copied `Uint8Array`
- wasm memory-backed slice
- worker-local copied buffer before hashing

Goal:

- determine whether shared memory is a necessary ingredient

Exit criteria:

- confirm or reject "shared memory is required to reproduce"

### Phase 4. Concurrency sensitivity

Run the same minimal repro:

- alone in one worker
- with extra workers alive
- with forced lower effective concurrency when possible

Goal:

- quantify whether contention is causal or merely aggravating

Exit criteria:

- confirm whether single-worker repro is sufficient

### Phase 5. Product decision

Only after the runtime line is understood enough, choose one of:

- keep a guarded workaround in Mina wasm
- bypass expensive digest verification for packaged assets under strict assumptions
- move verifier-cache verification off the critical startup path
- report a Chromium/wasm issue with a minimal external repro

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
