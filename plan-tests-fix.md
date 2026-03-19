# Mina Core Test Fix Plan

## Branch

- `codex/mina-core-tests-fix`

## Goal

- Reproducir, diagnosticar y corregir los tests fallando en `mina-core`.

## Scope

- Crate principal: `crates/core`
- Comando base de reproducción: `cargo test -p mina-core`

## Plan

1. Inspeccionar el estado actual del repo y preservar cambios locales existentes.
2. Reproducir los fallos de `mina-core` en entorno `nix develop`.
3. Aislar la causa raíz por test o grupo de tests.
4. Aplicar fixes mínimos y revisar impacto lateral.
5. Re-ejecutar los tests relevantes y registrar resultados.

## Progress Log

- 2026-03-19: Branch `codex/mina-core-tests-fix` creado desde el estado actual del repo.
- 2026-03-19: Archivo de plan inicial creado.
- 2026-03-19: Reproducción iniciada con `nix develop . -c cargo test -p mina-core`.
- 2026-03-19: Inspección rápida del crate: `mina-core` expone 7 tests unitarios visibles y varios doctests; si `cargo test -p mina-core` falla, la causa puede estar en documentación ejecutable o en setup compartido.
- 2026-03-19: Durante la reproducción, la build entró en recompilación amplia del workspace; revisión paralela de tests visibles en `chain_id`, `consensus` y `snark_job_id`.
- 2026-03-19: Confirmado que `crates/core` no tiene tests de integración dedicados en `crates/core/tests`; el foco de reproducción queda en unit tests y doctests del propio crate.
- 2026-03-19: La recompilación ya atravesó dependencias pesadas del stack (`poseidon`, `mina-hasher`, `mina-macros`); pendiente el primer resultado ejecutable de `cargo test -p mina-core`.
- 2026-03-19: Reproducción completada con fallo real en `cargo test -p mina-core`.
- 2026-03-19: Fallan `consensus::tests::long_range_fork` y `consensus::tests::short_range_fork` por deserialización de fixture: falta el campo `curr_global_slot_since_hard_fork` en JSON (`Error("missing field \`curr_global_slot_since_hard_fork\`", line: 168, column: 5)`).
- 2026-03-19: Causa raíz aislada: los fixtures históricos usan `curr_global_slot`, mientras que el tipo actual `ConsensusProofOfStakeDataConsensusStateValueStableV2` exige `curr_global_slot_since_hard_fork`.
- 2026-03-19: Fix elegido: agregar alias `#[serde(alias = "curr_global_slot")]` al campo nuevo para recuperar compatibilidad con fixtures y JSON legados sin reescribir datasets.
- 2026-03-19: Revalidación posterior reveló un segundo quiebre del mismo origen: falta `grace_period_slots` en `constants` dentro de los mismos fixtures históricos.
- 2026-03-19: Ajuste complementario: `MinaBaseProtocolConstantsCheckedValueStableV1.grace_period_slots` pasa a aceptar ausencia en JSON usando el valor por defecto del tipo.
- 2026-03-19: Validación final `nix develop . -c cargo test -p mina-core` exitosa.
- 2026-03-19: Resultado final: `7` unit tests ok, `10` doctests ok, `2` doctests ignored, `0` fallos en `mina-core`.
- 2026-03-19: Documento resumen agregado en `mina-core-tests-fixes.md`.

## Notes

- El worktree ya tenía cambios locales previos; no se van a revertir.
