# Fallos de tests del nodo

Fecha:

- `2026-03-17`

Entorno:

- repo: `/home/mtrapaglia/mina/mina-rust`
- ejecución dentro de `nix develop`
- comando: `cargo test -p mina-node --lib`

## Resumen

- total: 16
- pasaron: 11
- fallaron: 5

Log completo:

- `/home/mtrapaglia/mina/logs/test-mina-node.log`

## Tests fallidos

### 1. `daemon_json::test::test_daemon_json_read`

Archivo:

- `/home/mtrapaglia/mina/mina-rust/crates/node/src/daemon_json/mod.rs`

Línea relevante:

- abre `testing/data/daemon.json`

Fallo observado:

- `No such file or directory`

Detalle:

- el test abre una ruta relativa:
  `testing/data/daemon.json`
- desde el crate `crates/node` ese archivo no existe

Referencia:

- [mod.rs](/home/mtrapaglia/mina/mina-rust/crates/node/src/daemon_json/mod.rs#L78)

## 2. `ledger::tests::test_complete_with_empties`

Archivo:

- `/home/mtrapaglia/mina/mina-rust/crates/node/src/ledger/mod.rs`

Fallo observado:

- `assert_eq!` falló por hash esperado distinto del real

Valores:

- esperado: `jwxdRe86RJV99CZbxZzb4JoDwEnvNQbc6Ha8iPx7pr3FxYpjHBG`
- real: `jxhPaMCuPGE2hyKxfnid2o5SqDiofw9ktW9t2PW3B1LXSXLEFkf`

Referencia:

- [mod.rs](/home/mtrapaglia/mina/mina-rust/crates/node/src/ledger/mod.rs#L118)

## 3. `ledger::tests::test_complete_with_empties_with_num_accounts`

Archivo:

- `/home/mtrapaglia/mina/mina-rust/crates/node/src/ledger/mod.rs`

Fallo observado:

- `assert_eq!` falló por hash esperado distinto del real

Valores:

- esperado: `jwxdRe86RJV99CZbxZzb4JoDwEnvNQbc6Ha8iPx7pr3FxYpjHBG`
- real: `jxhPaMCuPGE2hyKxfnid2o5SqDiofw9ktW9t2PW3B1LXSXLEFkf`

Referencia:

- [mod.rs](/home/mtrapaglia/mina/mina-rust/crates/node/src/ledger/mod.rs#L134)

## 4. `rpc::heartbeat::tests::test_heartbeat_signing`

Archivo:

- `/home/mtrapaglia/mina/mina-rust/crates/node/src/rpc/heartbeat.rs`

Fallo observado:

- el `payload` firmado no coincide con el string esperado hardcodeado

Lectura inicial:

- parece una expectation desactualizada de serialización/base64
- también podría haber cambiado el shape del objeto serializado

Referencia:

- [heartbeat.rs](/home/mtrapaglia/mina/mina-rust/crates/node/src/rpc/heartbeat.rs#L225)

## 5. `transaction_pool::transaction_pool_state::tests::test_replay_pool`

Archivo:

- `/home/mtrapaglia/mina/mina-rust/crates/node/src/transaction_pool/transaction_pool_state.rs`

Fallo observado:

- `No such file or directory`

Detalle:

- el test intenta leer `/tmp/pool.bin`
- eso introduce una dependencia externa al repo

Referencia:

- [transaction_pool_state.rs](/home/mtrapaglia/mina/mina-rust/crates/node/src/transaction_pool/transaction_pool_state.rs#L141)

## Clasificación rápida

Fixtures o archivos faltantes:

- `test_daemon_json_read`
- `test_replay_pool`

Expectations o snapshots posiblemente desactualizados:

- `test_complete_with_empties`
- `test_complete_with_empties_with_num_accounts`
- `test_heartbeat_signing`

## Próximo orden sugerido

1. resolver los dos tests con archivos faltantes
2. verificar si los hashes de `ledger` cambiaron por cambio legítimo
3. verificar si el payload esperado de `heartbeat` quedó viejo
