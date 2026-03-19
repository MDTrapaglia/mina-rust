# Fix 02: documentar que la referencia monotónica no cruza workers

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 1

Clasificación: `hardening`
Estado real: `parcial`
Branch / commit: `codex/wasm-known-fixes`, `6ef07ca9e`

## Archivos

- `libs/redux/src/store.rs`

## Problema

El bug puede reintroducirse si otro cambio vuelve a asumir que una referencia
monotónica puede compartirse entre workers wasm.

## Cambio

- se agregó comentario inline explicando que en wasm cada worker tiene su propio
  origen monotónico

## Validación

- el comentario quedó junto al estado global de tiempo
- no hay todavía doc externa ni test dedicado a esta restricción

## Dependencias

- depende conceptualmente de `01`

## Riesgo / tradeoff

- la documentación inline ayuda, pero no evita por sí sola una regresión futura
- si el comentario se desactualiza, puede generar falsa confianza

## Alternativa descartada

Confiar sólo en memoria institucional o en tests. Se descartó porque la
restricción es conceptual y conviene dejarla visible en el punto de uso.
