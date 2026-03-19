# Wasm Webnode Frontend Plan

Fecha: 2026-03-19
Branch: codex/wasm-frontend-plan
Worktree: /home/mtrapaglia/mina/mina-rust-runtime-repro-firefox

## Objetivo

Construir un frontend visible y funcional para observar el nodo wasm en
Firefox, con historico persistido para analisis posterior.

## Alcance

- UI para estado en vivo (run, sync, peers, best_tip, recursos).
- Almacenamiento historico local.
- Export de datos para analisis.

Fuera de alcance: feature parity con dashboards completos o trazas privadas.

## Propuesta tecnica

### Frontend

- Base: pagina dedicada bajo `frontend/` o `wasm-smoke/`.
- UI: panel con tarjetas (run, sync, peers, best_tip, resources).
- Grilla con timeline (cada poll) y tabla de peers.
- Indicadores de salud (OK/Warn/Fail) con reglas simples.

### Captura de datos

- Polling a `rpcStatus()` cada 5s (configurable).
- Event log local con:
  - timestamp
  - sync status
  - best_tip height/hash
  - peers count + summary
  - resources/p2p queues
- Snapshot inicial de `buildEnv`, `browser`, `memory`.

### Persistencia

- IndexedDB como default (sin backend).
- Esquema:
  - `runs`: metadata (start/end, version, seed config)
  - `polls`: eventos por run (ts, status snapshot)
- Export: JSON (full) + CSV (resumen).

### API local opcional

- Endpoint local en el harness para exportar archivos.
- Sin servidor remoto.

## Entregables

1. UI inicial con polling + render de estado.
2. Persistencia en IndexedDB con historial por run.
3. Export JSON/CSV.
4. Documentacion de uso y parametros.

## Plan de trabajo

1. Definir modelo de datos y estructura de UI.
2. Implementar polling + render en vivo.
3. Implementar IndexedDB (create/open/write/compact).
4. Agregar export y filtros basicos.
5. Documentar y checklist de QA (Firefox).

## Criterios de exito

- UI visible en Firefox con polling estable.
- Historico persistido entre sesiones.
- Export reproducible para debugging.

## Riesgos

- Crecimiento de DB (limpieza/retencion).
- Impacto de polling en performance.
- Diferencias de compatibilidad entre browsers.
