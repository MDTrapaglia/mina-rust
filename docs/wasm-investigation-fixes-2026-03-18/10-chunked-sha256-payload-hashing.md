# Fix 10: dividir el hashing SHA-256 del payload en chunks

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `mitigación`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `698265fdb`

## Archivos

- `crates/ledger/src/proofs/verifiers.rs`

## Problema

El hash del payload `.postcard` en wasm/browser mostraba degradación fuerte y
era el frente activo del timeout.

## Cambio

- el digest del payload en wasm pasa a procesarse en chunks
- se agregó helper dedicado y test de equivalencia contra el hash completo

## Validación

- el commit agrega el test `chunked_sha256_matches_full_sha256`
- se ejecutó `cargo test -p mina-tree chunked_sha256_matches_full_sha256 --lib`
  en `codex/wasm-known-fixes` y pasó (`1/1`, `100%`)
- la motivación viene además de la evidencia previa del harness

## Dependencias

- práctica: este fix se vuelve relevante una vez que `03` y `05` permiten llegar
  al camino normal de caché

## Riesgo / tradeoff

- más iteraciones de `update()` implican overhead extra en runtimes donde el hash
  grande no era problema
- mitiga performance, pero no cambia el modelo de validación

## Alternativa descartada

Hashear todo el buffer de una sola vez. Se descartó porque ese patrón estaba
asociado al timeout observado en browser.
