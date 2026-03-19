# Fix 04: trazar el `status` de la respuesta HTTP en wasm

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 2

Clasificación: `diagnóstico`
Estado real: `no implementado`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Archivos objetivo

- `crates/core/src/http.rs`
- eventualmente la capa de tracing/beacons usada en el harness

## Problema

Sin un evento de tracing para `status.<code>`, el lado cliente obliga a inferir
si hubo `404`, `200` o redirección a partir de logs indirectos.

## Cambio propuesto

- emitir un evento explícito con el status HTTP observado en wasm

## Validación

- no implementado
- hoy sólo existe el `status` dentro del mensaje de error del fail-fast

## Dependencias

- útil sobre todo si se mantiene un harness de tracing activo
- complementa `03`

## Riesgo / tradeoff

- más eventos y algo más de ruido en observabilidad
- si se implementa con un transporte costoso, puede reintroducir artefactos de
  ejecución

## Alternativa descartada

Depender sólo de logs del server. Se descartó porque deja ciegos los puntos de
decisión del lado del cliente.
