# Plan de trabajo: validación browser real de `mina-node-web`

## Punto de partida

Estado ya validado:

- `mina-node` y `mina-node-native` compilan
- `mina-node-web` compila
- el build wasm es reproducible con `PROTOC` explícito
- la validación en Node no sirve como proxy del runtime real del web node

Estado no resuelto:

- falta validar `mina-node-web` en browser real
- el smoke previo con `chromium --headless` fue no concluyente
- el reintento automatizado con `chromium` gráfico tampoco produjo requests visibles al harness

Estado nuevo ya validado:

- un browser real remoto sí cargó correctamente el harness
- `crossOriginIsolated` fue `true`
- `init` y `build_env()` funcionaron
- `run(...)` quedó en timeout sin errores visibles
- Node volvió a fallar como proxy del runtime real incluso reintentando una inicialización más cercana al browser
- con `timeout_ms=60000`, `run(...)` quedó en estado `pending` sin errores visibles al momento de la captura
- con la corrida completa, `run(...)` terminó en `timeout` a `60000ms` sin errores visibles
- el bundle wasm fue recompilado con instrumentación de tiempos en `run()` y `setup_node()`
- el harness browser ahora captura `console.log/info/warn/error/debug` en el JSON visible
- la primera corrida instrumentada falló por una panic introducida por la telemetría local
- la corrección reemplaza `std::time::Instant` por `redux::Instant`, que es compatible con wasm
- la corrida corregida mostró solo el log `run(): start` en la página, sin errores
- eso no prueba que el worker no arranque: los logs del worker no comparten el `console` de la ventana
- el nuevo enfoque envía beacons de etapa a `/wasm-smoke/trace?...` para observar el progreso desde el servidor local
- el servidor wasm smoke fue corregido para:
  - responder `204` en `/wasm-smoke/trace`
  - loguear la URL completa
  - registrar `stage=...` decodificado en el log local
- la corrida con beacons confirmó stages del main thread:
  - `main:run.start`
  - `main:run.spawn.before`
  - `main:run.spawn.after`
- no apareció ningún `worker:...` en el servidor

## Objetivo de esta etapa

Confirmar si `mina-node-web` carga e inicializa correctamente en un navegador
real y separar:

- fallo del entorno de prueba
- fallo del harness local
- fallo real del runtime wasm/web worker

## Pasos

### 1. Levantar el servicio local

Meta:

- servir el harness wasm con cabeceras `COOP`/`COEP`

Comando:

```bash
node /home/mtrapaglia/mina/wasm-smoke/server.mjs
```

Éxito:

- responde en `http://127.0.0.1:8123/`

### 2. Abrir acceso desde otra máquina

Meta:

- usar browser real sin depender de la sesión SSH actual

Comando recomendado desde la máquina cliente:

```bash
ssh -L 8123:127.0.0.1:8123 usuario@host
```

Éxito:

- el usuario puede abrir `http://127.0.0.1:8123/...` desde su navegador local

Observación ya validada:

- desde esta shell fue necesario exportar `DISPLAY`, `XAUTHORITY`,
  `XDG_RUNTIME_DIR` y `WAYLAND_DISPLAY` para que Chromium gráfico arrancara
- aun así, el navegador automatizado no llegó al harness
- por eso el siguiente intento útil pasa a ser browser real controlado por el usuario

Observación adicional nueva:

- Node sigue sin ser una proxy fiable del runtime real de `mina-node-web`
- `init(undefined, memory)` falla en Node por `fetch(file://...)`
- incluso pasando un `WebAssembly.Module` explícito, el bundle paniquea en el
  hook `start` porque falta `hardware_concurrency`
- evidencia:
  - `/home/mtrapaglia/mina/logs/wasm-node-proxy-retry.log`
  - `/home/mtrapaglia/mina/logs/wasm-node-proxy-retry-compiled-module.log`

### 3. Validar fase A: `init` y `build_env`

URL:

```text
http://127.0.0.1:8123/wasm-smoke/index.html?skip_run=1
```

Qué verificar:

- carga del HTML
- carga del módulo `mina_node_web.js`
- inicialización del wasm
- respuesta visible de `build_env()`

