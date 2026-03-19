# Fixes separados de `wasm-investigation-conclusions-2026-03-18.md`

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`

Este directorio convierte el documento largo de investigación en fichas cortas
por fix. Cada ficha ahora incluye:

- clasificación
- estado real
- branch/commit cuando aplica
- validación
- dependencias
- riesgo/tradeoff

## Leyenda

- `fix real`: cambio de código o script orientado a producto
- `hardening`: endurecimiento o guardas operativas
- `mitigación`: reduce impacto, no elimina necesariamente la causa raíz
- `diagnóstico`: ayuda a aislar el problema, no corrige comportamiento final

## Índice

| #   | Archivo                                                                                                                        | Tipo             | Estado real                 | Commit                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------ | ---------------- | --------------------------- | ------------------------ |
| 01  | [01-monotonic-time-per-worker.md](./01-monotonic-time-per-worker.md)                                                           | fix real         | implementado                | `6ef07ca9e`              |
| 02  | [02-document-no-cross-worker-monotonic-reference.md](./02-document-no-cross-worker-monotonic-reference.md)                     | hardening        | parcial                     | `6ef07ca9e`              |
| 03  | [03-validate-resp-ok-in-wasm-fetch.md](./03-validate-resp-ok-in-wasm-fetch.md)                                                 | fix real         | implementado                | `fbad99a22`              |
| 04  | [04-trace-http-response-status.md](./04-trace-http-response-status.md)                                                         | diagnóstico      | no implementado             | -                        |
| 05  | [05-generate-missing-postcard-assets.md](./05-generate-missing-postcard-assets.md)                                             | fix real         | implementado                | `4f98d3342`, `e5e03d93c` |
| 06  | [06-fix-download-webnode-script-asset-assumptions.md](./06-fix-download-webnode-script-asset-assumptions.md)                   | hardening        | implementado                | `e5e03d93c`              |
| 07  | [07-fail-fast-when-postcard-asset-is-missing.md](./07-fail-fast-when-postcard-asset-is-missing.md)                             | hardening        | parcial                     | `fbad99a22`, `e5e03d93c` |
| 08  | [08-remove-sync-xhr-tracing-in-workers.md](./08-remove-sync-xhr-tracing-in-workers.md)                                         | diagnóstico      | no aplica al branch         | -                        |
| 09  | [09-keep-wasm-tracing-non-blocking.md](./09-keep-wasm-tracing-non-blocking.md)                                                 | diagnóstico      | no aplica al branch         | -                        |
| 10  | [10-chunked-sha256-payload-hashing.md](./10-chunked-sha256-payload-hashing.md)                                                 | mitigación       | implementado                | `698265fdb`              |
| 11  | [11-wasm-specific-small-hash-chunk-size.md](./11-wasm-specific-small-hash-chunk-size.md)                                       | mitigación       | implementado                | `698265fdb`              |
| 12  | [12-webcrypto-sha256-for-wasm.md](./12-webcrypto-sha256-for-wasm.md)                                                           | fix real         | implementado                | `3e7db1033`              |
| 13  | [13-avoid-rehashing-large-payloads-during-bootstrap.md](./13-avoid-rehashing-large-payloads-during-bootstrap.md)               | mitigación       | propuesto                   | -                        |
| 14  | [14-move-payload-validation-out-of-critical-bootstrap-path.md](./14-move-payload-validation-out-of-critical-bootstrap-path.md) | mitigación       | propuesto                   | -                        |
| 15  | [15-firefox-smoke-to-isolate-browser-specific-pathology.md](./15-firefox-smoke-to-isolate-browser-specific-pathology.md)       | diagnóstico      | validado                    | -                        |
| 16  | [16-firefox-target-keep-vs-optional.md](./16-firefox-target-keep-vs-optional.md)                                               | nota de decisión | derivado de evidencia nueva | -                        |
| 17  | [17-force-explicit-worker-shim-for-firefox-smoke-bringup.md](./17-force-explicit-worker-shim-for-firefox-smoke-bringup.md)     | diagnóstico      | retirado tras revalidación  | `757dba46a`              |

## Notas

- Los commits referencian `codex/wasm-known-fixes` en
  `/home/mtrapaglia/mina/mina-rust-known-fixes`.
- `04`, `08`, `09`, `15` y `17` son principalmente de observabilidad/diagnóstico.
- `13` y `14` siguen siendo decisiones de producto/arquitectura, no cambios
  cerrados de implementación.
- `16` no introduce un fix nuevo: documenta qué subset seguiría siendo necesario
  si el runtime objetivo fuera Firefox.
- `17` quedó como diagnóstico histórico: sirvió para instrumentar una etapa del
  smoke Firefox, pero la revalidación posterior mostró que el builder default
  también resuelve sin ese override.

## Validación ejecutada

Corridas ejecutadas sobre `codex/wasm-known-fixes` en
`/home/mtrapaglia/mina/mina-rust-known-fixes`:

| Comando                                                            | Resultado  | Tests | Pass          | Fail          |
| ------------------------------------------------------------------ | ---------- | ----- | ------------- | ------------- |
| `cargo test -p redux --lib`                                        | ok         | `0`   | `n/a`         | `n/a`         |
| `cargo test -p mina-core --lib`                                    | con fallas | `7`   | `5` (`71.4%`) | `2` (`28.6%`) |
| `cargo test -p mina-tree chunked_sha256_matches_full_sha256 --lib` | ok         | `1`   | `1` (`100%`)  | `0` (`0%`)    |

Resumen agregado de tests efectivamente ejecutados con casos:

- total: `8`
- pass: `6` (`75.0%`)
- fail: `2` (`25.0%`)

Notas de interpretación:

- `redux` compiló y ejecutó su target de tests, pero no contiene casos en
  `--lib`, por eso no entra en el porcentaje agregado.
- las 2 fallas observadas quedaron en `mina-core`:
  `consensus::tests::long_range_fork` y `consensus::tests::short_range_fork`
- no se ejecutó la suite completa del workspace
- validación adicional wasm/browser:
  - `PROTOC=/tmp/protoc-34/bin/protoc make build-wasm`: `ok`
  - Firefox headless smoke `run(null, [], [], null)`: `resolved`
  - repetición Firefox headless: `5/5` `resolved`, `0/5` `timeout`
  - Firefox headless smoke sin override explícito del worker: `5/5`
    `resolved`, `0/5` `timeout`
