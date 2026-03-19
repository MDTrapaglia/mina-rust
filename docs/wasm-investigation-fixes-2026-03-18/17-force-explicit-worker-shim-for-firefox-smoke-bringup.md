# Fix 17: forzar worker shim explícito para el bring-up Firefox del smoke wasm

Fuente: seguimiento posterior a `Fix 15`

Clasificación: `diagnóstico`
Estado real: `retirado tras revalidación`
Branch / commit: `codex/wasm-known-fixes`, `757dba46a` -> working tree `2026-03-19`

## Archivos

- `crates/node/web/src/lib.rs`

## Problema original

En el branch de known fixes, `wasm_thread` había vuelto al camino automático de
workers `blob:`. En Firefox eso dejaba el bring-up del smoke sin visibilidad
interna: el worker real no pasaba por el shim instrumentado del harness y
`run(null, [], [], null)` quedaba opaco cuando colgaba o timeouteaba.

## Cambio aplicado inicialmente

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

## Revalidación posterior

- sobre `2026-03-19` se retiró de nuevo sólo el override explícito del worker,
  manteniendo el resto del branch de known fixes igual
- `PROTOC=/tmp/protoc-34/bin/protoc make build-wasm`: `ok`
- Firefox headless smoke sin override explícito:
  - `5/5` corridas `resolved`
  - `0/5` timeouts
  - `run()` entre ~`10.3s` y `11.0s`
  - `rpcStatus()` respondió en todas las corridas
- el harness volvió a mostrar workers `blob:` del camino default de
  `wasm_thread`

Artefactos:

- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-smoke-server-v3.log`
- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-default-builder-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-default-builder-server-v1.log`

## Conclusión actual

El override explícito del worker ya no se sostiene como fix necesario para
Firefox. Su valor real fue transitorio:

- permitió instrumentar una etapa opaca del smoke
- ayudó a confirmar que Firefox podía levantar el nodo
- pero la revalidación posterior mostró que el builder default también pasa
  `5/5`

Por eso este fix queda mejor clasificado como diagnóstico histórico retirado, no
como dependencia actual del branch.

## Dependencias

- complementa `15`, que valida Firefox como control/browser más robusto
- usa el harness local `/home/mtrapaglia/mina/wasm-smoke/`
- depende de tener `pkg/mina_node_web.js` y `pkg/mina_node_web_bg.wasm`
  recompilados para el smoke

## Riesgo / tradeoff

- mientras estuvo activo, no era un fix de producto general
- además acoplaba el bundle wasm al layout puntual del harness
- mantenerlo después de la revalidación sólo agregaría complejidad innecesaria

## Alternativa descartada

Seguir sosteniéndolo como requisito del branch. Se descarta después de la
revalidación porque el camino `blob:` automático también levanta el nodo en
Firefox con buena repetibilidad.
