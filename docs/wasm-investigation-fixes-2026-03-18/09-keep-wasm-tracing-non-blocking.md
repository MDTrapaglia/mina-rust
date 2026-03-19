# Fix 09: mantener el tracing wasm no bloqueante

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 3

Clasificación: `diagnóstico`
Estado real: `no aplica al branch`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Problema

La observabilidad pesada en workers wasm puede cambiar el comportamiento que
intenta medir.

## Estado actual

No hay una política formal codificada en `codex/wasm-known-fixes`, pero el
branch tampoco introduce el tracing síncrono problemático.

## Validación

- revisión del branch: no aparece el transporte síncrono descrito en la
  investigación

## Dependencias

- conceptualmente ligada a `08`

## Riesgo / tradeoff

- tracing más liviano reduce artefactos, pero puede perder precisión temporal
- sin un contrato explícito, otro experimento podría romper esta regla

## Alternativa descartada

Subir el nivel de instrumentación aunque bloquee workers. Se descartó porque el
diagnóstico deja de describir el runtime real.
