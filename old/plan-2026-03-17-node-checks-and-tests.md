# Próximos pasos en Mina Rust

Contexto actual:
- repo disponible en `/home/mtrapaglia/mina/mina-rust`
- build debug validado para `mina-cli`
- binario disponible en `/home/mtrapaglia/mina/mina-rust/target/debug/mina`
- plan anterior archivado en `old/`

## Objetivo

Pasar de "compila localmente" a "empezar a trabajar sobre el nodo con una validación real de binario, checks y tests".

## Log de progreso

- 2026-03-17: se archivó el plan anterior en `/home/mtrapaglia/mina/old/plan-2026-03-17-build-setup.md`.
- 2026-03-17: se creó este nuevo `plan.md` para los siguientes pasos.
- 2026-03-17: se eligió `node` como foco inicial de trabajo.
- 2026-03-17: se validó `./target/debug/mina build-info` dentro de `nix develop`.
- 2026-03-17: `cargo check -p mina-node` pasó correctamente.
- 2026-03-17: `cargo check -p mina-node-native` pasó correctamente.
- 2026-03-17: `cargo test -p mina-node --lib` terminó con fallos: 11 tests ok, 5 fallidos.
- 2026-03-17: `cargo test -p mina-node-native --all-features --tests` se inició, pero se interrumpió para no seguir cargando la máquina durante una compilación larga en perfil `test`.

## Estado actual

Resultado corto:
- el binario `mina` responde
- el área `node` compila
- el área `node` no está verde en tests

Conclusión práctica:
- sí podés empezar a trabajar en `node`
- no conviene asumir que el estado actual del repo pasa toda la suite local del nodo
- antes de tocar lógica conviene aclarar si los fallos actuales son esperados por fixtures faltantes o si son regresiones reales del árbol

## Logs y comandos usados

Todos los logs quedaron en:

```bash
/home/mtrapaglia/mina/logs
```

Archivos relevantes:

- `mina-build-info.log`
- `check-mina-node.log`
- `check-mina-node-native.log`
- `test-mina-node.log`
- `test-mina-node-list.log`
- `test-mina-node-native.log`

Comandos ejecutados:

```bash
cd /home/mtrapaglia/mina/mina-rust
nix develop
./target/debug/mina build-info
cargo check -p mina-node
cargo check -p mina-node-native
cargo test -p mina-node --lib
cargo test -p mina-node-native --all-features --tests
```

Parámetros operativos usados para bajar el impacto:

```bash
nice -n 10
CARGO_BUILD_JOBS=2
```

## Validación del binario

`build-info` devolvió:

- versión: `v0.18.1-462-gab69eaed8`
- commit: `ab69eaed85fc71859fedbb2c36b2030294084a82`
- branch: `develop`
- rustc: `1.92.0`

## Resultado de checks

### `cargo check -p mina-node`

Estado:
- OK

Observaciones:
- terminó en perfil `dev`
- emitió muchas warnings en `mina-node` y `mina-p2p`
- no bloquea arrancar trabajo, pero indica deuda técnica o código condicionado por features

### `cargo check -p mina-node-native`

Estado:
- OK

Observaciones:
- terminó en perfil `dev`
- `mina-node-native` solo emitió 1 warning propio visible al final del log
- confirmó que el runtime nativo del nodo compila en este entorno

## Resultado de tests

### `cargo test -p mina-node --lib`

Estado:
- FAIL

Resumen:
- 16 tests totales
- 11 pasaron
- 5 fallaron

Fallos detectados:

1. `daemon_json::test::test_daemon_json_read`
   - falla por archivo inexistente (`No such file or directory`)
2. `ledger::tests::test_complete_with_empties`
   - falla por mismatch de hash esperado vs real
3. `ledger::tests::test_complete_with_empties_with_num_accounts`
   - falla por mismatch de hash esperado vs real
4. `rpc::heartbeat::tests::test_heartbeat_signing`
   - falla por diferencia en payload/base64 esperado
5. `transaction_pool::transaction_pool_state::tests::test_replay_pool`
   - falla por archivo inexistente (`No such file or directory`)

Lectura inicial:
- hay al menos dos fallos por fixtures o archivos faltantes
- hay al menos tres fallos funcionales o de snapshots/valores esperados desactualizados

