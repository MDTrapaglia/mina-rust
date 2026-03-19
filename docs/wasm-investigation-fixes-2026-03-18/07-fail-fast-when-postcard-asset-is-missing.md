# Fix 07: fallar temprano si falta un asset `.postcard`

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 2

Clasificación: `hardening`
Estado real: `parcial`
Branch / commit: `codex/wasm-known-fixes`, `fbad99a22`, `e5e03d93c`

## Archivos

- `crates/core/src/http.rs`
- `frontend/scripts/download-webnode.sh`

## Problema

Si falta un asset esperado, el sistema puede seguir por un camino incorrecto o
mucho más caro, dificultando el diagnóstico.

## Cambio

- en runtime wasm, el fetch falla temprano si la respuesta no es exitosa
- en setup, el script verifica que los assets esperados existan

## Validación

- hay fail-fast a nivel HTTP y a nivel script
- no existe todavía un error semántico dedicado del lado del loader de verifier

## Dependencias

- complementa `03`, `05` y `06`

## Riesgo / tradeoff

- menor tolerancia a entornos parcialmente rotos
- si el operador esperaba fallback silencioso, ahora verá fallas explícitas

## Alternativa descartada

Mantener fallback silencioso a reconstrucción. Se descartó porque encarece el
arranque y esconde el error real.