Resultado esperado:

- la página deja un JSON visible y no queda en blanco

Estado:

- `cumplido`

Resultado observado en browser real:

- `crossOriginIsolated: true`
- exports presentes
- `init: ok`
- `build_env(): ok`
- target wasm correcto: `wasm32-unknown-unknown`
- toolchain wasm correcto: `nightly 1.96.0-nightly`

### 4. Validar fase B: `run(...)`

URL:

```text
http://127.0.0.1:8123/wasm-smoke/index.html
```

Qué verificar:

- si `run(null, [], [], null)` resuelve
- si queda colgado
- si falla con error explícito

Resultado esperado:

- distinguir claramente entre init correcto y arranque completo del nodo

Estado:

- `parcial`

Resultado observado en browser real:

- `run(null, [], [], null)` quedó en:

```json
{
  "state": "timeout",
  "timeoutMs": 10000
}
```

- `errors: []`

Resultado adicional observado en browser real con `timeout_ms=60000`:

```json
{
  "state": "timeout",
  "timeoutMs": 60000
}
```

- `errors: []`

Resultado observado en la primera corrida instrumentada:

```json
{
  "state": "timeout",
  "timeoutMs": 60000,
  "elapsedMs": 60001
}
```

- `errors` incluyó `Uncaught RuntimeError: unreachable`
- `logs` mostró una panic en `std/src/sys/time/unsupported.rs` con
  `time not implemented on this platform`

Interpretación de esa corrida instrumentada:

- el fallo fue de la telemetría agregada localmente, no evidencia nueva del nodo wasm
- la fuente fue `std::time::Instant` en un target wasm/browser
- hace falta repetir el smoke con la telemetría corregida antes de sacar nuevas conclusiones

Resultado observado en la corrida corregida de la telemetría:

```json
{
  "state": "timeout",
  "timeoutMs": 60000,
  "elapsedMs": 60001
}
```

- `errors: []`
- `logs` visibles en la página solo incluyeron:
  - `mina-node-web run(): start`

Interpretación de esa corrida corregida:

- el timeout persiste
- la telemetría de la ventana funciona
- la ausencia de logs posteriores en la página no basta para culpar a `thread::spawn(...)`
- hace falta una vía de observación que funcione también dentro del worker wasm

Evidencia adicional del servidor local:

- durante el smoke browser se observaron requests a:
  - `/assets/webnode/circuit-blobs/berkeley-devnet/block_verifier_index.postcard`
  - `/assets/webnode/circuit-blobs/berkeley-devnet/transaction_verifier_index.postcard`

Interpretación parcial:

- `run(...)` alcanza al menos la etapa de provisión de índices de verificadores
- el siguiente cuello de botella probable está durante o después de la construcción
  de verificadores y antes del envío del `RpcSender`

Interpretación:

- no hay evidencia de fallo de carga o excepción JS inmediata
- el wasm arranca lo suficiente como para exponer API y responder `build_env()`
- `run(...)` entra efectivamente en ejecución y no falla de inmediato
- en browser real, `run(...)` no devolvió `RpcSender` dentro de `60s`
- el siguiente problema a investigar es si `run(...)` termina resolviendo muy tarde o si
  queda bloqueado antes de devolver el `RpcSender`
- el harness ya fue ajustado para aceptar `timeout_ms` por query string y usar
  `30000ms` por defecto en nuevas corridas

Próxima URL recomendada:

```text
http://127.0.0.1:8123/wasm-smoke/index.html?timeout_ms=30000
```

Si sigue quedando corto:

```text
http://127.0.0.1:8123/wasm-smoke/index.html?timeout_ms=60000
```

Qué observar en la próxima corrida:

- verificar en `/home/mtrapaglia/mina/logs/wasm-smoke-server.log` requests a:
  - `/wasm-smoke/trace?stage=main:run.start`
  - `/wasm-smoke/trace?stage=main:run.spawn.after`
  - `/wasm-smoke/trace?stage=worker:...`
- usar esos beacons para ubicar el último stage alcanzado antes del timeout

Resultado observado con beacons activos:

