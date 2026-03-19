# Plan de trabajo: siguientes pasos `native` vs `wasm`

## Punto de partida

Estado actual ya validado:

- baseline `native` disponible
- baseline `wasm` compilada
- artefactos web generados en `/home/mtrapaglia/mina/mina-rust/pkg`
- smoke mínimo del módulo wasm ok

Estado adicional validado en esta ejecución:

- existe un harness local en `/home/mtrapaglia/mina/wasm-smoke`
- el flujo wasm reproducible quedó identificado con `PROTOC` explícito
- `build_env` nativo sigue accesible desde `./target/debug/mina build-info`
- la inicialización wasm en Node no es equivalente a browser y falla por expectativa de web worker
- el smoke browser con `chromium --headless` quedó bloqueado por el entorno local y no llegó al servidor HTTP

## Objetivo de esta etapa

Pasar de “compila” a “tenemos una comparación útil entre ambas variantes”.

## Progreso ejecutado

- harness creado:
  - `/home/mtrapaglia/mina/wasm-smoke/index.html`
  - `/home/mtrapaglia/mina/wasm-smoke/server.mjs`
- logs nuevos:
  - `/home/mtrapaglia/mina/logs/wasm-smoke-server.log`
  - `/home/mtrapaglia/mina/logs/wasm-browser-smoke-chromium.log`
  - `/home/mtrapaglia/mina/logs/wasm-browser-init-smoke.log`
  - `/home/mtrapaglia/mina/logs/wasm-browser-init-smoke-2.log`
  - `/home/mtrapaglia/mina/logs/wasm-build-env-node.err`
- estado resumido:
  - `native`: ok para `build-info`
  - `wasm` build: ok
  - `wasm` runtime en Node: falla
  - `wasm` runtime en browser headless: no concluyente por limitación del entorno

## Prioridades

### 1. Smoke test real en navegador

Meta:

- verificar que el bundle wasm inicializa en entorno browser
- confirmar que `build_env()` y `run(...)` pueden cargarse sin fallo inmediato

Trabajo:

1. preparar una página mínima o harness local para cargar `pkg/mina_node_web.js`
2. abrir el módulo en navegador
3. registrar errores de inicialización, consola y carga de wasm
4. documentar si aparecen restricciones de `WebRTC`, workers o `SharedArrayBuffer`

Resultado esperado:

- `wasm` deja de ser solo un artefacto compilado y pasa a tener una validación real de runtime

Estado:

- `parcial / bloqueado por entorno`

Progreso ejecutado:

1. se creó un servidor HTTP local con cabeceras `COOP`/`COEP` para `SharedArrayBuffer`
2. se creó una página mínima que intenta cargar `mina_node_web.js`, inicializar memoria compartida y opcionalmente llamar `run(...)`
3. se probó con `chromium --headless`
4. el navegador devolvió `Page load timed out`
5. el servidor no registró requests del navegador durante esas corridas

Conclusión:

- en esta máquina, el smoke browser con `chromium --headless` no fue válido como señal de runtime de `mina-node-web`
- no hay evidencia local de fallo del bundle wasm en browser
- sí hay evidencia de que el entorno de prueba headless no está llegando al servidor local

### 2. Primera prueba comparable `native` vs `wasm`

Meta:

- elegir una operación pequeña que exista en ambos entornos

Candidatos recomendados:

- lectura de `build_env`
- inicialización básica del nodo sin networking real
- alguna operación de serialización o RPC de solo lectura

Trabajo:

1. definir una prueba mínima que no dependa de producción de bloques
2. correr equivalente en `native`
3. correr equivalente en `wasm`
4. registrar diferencia, hipótesis y alcance

Resultado esperado:

- primera fila útil de comparación funcional entre ambos runtimes

Estado:

- `parcialmente cumplido`

Prueba usada:

- `native`: `./target/debug/mina build-info`
- `wasm`: intento de inicialización del módulo + `build_env()` desde Node usando `initSync`

Resultado:

1. `native` respondió correctamente y mantiene estos datos:
   - versión `v0.18.1-462-gab69eaed8`
   - commit `ab69eaed85fc71859fedbb2c36b2030294084a82`
   - branch `develop`
   - rustc `1.92.0`
