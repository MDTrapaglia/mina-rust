# Firefox Devnet Validation Plan

Fecha: 2026-03-19
Worktree: /home/mtrapaglia/mina/mina-rust-runtime-repro-firefox
Branch: codex/wasm-runtime-repro-firefox

## Objetivo

Demostrar que el nodo wasm en Firefox no solo inicia y mantiene estabilidad,
sino que valida devnet con progreso observable del `best_tip`.

## Precondiciones

- Firefox instalado
- geckodriver disponible en `/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/.tools/geckodriver/geckodriver`
- harness wasm-smoke accesible en `/home/mtrapaglia/mina/wasm-smoke/`
- build wasm actualizado

## Plan

1. Devnet long-run (30-60 min).
   - usar `seed_nodes_url=https://bootnodes.minaprotocol.com/networks/devnet-webrtc.txt`
   - polling cada 5s con `poll_duration_ms` >= 1_800_000
   - guardar artifacts en `logs/firefox-webnode-peers-strong-v2.json`

2. Validacion de progreso.
   - confirmar `sync.status=Synced` en la mayor parte del intervalo
   - observar al menos un cambio de `best_tip.height` durante el run
   - registrar min/max height en el resumen

3. Estabilidad de peers.
   - contar entradas con `peers` no vacios
   - verificar que no haya caidas prolongadas (ej. > 2 min sin peers)

4. Cierre.
   - actualizar este plan con resultados y dejar evidencia
   - si cumple, declarar nodo wasm devnet funcional

## Criterios de exito

- `polling.complete=true`
- `sync.status=Synced`
- `best_tip.height` cambia al menos una vez
- peers presentes en la mayor parte del intervalo