- llegaron:
  - `main:run.start`
  - `main:run.spawn.before`
  - `main:run.spawn.after`
- no llegó ningún `worker:run.worker.enter`

Interpretación actual:

- `thread::spawn(...)` sí devuelve en el caller
- el main thread no queda bloqueado en la llamada a spawn
- el problema está después del spawn lógico y antes del primer beacon dentro del worker
- los siguientes candidatos son:
  - error de arranque del worker JS
  - error al cargar el shim/module del worker
  - fallo antes de entrar al closure Rust del worker
- para aislar ese tramo, el harness ahora intercepta `window.Worker` y registra:
  - creación del worker
  - `postMessage(init)`
  - `worker.onerror`
  - `worker.onmessageerror`
  - `navigator.hardwareConcurrency`

### 5. Registrar evidencia

Guardar:

- captura de pantalla si falla
- errores de consola
- requests observados en `/home/mtrapaglia/mina/logs/wasm-smoke-server.log`

Meta:

- convertir el smoke browser en una evidencia útil, no en una impresión

### 6. Elegir siguiente frente

Si fase A falla:

- revisar primero `wasm-smoke/index.html`
- revisar imports de `pkg/mina_node_web.js`
- revisar restricciones de workers y memoria compartida

Si fase A pasa y fase B falla:

- entrar por `crates/node/web`
- revisar arranque de `run(...)`
- diferenciar fallo de networking/WebRTC de fallo de init

Si fase A y B pasan:

- usar el frontend del repo como entorno comparativo más real
- pasar a pruebas funcionales `native` vs `wasm`

Si fase A pasa y fase B queda en timeout sin errores:

- revisar si `run(...)` está diseñado para tardar más de `10s` incluso sin seeds
- aumentar el timeout del harness y volver a probar
- instrumentar timestamps antes y después de `setup_node(...)` o del retorno del `RpcSender`
- comparar contra el flujo real del frontend en `frontend/src/app/core/services/web-node.service.ts`
- instrumentar el arranque JS del worker

Estado de esa instrumentación JS:

- `cumplido` en `wasm-smoke/index.html`
- no requiere recompilar `mina-node-web`
- la próxima corrida debería mostrar en el JSON visible si:
  - el worker se construye
  - el `postMessage(init)` sale
  - el worker emite `error` o `messageerror`
- además, el harness ahora manda beacons al servidor para:
  - `harness:worker.construct#...`
  - `harness:worker.postMessage#...`
  - `harness:worker.error#...`
  - `harness:worker.messageerror#...`
  - `harness:window.error`
  - `harness:window.unhandledrejection`

Pasos a seguir ahora:

1. repetir el smoke con `http://127.0.0.1:8123/wasm-smoke/index.html?timeout_ms=60000`
2. leer en el JSON visible:
   - `worker#... construct`
   - `worker#... postMessage`
   - `worker#... error`
   - `worker#... messageerror`
3. leer en el servidor si aparecen:
   - `harness:worker.*`
   - `main:*`
   - `worker:*`
4. clasificar el fallo según la evidencia:
   - antes de crear el worker
   - al serializar o enviar `init`
   - al cargar el shim
   - al inicializar wasm dentro del worker
   - después de entrar al worker Rust
5. solo si el worker llega a `worker:*`, volver al lado Rust y localizar el tramo de `setup_node(...)`

Resultado observado en la última corrida del smoke:

- sí aparecieron beacons del harness:
  - `harness:worker.construct#...`
  - `harness:worker.postMessage#...`
- siguieron apareciendo beacons del main:
  - `main:run.start`
  - `main:run.spawn.before`
  - `main:run.spawn.after`
- siguieron ausentes:
  - `worker:*`
  - `harness:worker.error#...`
  - `harness:worker.messageerror#...`

Interpretación refinada:

- el navegador sí construye workers
- el `init` sí se envía al worker
- no hay evidencia de error JS inmediato en la creación del worker o en `postMessage(init)`
- el corte probable quedó entre:
  - bootstrap del worker
  - inicialización del módulo wasm dentro del worker
  - entrada a `#[wasm_bindgen(start)]`
  - entrada al closure Rust del hilo de `run()`