2. `wasm` no alcanzó `build_env()` en Node
3. el módulo abortó con `RuntimeError: unreachable`
4. la traza cae en `wasm_thread::wasm32::utils::is_web_worker_thread`

Conclusión:

- la primera diferencia útil ya quedó identificada: `mina-node-web` no inicializa en un runtime Node puro aunque se le pase memoria compartida
- eso refuerza que el target correcto para comparar esta variante es browser/web worker, no Node

### 3. Endurecer el flujo wasm

Meta:

- evitar fallos falsos de entorno como el de `protoc`

Trabajo:

1. revisar si conviene fijar `PROTOC` explícitamente en el flujo de build wasm
2. verificar si el `Makefile` o un wrapper local deberían preservar mejor el entorno de `nix develop`
3. dejar documentado el comando confiable para recompilar `mina-node-web`

Resultado esperado:

- build wasm repetible y menos frágil

Estado:

- `cumplido a nivel de documentación operativa`

Comando confiable identificado:

```bash
nix develop /home/mtrapaglia/mina/mina-rust \
  -c env CARGO_BUILD_JOBS=2 \
  PROTOC=/nix/store/mvhwlpfpyy241jw4ansg99q2drkgzw86-protobuf-31.1/bin/protoc \
  nice -n 10 \
  make -C /home/mtrapaglia/mina/mina-rust build-wasm
```

Notas:

- el fallo previo no era del código de Mina sino del entorno anidado que perdía `protoc`
- por ahora alcanza con documentar el comando confiable; no hace falta tocar `Makefile` todavía

### 4. Revisar warnings específicos de wasm

Meta:

- separar warnings esperables de warnings que indiquen riesgo real

Foco inicial:

- warning de `-Ctarget-feature=atomics`
- warning de carga ES module en Node

Trabajo:

1. confirmar si `atomics` es requisito esperado del target web de este repo
2. distinguir warning de tooling local vs warning del runtime objetivo
3. decidir si alguno requiere cambio inmediato o solo documentación

Resultado esperado:

- menos ruido al interpretar fallos reales del target wasm

Estado:

- `revisado`

Hallazgos:

1. el warning de `-Ctarget-feature=atomics` aparece en build, pero está alineado con `/home/mtrapaglia/mina/mina-rust/.cargo/config.toml`
2. ese target configura explícitamente `+atomics`, `--shared-memory` e `--import-memory` para multithreading wasm
3. el warning `MODULE_TYPELESS_PACKAGE_JSON` apareció solo en el smoke con Node
4. ese warning es de tooling local de Node y no del runtime objetivo del web node

Conclusión:

- `atomics` hoy es parte esperada del target wasm de este repo
- el warning de Node no justifica cambios en el código del proyecto

### 5. Elegir el primer frente de implementación

Meta:

- decidir dónde empezar a trabajar una vez validado el runtime web

Opciones con más sentido:

- `crates/node/web`
- `crates/node/common`
- integración `p2p`/`WebRTC`

Criterio:

- priorizar algo que pueda compararse contra `native`
- evitar áreas que hoy ya están contaminadas por fallos de fixtures o tests no verdes del nodo base

Estado:

- `pendiente, pero con foco más claro`

Siguiente foco recomendado:

1. validar `mina-node-web` en un browser interactivo real o en el frontend del repo, no en Node
2. entrar por `crates/node/web` y `crates/node/common`
3. dejar `p2p`/`WebRTC` para después de confirmar carga e init del runtime web

## Orden sugerido de ejecución

1. smoke real en navegador
2. prueba mínima comparable `native` vs `wasm`
3. estabilización del comando de build wasm
4. revisión de warnings específicos
5. elección del primer objetivo de código

## Criterio de éxito

Esta etapa queda bien cerrada si logramos:

- confirmar que `mina-node-web` carga en browser
- tener al menos una prueba funcional comparable entre `native` y `wasm`
- dejar un flujo wasm reproducible
- documentar claramente las primeras divergencias reales

## Estado de cierre de esta ejecución

- `flujo wasm reproducible`: sí
- `primera divergencia real native vs wasm`: sí
- `carga confirmada en browser`: no, por limitación del entorno de smoke headless local
- `siguiente paso recomendado`: usar browser real o el frontend del repo para la validación web
