# Wasm Known Fixes Test Coverage

Este documento resume la cobertura de tests agregada en `codex/wasm-known-fixes`
para los fixes conocidos investigados en wasm/browser.

## Objetivo

La idea de esta tanda no fue crear una suite artificial para los 15 items del
documento de investigación, sino agregar pruebas con señal real para los fixes
implementados en esta branch y dejar explícito qué parte todavía requiere un
harness wasm/browser más completo.

## Mapa de cobertura

| Fix | Estado del fix en branch | Cobertura agregada | Tipo de prueba | Estado |
| --- | --- | --- | --- | --- |
| `01` tiempo monotónico por worker en wasm | implementado | no se agregó test nuevo | requeriría worker wasm real | pendiente |
| `02` documentar referencia monotónica por worker | parcial | no aplica | documentación | n/a |
| `03` fail-fast en fetch wasm no-ok | implementado | `http::tests::non_ok_fetch_uses_not_found_error_kind` y `http::tests::non_ok_fetch_error_mentions_url_and_status` | unit test Rust | agregado |
| `04` trazar `status` HTTP | no implementado | no aplica | no hay cambio de producto | n/a |
| `05` generar `.postcard` faltantes | implementado | `tests::source_json_selects_expected_embedded_sources`, `tests::blob_bytes_prefixes_source_and_payload_digests`, más harness shell del script | unit test Rust + shell harness | agregado |
| `06` corregir supuestos de `download-webnode.sh` | implementado | harness shell con `berkeley-devnet -> devnet` y `3.0.0mainnet -> mainnet` | shell harness | agregado |
| `07` fail-fast si falta `.postcard` | parcial | harness shell falla si el generador no deja un postcard esperado | shell harness | agregado |
| `08` remover tracing síncrono con XHR | no aplica al branch | no aplica | diagnóstico | n/a |
| `09` mantener tracing wasm no bloqueante | no aplica al branch | no aplica | diagnóstico | n/a |
| `10` chunking de SHA-256 del payload | implementado | `chunked_sha256_matches_full_sha256` | unit test Rust | agregado |
| `11` chunk size chico para wasm/browser | implementado | `wasm_hash_chunk_size_contract_is_8k` | unit test Rust | agregado |
| `12` WebCrypto para SHA-256 en wasm | implementado | `webcrypto_sha256_matches_rust_sha256` | wasm browser test | agregado pero no ejecutado en esta sesión |
| `13` evitar rehash durante bootstrap | no implementado | no aplica | cambio no presente | n/a |
| `14` mover validación fuera del camino crítico | no implementado | no aplica | cambio no presente | n/a |
| `15` smoke Firefox | no implementado | no aplica | diagnóstico | n/a |

## Tests agregados

### `crates/core/src/http.rs`

Fix cubierto: `03`

- `non_ok_fetch_uses_not_found_error_kind`
- `non_ok_fetch_error_mentions_url_and_status`

Qué validan:

- que el fail-fast use `std::io::ErrorKind::NotFound`
- que el mensaje preserve `url` y `status`

Limitación:

- prueban el contrato del error, no un `fetch()` real en browser

### `tools/generate-webnode-circuit-blobs/src/main.rs`

Fix cubierto: `05`

- `source_json_selects_expected_embedded_sources`
- `blob_bytes_prefixes_source_and_payload_digests`

Qué validan:

- que el generador use el source JSON correcto por network
- que el blob generado preserve el layout esperado:
  `source_digest + payload_digest + payload`

### `frontend/scripts/tests/download-webnode.sh`

Fixes cubiertos: `05`, `06`, `07`

Casos cubiertos:

- genera postcards para `berkeley-devnet`
- genera postcards para `3.0.0mainnet`
- falla si el generador no deja `transaction_verifier_index.postcard`

Qué validan:

- que el script ya no dependa de bajar `.postcard` desde release assets
- que el mapeo entre `CIRCUITS_VERSION` y network lógica sea correcto
- que el script falle temprano ante assets generados faltantes

### `crates/ledger/src/proofs/verifiers.rs`

Fixes cubiertos: `10`, `11`, `12`

- `chunked_sha256_matches_full_sha256`
- `wasm_hash_chunk_size_contract_is_8k`
- `webcrypto_sha256_matches_rust_sha256`

Qué validan:

- que el hashing chunked produzca exactamente el mismo digest que el hashing
  completo
- que el contrato del chunk size wasm quede fijado en `8 * 1024`
- que WebCrypto y el hasher Rust produzcan el mismo SHA-256

Limitación:

- el test de WebCrypto está agregado, pero no fue ejecutado en esta sesión

## Validación ejecutada en esta sesión

Corridas realizadas sobre `codex/wasm-known-fixes`:

- `bash frontend/scripts/tests/download-webnode.sh`
- `cargo test -p mina-core http::tests:: --lib`
- `cargo test -p generate-webnode-circuit-blobs`
- `cargo test -p mina-tree verifiers::tests:: --lib`

Resultado observado:

- shell harness: `ok`
- `mina-core` tests nuevos: `2/2` pass
- `generate-webnode-circuit-blobs`: `2/2` pass
- `mina-tree` tests nativos nuevos: `2/2` pass

Totales de esta tanda de tests nuevos ejecutados:

- tests ejecutados: `6`
- pass: `6`
- fail: `0`

## Cobertura pendiente

Lo que todavía no queda cubierto por una prueba ejecutada en esta sesión:

- `01`: un test real de aislamiento entre workers wasm
- `12`: ejecución efectiva del test browser/wasm de WebCrypto

Esos dos puntos requieren un harness wasm/browser real y no sólo unit tests
nativos.
