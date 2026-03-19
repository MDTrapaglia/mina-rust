# Foco de testing para la implementación liviana `wasm`

## Hipótesis de trabajo

Sí: la superficie más parecida a una implementación liviana/móvil en este repo
es la variante web `wasm`, no `node/native`.

La referencia concreta es:

- crate: `/home/mtrapaglia/mina/mina-rust/crates/node/web`
- package: `mina-node-web`

Base técnica:

- `crates/node/web` compila a `wasm32-unknown-unknown`
- expone un nodo para correr en entorno web
- usa `p2p-webrtc` por default
- `crates/node/common` comparte lógica con la variante nativa

## Objetivo

Pasar de “sabemos que existe `mina-node-web`” a “tenemos una estrategia concreta
para validar la variante wasm/web como base liviana para dispositivos móviles”.

## Principio práctico

Para móvil no conviene empezar por “levantar todo en un teléfono”.
Conviene validar en este orden:

1. build wasm
2. smoke test del artefacto generado
3. validación funcional mínima en navegador
4. validación WebRTC/P2P
5. recién después constraints reales de móvil

## 1. Preparar toolchain wasm

Desde el repo:

```bash
cd /home/mtrapaglia/mina/mina-rust
make setup-wasm
```

Esto instala:

- nightly requerido por el repo para wasm
- target `wasm32-unknown-unknown`
- `wasm-bindgen-cli`

## 2. Validar build de `mina-node-web`

Comando base:

```bash
make build-wasm
```

Qué hace:

- compila `crates/node/web` para `wasm32-unknown-unknown`
- genera bindings web con `wasm-bindgen`
- deja artefactos en `pkg/`

Criterio de éxito:

- existe el `.wasm`
- existen los bindings JS generados
- no hay errores de target wasm ni de `wasm-bindgen`

## 3. Inspeccionar la API wasm mínima

Archivo clave:

- `/home/mtrapaglia/mina/mina-rust/crates/node/web/src/lib.rs`

Superficie mínima a validar:

- `build_env()`
- `run(...)`

Preguntas a responder:

1. ¿el módulo carga correctamente en navegador?
2. ¿`build_env()` responde?
3. ¿`run(...)` devuelve interfaz RPC usable?
4. ¿el nodo wasm arranca sin `block_producer`?

## 4. Hacer smoke test de navegador

Primer objetivo:

- cargar el bundle wasm en un navegador de escritorio

Motivo:

- es más barato y más observable que saltar directo a móvil
- si falla en desktop browser, móvil no agrega señal útil

Smoke test sugerido:

1. servir el directorio generado `pkg/`
2. cargar el módulo wasm en una página mínima
3. llamar `build_env()`
4. llamar `run(null, seed_nodes_urls, seed_nodes_addresses, null)`
5. confirmar que no rompe al inicializar tracing, workers y RPC

## 5. Foco específico de testing funcional

### A. Arranque del nodo wasm

Validar:

- carga del módulo
- inicialización de panic hook
- inicialización de tracing
- `init_rayon()`
- creación del `NodeBuilder`

### B. Networking WebRTC

Validar:

- parseo de peers con multiaddr WebRTC
- conexión a seed nodes por URL
- conexión a peers por address
- fallos de fetch y logging asociado

Docs relevantes:

- `/home/mtrapaglia/mina/mina-rust/website/docs/developers/webrtc.md`
- `/home/mtrapaglia/mina/mina-rust/website/docs/developers/testing/p2p-tests.md`

### C. RPC mínima

Validar:

- que `run(...)` devuelva `RpcSender`
- operaciones de lectura básicas primero
- evitar comenzar por flujos pesados o block production

### D. Compatibilidad de recursos

Medir al menos:

- tiempo de carga del wasm
- consumo de memoria durante arranque
- estabilidad del worker

## 6. Orden sugerido de tests

### Etapa 1: compilación

```bash
make setup-wasm
make build-wasm
```

### Etapa 2: smoke test local

- cargar wasm en navegador desktop
- probar `build_env()`
- probar `run(...)` sin producción de bloques

### Etapa 3: WebRTC básico

- usar peers de prueba
- validar fetch de seed URLs
- validar parseo de direcciones `/webrtc/.../p2p/...`

### Etapa 4: escenarios

Tomar como referencia testing/scenarios con `p2p-webrtc`:

```bash
cargo run --release --features scenario-generators,p2p-webrtc \
  --bin mina-node-testing -- scenarios-generate --name p2p-signaling --output=json
```

Esto no reemplaza el test wasm en navegador, pero sí ayuda a validar la parte
de signaling/WebRTC del stack.

## 7. Qué no haría al principio

- no empezaría por `mina-node-native`
- no empezaría por tests de performance móvil real
- no empezaría por block production en wasm
- no asumiría que “browser = mobile listo”

## 8. Riesgos actuales

1. El repo no quedó todavía validado con una suite wasm local en esta máquina.
2. La suite `mina-node` base ya mostró tests rotos en el árbol actual.
3. WebRTC y browser workers pueden introducir problemas distintos a la variante nativa.
4. Móvil real probablemente agregue restricciones de memoria, backgrounding y red.

## 9. Próximo paso concreto

Si el objetivo es ir por la implementación liviana, el siguiente paso razonable
no es tocar `node/native`, sino:

1. correr `make setup-wasm`
2. correr `make build-wasm`
3. inspeccionar `pkg/`
4. preparar un smoke test mínimo de navegador para `mina-node-web`
