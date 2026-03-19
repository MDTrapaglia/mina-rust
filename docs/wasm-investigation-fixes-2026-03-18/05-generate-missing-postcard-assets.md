# Fix 05: generar localmente los `.postcard` faltantes

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 2

Clasificación: `fix real`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `4f98d3342`, `e5e03d93c`

## Archivos

- `tools/generate-webnode-circuit-blobs/src/main.rs`
- `frontend/scripts/download-webnode.sh`

## Problema

Si faltan `block_verifier_index.postcard` y
`transaction_verifier_index.postcard`, el runtime cae en rutas caras o
equivocadas y la investigación se desvía.

## Cambio

- se agregó una herramienta para generar blobs localmente
- el script de assets ahora genera los `.postcard` en lugar de asumir que llegan
  desde el release

## Validación

- `download-webnode.sh` quedó con chequeo sintáctico válido
- el script ahora verifica existencia de los archivos generados
- no se reran el flujo completo de generación en esta sesión

## Dependencias

- depende de la herramienta agregada en `4f98d3342`
- complementa `03` y `06`

## Riesgo / tradeoff

- generar localmente agrega costo de CPU/tiempo al setup
- vuelve el script más dependiente del entorno Rust/Nix correcto

## Alternativa descartada

Reconstruir siempre el verifier en runtime. Se descartó porque es más caro y
oculta un problema de distribución de assets.