### `cargo test -p mina-node-native --all-features --tests`

Estado:
- iniciado pero no completado

Motivo:
- la compilación en perfil `test` es bastante más pesada que la del crate base
- para no seguir cargando CPU/RAM por demasiado tiempo se prefirió interrumpir esta corrida

Conclusión:
- la parte nativa compila (`cargo check` ok)
- la suite completa de tests nativos queda pendiente si querés una validación más exhaustiva

## 1. Entrar siempre por el entorno aislado

Trabajar desde:

```bash
cd /home/mtrapaglia/mina/mina-rust
nix develop
```

Objetivo:
- evitar conflictos de dependencias
- mantener consistencia con el build ya validado

## 2. Validar el binario compilado

Probar que el binario responde y ver sus comandos:

```bash
./target/debug/mina --help
./target/debug/mina build-info
```

Criterio de éxito:
- el CLI responde sin recompilar nada raro
- `build-info` devuelve metadata útil del binario

## 3. Leer la documentación de arquitectura

Primera lectura recomendada:

```bash
less ARCHITECTURE.md
```

Objetivo:
- entender módulos principales
- ubicar cómo se relacionan `cli`, `node`, `p2p`, `ledger`, `snark`

## 4. Revisar la guía interna del repo

Leer:

```bash
less CLAUDE.md
```

Objetivo:
- entender convenciones del repo
- ver comandos recomendados
- detectar restricciones o flujos especiales de contribución

## 5. Explorar superficies de trabajo reales

Inspección rápida:

```bash
make help
ls crates
ls tools
```

Después elegir un foco inicial entre:

1. `node`
2. `ledger`
3. `p2p`
4. `frontend`
5. `testing`

Regla práctica:
- si querés protocolo y networking, empezar por `p2p` o `node`
- si querés estado, cuentas y transiciones, empezar por `ledger`
- si querés velocidad para aportar algo chico, empezar por `testing`

## 6. Correr chequeos livianos antes de meterte a tocar código

Como primera pasada no destructiva:

```bash
cargo check -p mina-cli
cargo check -p mina-node
```

Opcional después:

```bash
make check
```

Objetivo:
- confirmar que el árbol sigue sano para más de un crate
- detectar rápido si hay diferencias entre compilar y chequear

## 7. Elegir un flujo de ejecución

Tenés dos caminos útiles:

### Camino A: entender el CLI

```bash
./target/debug/mina build-info
./target/debug/mina misc --help
./target/debug/mina wallet --help
```

### Camino B: acercarte al nodo

Primero preparar binario release:

```bash
make build-release
```

Después explorar arranque:

```bash
./target/release/mina node --help
```

Nota:
- no recomiendo arrancar el nodo completo todavía si antes no definimos qué querés investigar

## 8. Próximas tareas recomendadas en `node`

Orden sugerido desde el estado actual:

1. reproducir y clasificar los 5 fallos de `cargo test -p mina-node --lib`
2. inspeccionar qué fixtures faltan para `daemon_json` y `transaction_pool`
3. revisar por qué cambiaron los hashes esperados en `crates/node/src/ledger/mod.rs`
4. revisar el test de `heartbeat` para ver si cambió la serialización esperada
5. recién después explorar el flujo de arranque del nodo nativo

## 9. Criterio de cierre de este plan

Este plan queda cumplido si:

- podés entrar con `nix develop`
- el binario `mina` responde
- el área `node` compila en `check`
- tenés un mapa claro de qué tests del nodo fallan
- definiste si tu primera tarea va a ser arreglar tests/fixtures o avanzar sobre lógica del nodo

## 10. Próximo paso sugerido

Orden recomendado desde acá:

1. abrir `crates/node/src/daemon_json/mod.rs`
2. abrir `crates/node/src/transaction_pool/transaction_pool_state.rs`
3. abrir `crates/node/src/ledger/mod.rs`
4. abrir `crates/node/src/rpc/heartbeat.rs`
5. decidir si querés que el siguiente trabajo sea:
   - arreglar fixtures faltantes
   - corregir expectations rotas
   - investigar el arranque del nodo nativo