Siguiente ajuste en curso:

- fijar explícitamente la shim URL de `wasm_bindgen` para `wasm_thread`
- emitir beacons desde `#[wasm_bindgen(start)]`

Meta del ajuste:

- eliminar la dependencia del autodescubrimiento vía `script_path.js`
- distinguir si el worker alcanza a inicializar el módulo antes del hilo de `run()`

Estado del ajuste:

- `implementado` en `crates/node/web/src/lib.rs`
- `rebuild wasm` completado
- log: `/home/mtrapaglia/mina/logs/build-wasm-instrumented-v4.log`

Resultado observado con la build nueva:

- el módulo principal sí entra a:
  - `main:module.start.enter`
  - `main:module.start.builder_default_set`
  - `main:module.start.main_thread_init.done`
  - `main:module.start.init_rayon.begin`
- durante `run(...)` siguen apareciendo:
  - `harness:worker.construct#...`
  - `harness:worker.postMessage#...`
  - `main:run.start`
  - `main:run.spawn.before`
  - `main:run.spawn.after`
- y ahora además aparecieron:
  - `harness:worker.error#2`
  - `harness:worker.error#4`

Interpretación nueva:

- el problema ya no es ausencia total de señal del worker
- hay errores reales del worker después de `construct + postMessage`
- el módulo principal carga y arranca
- el siguiente dato faltante es el detalle exacto de esos `worker.error`

Siguiente ajuste:

- el harness ya envía `details=` en los beacons de `worker.error`
- el servidor local fue actualizado para loguear esos `details`
- hace falta una corrida más con el servidor reiniciado para capturar el mensaje exacto

Resultado de esa corrida:

- `worker.construct#2` y `worker.construct#4` usaron:
  - `options.type = "module"`
  - `scriptUrl = blob:http://127.0.0.1:8123/...`
- `worker.postMessage#2` y `worker.postMessage#4` enviaron correctamente:
  - `Array(WebAssembly.Module, WebAssembly.Memory, ptr)`
- luego fallaron:
  - `worker.error#2`
  - `worker.error#4`
- pero el evento de error vino sin detalle útil:
  - `message = null`
  - `filename = null`
  - `lineno = null`
  - `colno = null`

Interpretación actual:

- el problema no está en crear el worker ni en enviarle `init`
- el fallo está en el arranque de un worker `module` sobre `blob:` URL
- eso encaja mejor con un problema de carga/evaluación del script de worker
  que con un problema dentro de `setup_node(...)`
- el foco técnico se movió de Rust a la estrategia de worker script de `wasm_thread`

Siguientes pasos recomendados:

1. inspeccionar qué workers corresponden a `rayon` y cuáles a `run()`
2. probar desactivar temporalmente `options.type = "module"` o fijar un `worker_script_url` clásico explícito
3. si hace falta, reemplazar el blob autogenerado por un worker script servido desde archivo
4. solo después de estabilizar ese bootstrap volver a investigar `setup_node(...)`

Si la corrida instrumentada vuelve a fallar:

- verificar primero si el fallo viene de la telemetría o del nodo
- evitar usar `std::time::*` directamente en código wasm/browser

## Criterio de éxito

Esta etapa queda cerrada si logramos:

- confirmar que el browser real llega al servicio local
- confirmar si `skip_run=1` inicializa bien
- confirmar si `run(...)` arranca o falla
- dejar la próxima investigación enfocada en un punto técnico concreto

## Estado de cierre actual

- browser real: `ok`
- `init`: `ok`
- `build_env()`: `ok`
- `run(...)`: `timeout` a `10s`
- `run(...)` con `60s`: `timeout`, sin error visible
- Node proxy: `no confiable`
- beacons del main: `ok`
- beacons del worker: `ausentes`
- instrumentación JS del worker en el harness: `lista`
- beacons del harness para eventos JS del worker: `listos`
- siguiente foco: rebuild wasm con shim URL explícita y beacons en `#[wasm_bindgen(start)]`
- instrumentación wasm: `compilada`
- el foco en `setup_node(...)` queda diferido hasta confirmar que el worker realmente entra a Rust
