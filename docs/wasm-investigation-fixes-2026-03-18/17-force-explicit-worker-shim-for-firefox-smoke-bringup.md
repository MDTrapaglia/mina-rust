# Fix 17: forzar worker shim explícito para el bring-up Firefox del smoke wasm

Fuente: seguimiento posterior a `Fix 15`

Clasificación: `diagnóstico`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `757dba46a`

## Archivos

- `crates/node/web/src/lib.rs`

## Problema

En el branch de known fixes, `wasm_thread` había vuelto al camino automático de
workers `blob:`. En Firefox eso dejaba el bring-up del smoke sin visibilidad
interna: el worker real no pasaba por el shim instrumentado del harness y
`run(null, [], [], null)` quedaba opaco cuando colgaba o timeouteaba.

## Cambio

- se reinstaló un `thread::Builder` explícito con:
  - `worker_script_url("/wasm-smoke/mina-worker-module.js")`
  - `wasm_bindgen_shim_url("/mina-rust/pkg/mina_node_web.js")`
- el override se activa sólo cuando `location.pathname` empieza con
  `"/wasm-smoke/"`
- se agregaron beacons baratos de etapa vía `fetch("/wasm-smoke/trace?...")`
  para distinguir:
  - bootstrap main thread
  - entrada al worker
  - avance de `setup_node(...)`
  - envío de `RpcSender`

## Validación

- `make build-wasm` cerró exitosamente usando
  `PROTOC=/tmp/protoc-34/bin/protoc`
- smoke Firefox headless:
  - `run(null, [], [], null)` resolvió en ~`9s`
  - `rpcStatus()` respondió
- repetibilidad inicial:
  - `5/5` corridas `resolved`
  - `0/5` timeouts
- el log del servidor mostró:
  - `worker.setup.block_verifier.begin/complete`
  - `worker.setup.tx_verifier.begin/complete`
  - `worker.setup.build.complete`
  - `worker.run.worker.rpc_sender.sent`

Artefactos:

- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-smoke-server-v3.log`

## Dependencias

- complementa `15`, que valida Firefox como control/browser más robusto
- usa el harness local `/home/mtrapaglia/mina/wasm-smoke/`
- depende de tener `pkg/mina_node_web.js` y `pkg/mina_node_web_bg.wasm`
  recompilados para el smoke

## Riesgo / tradeoff

- no es un fix de producto general: queda guardado al contexto `"/wasm-smoke/"`
- introduce instrumentación de diagnóstico dentro del bundle wasm
- si el layout del harness cambia, también hay que actualizar estas rutas

## Alternativa descartada

Seguir únicamente con el worker `blob:` automático. Se descartó para esta etapa
porque impedía observar el punto exacto del bring-up en Firefox y demoraba la
iteración sobre el bootstrap wasm.
