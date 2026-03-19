# Plan de trabajo: traza interna del worker en `mina-node-web`

## Objetivo actual

Determinar si el flujo del worker wasm se corta:

- antes de ejecutar `spawn_local(...)`
- entre `spawn_local(...)` y `setup_node(...)`
- dentro de `setup_node(...)`
- o antes de devolver `RpcSender`

## Estado confirmado

- `mina-node-web` compila y carga en browser real.
- `build_env()` funciona en browser real.
- `run(...)` sigue sin resolver dentro de `60000ms`.
- Node no sirve como proxy fiable del runtime web.
- Con `worker_script_url("/wasm-smoke/mina-worker-module.js")` el worker explícito sí entra a Rust.
- En la última validación apareció el beacon `worker:run.worker.enter`.
- La corrida más reciente mostró que el worker paniqueaba antes de `spawn_local(...)` por la instrumentación: se estaba usando un `Instant` creado en el main thread dentro del worker.
- Después se confirmó que incluso `Instant` locales dentro del worker/`setup_node()` seguían introduciendo una panic en `wasm_timer::wasm::Instant`.
- Se removieron todas las mediciones por tiempo del lado worker para dejar solo `trace_stage(...)` y logs textuales.
- La secuencia de stages quedó en `worker:run.worker.enter` antes de volver a panicar.
- El siguiente sospechoso es el path de `log::info!`/tracing dentro del worker, porque está exactamente entre `run.worker.enter` y `run.worker.spawn_local.scheduled`.
- Última confirmación: sin `log::info!` del lado worker, el flujo avanza hasta:
  - `worker:run.worker.spawn_local.scheduled`
  - `worker:run.worker.keepalive`
  - `worker:run.worker.keepalive.before_throw`
  - `worker:run.worker.spawn_local.enter`
- El siguiente bloqueo visible es el throw intencional de `keep_worker_alive_cursed_hack()`.
- Incluso con el throw tratado como esperado en el shim, seguía faltando `worker:setup.start`.
- Sospechoso actual: el `log::info!` inicial de `setup_node()` que todavía corría antes del primer beacon `setup.start`.

## Hipótesis activa

El fallo principal ya no está en el bootstrap `blob/module` original. El foco actual sigue siendo el tramo interno del worker después de `run.worker.enter`, pero primero hubo que remover una panic artificial introducida por la propia traza.

## Instrumentación activa

### Rust: `crates/node/web/src/lib.rs`

Se están emitiendo beacons en:

- `run.worker.enter`
- `run.worker.spawn_local.enter`
- `run.worker.spawn_local.scheduled`
- `run.worker.setup_node.complete`
- `run.worker.rpc_sender.sent`
- `run.worker.keepalive`
- `run.worker.keepalive.before_throw`

Y dentro de `setup_node(...)` en:

- `setup.start`
- `setup.block_verifier.begin/complete`
- `setup.tx_verifier.begin/complete`
- `setup.genesis.begin/complete`
- `setup.builder.begin`
- `setup.builder.verifiers_wired`
- `setup.seed_urls.begin/complete`
- `setup.initial_peers.begin/complete`
- `setup.block_producer.begin/complete`
- `setup.service.begin`
- `setup.build.begin/complete`

### JS: `wasm-smoke/mina-worker-module.js`

El worker explícito ahora emite beacons en:

- `worker-shim:onmessage`
- `worker-shim:init.complete`
- `worker-shim:entry.before`
- `worker-shim:entry.complete`
- `worker-shim:close.before`
- `worker-shim:error`

Esto permite distinguir:

- si el worker explícito recibe correctamente `module/memory/work`
- si `init(module, memory)` completa
- si `wasm_thread_entry_point(work)` llega a ejecutarse
- si el error ocurre antes o después de entrar al entry point wasm

## Último hallazgo útil

La última traza del servidor mostró esta secuencia:

- `worker-shim:init.complete`
- `worker-shim:entry.before`
- `worker:run.worker.enter`

Después de eso apareció:

- `worker-shim:error`
- `RuntimeError: unreachable`

La stack apunta a:

- `<wasm_timer::wasm::Instant as core::ops::arith::Sub>::sub`

Conclusión:

- el worker sí entra al entry point wasm
- el corte ocurre antes de `run.worker.spawn_local.enter`
- la causa inmediata fue una resta de `Instant` entre contextos incompatibles, introducida por la instrumentación

Corrección aplicada:

- se reemplazó el uso de `run_started.elapsed()` dentro del worker por un `worker_started = Instant::now()` creado localmente en el mismo thread
- luego se removieron por completo las mediciones con `Instant` dentro del worker y de `setup_node()`, porque también paniqueaban en este runtime wasm

## Archivos y rutas útiles

- repo: `/home/mtrapaglia/mina/mina-rust`
- harness: `/home/mtrapaglia/mina/wasm-smoke/index.html`
- worker explícito: `/home/mtrapaglia/mina/wasm-smoke/mina-worker-module.js`
- servidor local: `/home/mtrapaglia/mina/wasm-smoke/server.mjs`
- build actual en curso/log: `/home/mtrapaglia/mina/logs/build-wasm-trace-worker-v2.log`

URL de prueba actual:

```text
http://127.0.0.1:8124/wasm-smoke/index.html?timeout_ms=60000
```

## Siguiente paso inmediato

1. Terminar el rebuild wasm sin mediciones `Instant` dentro del worker.
2. Repetir el smoke en browser real.
3. Leer en el servidor la secuencia de stages para ubicar el último punto alcanzado entre:
   - `worker-shim:init.complete`
   - `worker-shim:entry.before`
   - `worker:run.worker.enter`
   - `worker:run.worker.spawn_local.scheduled`
   - `worker:run.worker.spawn_local.enter`
   - `worker:setup.*`
   - `worker:run.worker.rpc_sender.sent`

## Criterio de éxito

Esta etapa queda resuelta si identificamos el último `stage` alcanzado antes del bloqueo, con suficiente precisión como para decidir el siguiente parche en uno de estos puntos:

- scheduler async del worker
- `keep_worker_alive_cursed_hack()`
- inicialización de verificadores
- `node_builder.build()`
- envío de `RpcSender`

## Cambio actual

Se removió `log::info!` del tramo worker:

- entre `run.worker.enter` y `run.worker.spawn_local.scheduled`
- dentro del async del worker
- dentro de `setup_node(...)`

Objetivo:

- comprobar si la panic restante viene del logging/tracing wasm en worker threads

## Cambio actual

El worker explícito ahora intercepta el error esperado:

- `Cursed hack to keep workers alive`

Y lo trata como señal de keepalive en vez de repropagarlo al browser.

Objetivo:

- mantener vivo el worker lo suficiente como para observar stages de `setup_node(...)` y `rpc_sender`

## Cambio actual

Se removió el `log::info!` inicial de `setup_node()`.

Objetivo:

- comprobar si el flujo avanza por fin a `worker:setup.start` y `worker:setup.block_verifier.begin`
