# Plan de trabajo: `native` y `wasm` en paralelo

## Estado de ejecución

Fecha de avance: `2026-03-17`

Estado general:

- baseline `native`: consolidada
- baseline `wasm`: abierta y compilada
- comparación inicial `native vs wasm`: disponible

Logs nuevos de esta ejecución:

- `/home/mtrapaglia/mina/logs/setup-wasm.log`
- `/home/mtrapaglia/mina/logs/build-wasm.log`
- `/home/mtrapaglia/mina/logs/build-wasm-retry.log`
- `/home/mtrapaglia/mina/logs/wasm-module-smoke.log`

## Objetivo

Mantener disponibles los dos entornos del nodo para poder comparar:

- implementación nativa: `mina-node` / `mina-node-native`
- implementación web/wasm: `mina-node-web`

La idea no es reemplazar una por la otra, sino usar ambas como referencia para:

- detectar diferencias de comportamiento
- aislar fallos específicos de `wasm`
- verificar si un bug existe en ambas variantes o solo en una

## Principio de trabajo

No mezclar “cambiar de foco” con “perder el entorno anterior”.

En este repo tiene más sentido trabajar así:

1. mantener el flujo `native` disponible
2. sumar el flujo `wasm` como segundo eje
3. comparar resultados entre ambos

## Contexto técnico

Superficies relevantes:

- nativo: `/home/mtrapaglia/mina/mina-rust/crates/node/native`
- nodo base: `/home/mtrapaglia/mina/mina-rust/crates/node`
- compartido: `/home/mtrapaglia/mina/mina-rust/crates/node/common`
- wasm/web: `/home/mtrapaglia/mina/mina-rust/crates/node/web`

Lectura del repo:

- `native` y `wasm` no son proyectos separados
- comparten parte importante de la lógica
- la comparación entre ambos tiene valor real

## Qué queremos preservar

### Eje A: entorno nativo

Mantener documentado y disponible:

- `nix develop`
- `cargo check -p mina-node`
- `cargo check -p mina-node-native`
- estado actual de tests del nodo

Objetivo:

- no perder la línea base que ya tenemos para el nodo normal

### Eje B: entorno wasm

Agregar sin romper lo anterior:

- `make setup-wasm`
- `make build-wasm`
- smoke tests del crate `mina-node-web`

Objetivo:

- abrir una segunda línea base para comparar contra `native`

## Estrategia de comparación

Comparar siempre por capas, no todo junto.

### 1. Build

Preguntas:

- ¿compila en `native`?
- ¿compila en `wasm`?
- ¿qué warnings aparecen en cada uno?

### 2. Inicialización

Preguntas:

- ¿el nodo arranca en `native`?
- ¿el módulo wasm carga e inicializa?
- ¿hay diferencias en tracing, workers o setup?

### 3. Networking

Preguntas:

- ¿el flujo `p2p` se comporta igual?
- ¿qué parte depende de `WebRTC` y no existe igual en `native`?
- ¿hay fallos exclusivos del navegador?

### 4. RPC y serialización

Preguntas:

- ¿la misma operación devuelve datos equivalentes?
- ¿hay diferencias de formato, payload o encoding?
- ¿aparece un bug solo en `wasm` aunque la lógica compartida sea la misma?

### 5. Recursos y restricciones

Preguntas:

- ¿qué consume más memoria?
- ¿qué limitaciones impone `wasm`?
- ¿qué cosas del nodo nativo no aplican en browser/mobile?

## Matriz inicial de comparación

| Prueba | Native | Wasm | Diferencia | Hipótesis |
|---|---|---|---|---|
| `cargo check -p mina-node` | ok | n/a | baseline nativa válida | ya verificado antes |
| `cargo check -p mina-node-native` | ok | n/a | baseline runtime nativo válida | ya verificado antes |
| `cargo test -p mina-node --lib` | fail | n/a | `native` no está completamente verde | hay fixtures faltantes y expectations rotas |
| `make setup-wasm` | n/a | ok | requiere nightly + `wasm-bindgen-cli` | setup específico del target web |
| `make build-wasm` | n/a | ok | primera ejecución falló por `protoc` no heredado | la invocación con shell anidada perdía `PATH` del entorno `nix develop` |
| carga de módulo generado | n/a | ok | warning de Node sobre `package.json` sin `type=module` | warning del entorno Node, no del bundle wasm en sí |

## Orden de trabajo sugerido

### Fase 1: consolidar baseline `native`

Usar como línea base lo ya observado:

- el nodo compila
- `mina-node-native` compila
- `cargo test -p mina-node --lib` no está completamente verde

Resultado esperado:

- tener claro qué problemas ya existen antes de empezar a culpar a `wasm`

Estado:

- completada

Evidencia:

