# Fix 01: tiempo monotónico por worker en wasm

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 1

Clasificación: `fix real`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `6ef07ca9e`

## Archivos

- `libs/redux/src/store.rs`

## Problema

Compartir una referencia monotónica global entre workers wasm terminaba en
`RuntimeError: unreachable` dentro de `wasm_timer::Instant::sub`.

## Cambio

- la referencia inicial deja de ser global en wasm
- pasa a inicializarse por worker/thread

## Validación

- la investigación dejó de reproducir el panic en `wasm_timer::Instant::sub`
- el commit agrega además comentario inline sobre el origen monotónico por
  worker

## Dependencias

- ninguna estricta
- sirve de base para cualquier bootstrap wasm que use tiempo monotónico

## Riesgo / tradeoff

- tiempos capturados en workers distintos ya no son comparables como si
  compartieran un único origen global
- si había código asumiendo esa comparabilidad implícita, ese supuesto deja de
  ser válido

## Alternativa descartada

Mantener una referencia global con coordinación entre workers. Se descartó
porque pelea contra el modelo del runtime wasm y deja viva la clase de bug.
