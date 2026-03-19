# Fix 11: usar `HASH_CHUNK_SIZE` chico solo en wasm/browser

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `mitigación`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `698265fdb`

## Archivos

- `crates/ledger/src/proofs/verifiers.rs`

## Problema

La sensibilidad al tamaño de chunk no aparecía como un problema general del
target nativo, sino de wasm/browser.

## Cambio

- el chunk chico quedó aplicado sólo en la ruta wasm
- el valor actual implementado es `8 KiB`

## Validación

- revisión del código: el tamaño chico sólo se usa bajo `target_family = "wasm"`
- la decisión se apoya en la evidencia comparativa levantada durante la
  investigación

## Dependencias

- depende de `10`

## Riesgo / tradeoff

- puede no ser el óptimo universal para todos los navegadores
- suma overhead en cantidad de llamadas aunque reduzca picos de latencia

## Alternativa descartada

Usar el mismo chunk size en todos los targets. Se descartó porque no había
evidencia para degradar nativo por un problema localizado en wasm/browser.
