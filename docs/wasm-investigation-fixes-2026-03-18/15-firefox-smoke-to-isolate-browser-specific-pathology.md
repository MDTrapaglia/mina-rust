# Fix 15: correr smoke en Firefox para aislar patología específica de browser

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `diagnóstico`
Estado real: `validado`
Branch: `codex/firefox-webnode-bringup`

## Problema

Todavía puede quedar duda entre un problema general de wasm/browser y un
comportamiento más específico de Chromium.

## Cambio ejecutado

- se corrió el smoke real en Firefox headless
- luego se recompiló `mina-node-web` desde la rama basada en known fixes
- se fijó `wasm_thread::Builder` para usar `worker_script_url("/wasm-smoke/mina-worker-module.js")`
  y `wasm_bindgen_shim_url("/mina-rust/pkg/mina_node_web.js")`
- se agregaron beacons mínimos de etapa en `crates/node/web/src/lib.rs`

## Resultado observado

- Firefox dejó de ser sólo control negativo:
  - `run(null, [], [], null)` resolvió en ~`9s`
  - `rpcStatus()` respondió dentro del smoke
- repetibilidad inicial:
  - `5/5` corridas `resolved`
  - `0/5` timeouts
  - `run()` entre ~`8.9s` y `9.5s`
- el estado observado del nodo fue:
  - `transition_frontier.sync.status = "Idle"`
  - `transition_frontier.sync.phase = "Bootstrap"`
- el servidor local registró la secuencia completa:
  - `worker.run.worker.enter`
  - `worker.setup.block_verifier.begin/complete`
  - `worker.setup.tx_verifier.begin/complete`
  - `worker.setup.build.complete`
  - `worker.run.worker.rpc_sender.sent`
- también quedaron confirmados los fetches de:
  - `block_verifier_index.postcard`
  - `transaction_verifier_index.postcard`

Artefactos útiles:

- `/home/mtrapaglia/mina/logs/firefox-webnode-run-v2.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-run-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-smoke-server-v2.log`

## Validación

- el smoke en Firefox sí separó hipótesis
- además mostró que, con la ruta explícita del worker wasm, el web node ya
  levanta lo suficiente como para devolver `RpcSender` y `status()`

## Dependencias

- ninguna estricta
- idealmente se corre después de `03`, `05`, `10` y `12` para observar el camino
  más normal posible

## Riesgo / tradeoff

- consume tiempo de diagnóstico y puede sumar variabilidad de entorno
- por sí solo no prueba que todo el soporte browser quede resuelto
- el cambio de `worker_script_url` explícito hoy se usa como herramienta de
  bring-up/debug y todavía hay que decidir si queda como fix permanente,
  configuración condicional o sólo harness de validación

## Alternativa descartada

Seguir iterando sólo con Chromium. Se descartó como estrategia única porque deja
sin aislar la dimensión browser-specific.
