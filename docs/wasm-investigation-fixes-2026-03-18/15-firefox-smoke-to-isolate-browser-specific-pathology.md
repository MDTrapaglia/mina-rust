# Fix 15: correr smoke en Firefox para aislar patología específica de browser

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `diagnóstico`
Estado real: `propuesto`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Problema

Todavía puede quedar duda entre un problema general de wasm/browser y un
comportamiento más específico de Chromium.

## Cambio propuesto

- correr el mismo smoke en Firefox y comparar comportamiento

## Validación

- no implementado
- su valor está en separar hipótesis, no en corregir código directamente

## Dependencias

- ninguna estricta
- idealmente se corre después de `03`, `05`, `10` y `12` para observar el camino
  más normal posible

## Riesgo / tradeoff

- consume tiempo de diagnóstico y puede sumar variabilidad de entorno
- no garantiza un fix; sólo mejora la calidad de la hipótesis

## Alternativa descartada

Seguir iterando sólo con Chromium. Se descartó como estrategia única porque deja
sin aislar la dimensión browser-specific.
