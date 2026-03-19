# Fix 08: remover tracing síncrono con `XMLHttpRequest` en workers

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 3

Clasificación: `diagnóstico`
Estado real: `no aplica al branch`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Archivos relevantes en investigación

- `crates/ledger/src/proofs/verifiers.rs`
- `crates/ledger/src/proofs/caching.rs`

## Problema

El beacon síncrono en workers alteraba el scheduling y contaminaba la
observabilidad.

## Estado actual

Ese tracing síncrono fue un artefacto de investigación local. No quedó presente
en `codex/wasm-known-fixes`, por lo que no hubo que removerlo en esa rama.

## Validación

- revisión del diff/branch: no hay commit de producto asociado a esta técnica

## Dependencias

- ninguna

## Riesgo / tradeoff

- si se reintroduce tracing síncrono más adelante, el problema puede reaparecer
- sin ese tracing, la observabilidad puede ser menos intrusiva pero también menos
  detallada

## Alternativa descartada

Mantenerlo para preservar orden estricto de eventos. Se descartó porque el costo
era deformar la ejecución real.
