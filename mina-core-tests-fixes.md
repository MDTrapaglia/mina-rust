# Mina Core Test Fixes

## Context

Se reprodujo `cargo test -p mina-core` dentro de `nix develop` y fallaban dos tests:

- `consensus::tests::long_range_fork`
- `consensus::tests::short_range_fork`

La causa no era la lógica de consenso en sí, sino incompatibilidad entre fixtures JSON históricos y los tipos actuales de `mina-p2p-messages`.

## Fix 1: compatibilidad con `curr_global_slot`

### Problema

Los fixtures en `tests/files/forks/*.json` usan el campo:

- `curr_global_slot`

Pero el tipo actual `ConsensusProofOfStakeDataConsensusStateValueStableV2` exige:

- `curr_global_slot_since_hard_fork`

Eso hacía fallar la deserialización antes de ejecutar la lógica del test.

### Solución

Se agregó compatibilidad backward con `serde`:

- archivo: `crates/p2p-messages/src/v2/generated.rs`
- cambio: `#[serde(alias = "curr_global_slot")]`

### Motivo

La solución preserva compatibilidad con fixtures y JSON legados sin reescribir datasets históricos ni duplicar lógica en tests.

## Fix 2: default para `grace_period_slots`

### Problema

Después del primer fix, los mismos fixtures seguían fallando porque en `constants` no incluyen:

- `grace_period_slots`

El tipo actual `MinaBaseProtocolConstantsCheckedValueStableV1` lo considera obligatorio.

### Solución

Se hizo tolerante la deserialización humana cuando el campo falta:

- archivo: `crates/p2p-messages/src/v2/generated.rs`
- cambio: `#[serde(default = "MinaBaseProtocolConstantsCheckedValueStableV1::default_grace_period_slots")]`
- archivo: `crates/p2p-messages/src/v2/manual.rs`
- cambio: helper público `default_grace_period_slots()`

### Motivo

Esto permite cargar fixtures anteriores usando el default ya definido por el tipo, evitando migraciones manuales de JSON que no aportan señal al test.

## Archivos cambiados

- `crates/p2p-messages/src/v2/generated.rs`
- `crates/p2p-messages/src/v2/manual.rs`
- `plan-tests-fix.md`
- `mina-core-tests-fixes.md`

## Validación

Comando ejecutado:

```bash
nix develop . -c cargo test -p mina-core
```

Resultado:

- `7` unit tests ok
- `10` doctests ok
- `2` doctests ignored
- `0` fallos

## Criterio de diseño

Se eligió compatibilidad backward en deserialización en lugar de editar fixtures, porque:

- los datos legados ya existen en más de un lugar del repo
- el problema es de formato, no de semántica del test
- el cambio mantiene bajo el costo de mantenimiento de assets históricos