- `/home/mtrapaglia/mina/logs/check-mina-node.log`
- `/home/mtrapaglia/mina/logs/check-mina-node-native.log`
- `/home/mtrapaglia/mina/logs/test-mina-node.log`
- `/home/mtrapaglia/mina/node-test-failures.md`

### Fase 2: abrir baseline `wasm`

Sin tocar aún lógica de negocio:

1. preparar toolchain wasm
2. compilar `mina-node-web`
3. verificar artefactos generados
4. definir smoke test mínimo de navegador

Resultado esperado:

- tener una línea base separada para `wasm`

Estado:

- parcialmente completada

Ejecutado:

1. `make setup-wasm`
2. `make build-wasm`
3. verificación de artefactos generados en `/home/mtrapaglia/mina/mina-rust/pkg`
4. smoke mínimo no-browser por import del módulo generado

Resultado real:

- `setup-wasm` quedó ok
- `build-wasm` quedó ok
- artefactos generados:
  - `/home/mtrapaglia/mina/mina-rust/target/wasm32-unknown-unknown/release/mina_node_web.wasm`
  - `/home/mtrapaglia/mina/mina-rust/pkg/mina_node_web.js`
  - `/home/mtrapaglia/mina/mina-rust/pkg/mina_node_web_bg.wasm`
  - `/home/mtrapaglia/mina/mina-rust/pkg/mina_node_web.d.ts`
  - `/home/mtrapaglia/mina/mina-rust/pkg/mina_node_web_bg.wasm.d.ts`
- tamaños observados:
  - wasm release: `32M`
  - wasm bindgen output: `38M`

Observaciones:

- primer intento de build wasm falló porque `protoc` dejó de estar visible al entrar en un `sh -lc` anidado
- `protoc` sí existe dentro de `nix develop`
- el build correcto salió al exportar `PROTOC` explícitamente y evitar depender de esa shell anidada
- queda pendiente el smoke test con navegador real

Logs:

- `/home/mtrapaglia/mina/logs/setup-wasm.log`
- `/home/mtrapaglia/mina/logs/build-wasm.log`
- `/home/mtrapaglia/mina/logs/build-wasm-retry.log`
- `/home/mtrapaglia/mina/logs/wasm-module-smoke.log`

### Fase 3: alinear pruebas comparables

Elegir pruebas que sí tengan sentido en ambos entornos:

- build
- init
- RPC de lectura
- serialización
- networking compatible con WebRTC

No empezar por:

- block production
- performance móvil real
- suites grandes sin baseline clara

Estado:

- iniciada

Lo ya comparable:

- `native`: checks del nodo y del runtime nativo
- `wasm`: build completo del bundle y carga del módulo exportado

Smoke ejecutado en `wasm`:

- import del módulo generado con Node `v24.12.0`
- validación de exports:
  - `build_env`
  - `run`
  - `RpcSender`

Resultado:

- `true,true,true`

Limitación:

- esto valida artefacto y surface API exportada
- no valida aún ejecución en browser, `WebRTC`, ni arranque funcional del nodo wasm

### Fase 4: registrar divergencias

Cada vez que algo falle en `wasm`, clasificar:

1. falla también en `native`
2. pasa en `native` y falla en `wasm`
3. depende de APIs exclusivas del navegador
4. depende de archivos o runtime nativo no disponibles en wasm

Estado:

- iniciada

Divergencias ya observadas:

1. el flujo wasm depende de setup de toolchain adicional
2. el build wasm mostró sensibilidad al entorno de `protoc`
3. apareció warning específico de target wasm sobre `-Ctarget-feature=atomics`
4. el smoke por Node dio warning de `MODULE_TYPELESS_PACKAGE_JSON`

## Ventajas de esta estrategia

- evita perder el contexto del nodo normal
- permite comparar implementaciones con base común
- reduce el riesgo de diagnosticar mal un bug
- separa fallos del repo de fallos específicos de plataforma

## Riesgos a tener presentes

1. `native` no está completamente verde hoy, así que no todo fallo en `wasm` será nuevo.
2. `wasm` tiene restricciones reales de browser que no existen en `native`.
3. algunas pruebas del nodo nativo no van a ser trasladables a `wasm` sin adaptación.

## Documentos relacionados

- contexto general: `/home/mtrapaglia/mina/AGENTS.md`
- fallos actuales del nodo: `/home/mtrapaglia/mina/node-test-failures.md`
- foco wasm: `/home/mtrapaglia/mina/wasm-light-testing-plan.md`

## Próximos pasos sugeridos

1. ejecutar un smoke real en navegador para verificar `init` del bundle wasm
2. definir una prueba pequeña de lectura o RPC comparable entre `native` y `wasm`
3. revisar si el warning de `atomics` es esperado o si requiere ajuste del target/configuración
4. decidir si conviene fijar `PROTOC` en el flujo wasm para evitar falsos fallos de entorno
