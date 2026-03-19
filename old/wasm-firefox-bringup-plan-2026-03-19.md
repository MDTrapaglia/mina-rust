# Firefox Webnode Bring-up Plan

Fecha: 2026-03-19
Worktree: /home/mtrapaglia/mina/mina-rust-runtime-repro-firefox
Branch: codex/wasm-runtime-repro-firefox

## Objetivo

Confirmar que el nodo wasm en Firefox es estable mas alla del smoke, y dejar
evidencia reproducible de estabilidad y conectividad sin depender de Chromium.

## Precondiciones

- Firefox instalado en host
- geckodriver disponible en `/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/.tools/geckodriver/geckodriver`
- harness wasm-smoke accesible en `/home/mtrapaglia/mina/wasm-smoke/`
- build wasm actualizado (ver paso 1)

## Plan

1. Build wasm y sync del bundle para el harness.
   - `PROTOC=/tmp/protoc-34/bin/protoc make build-wasm`
   - `rsync -a --delete --exclude '.keep' /home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/pkg/ /home/mtrapaglia/mina/mina-rust/pkg/`

2. Smoke Firefox headless basico con repeticion.
   - levantar `wasm-smoke/server.mjs`
   - correr 10 repeticiones en `http://127.0.0.1:8161/wasm-smoke/index.html?timeout_ms=30000`
   - guardar summary en `logs/firefox-webnode-smoke-headless-v1.json`

3. Smoke Firefox no-headless.
   - ejecutar 3 corridas en modo grafico
   - verificar `run()` y `rpcStatus()` en todas
   - guardar artifacts en `logs/firefox-webnode-smoke-nonheadless-v1.json`

4. Smoke extendido con polling.
   - luego del `run()`, consultar `rpcStatus()` cada 5s por 5 minutos
   - objetivo: detectar cuelgues post-bringup
   - guardar timeline en `logs/firefox-webnode-smoke-polling-v1.json`

5. Conectividad real (si aplica).
   - inyectar seed URL real y/o peers
   - esperar que `rpcStatus().peers` pase de `[]` a algun valor
   - guardar artifacts en `logs/firefox-webnode-peers-v1.json`

6. Resumen final.
   - actualizar `wasm-runtime-repro-plan.md` con resultados
   - si todo es estable, declarar Firefox como baseline y cerrar track Chromium

## Criterios de exito

- headless: `>= 9/10` corridas `resolved`
- non-headless: `>= 2/3` corridas `resolved`
- polling: 5 minutos sin cuelgue ni excepciones
- conectividad: al menos un peer reportado en `rpcStatus()`

## Notas

- mantener el harness estable durante todo el plan para evitar variabilidad
- si falla en non-headless pero no en headless, priorizar diagnostico grafico

## Proximos pasos (actualizado 2026-03-19)

1. Opcional (si hay flakes):
   - correr 20 repeticiones headless adicionales
   - comparar tasas `resolved` vs `runner_timeout` y adjuntar resumen
2. Seguimiento:
   - mantener Chromium fuera del scope (baseline = Firefox)
   - revisar si es necesario mover el harness actualizado al repo principal

## Avances (2026-03-19)

- Polling estable 5 min con Firefox headless:
  `logs/firefox-webnode-smoke-polling-v3.json` (status `resolved`).
- Runner actualizado para esperar `polling.complete` y marcar
  `polling_incomplete` si no cierra en el grace window.
- Non-headless ejecutado con `Xvfb`:
  `logs/firefox-webnode-smoke-nonheadless-v1.json` (3/3 `resolved`).
- Harness extendido para aceptar `seed_nodes_url`, `seed_nodes_address`,
  `genesis_config_url` por query.
- Conectividad real con seeds (devnet webrtc):
  `logs/firefox-webnode-peers-v1.json` con peers observados durante polling
  (24 entradas con peers no vacios).
- Validacion devnet extendida (15 min):
  `logs/firefox-webnode-peers-strong-v1.json` con peers observados durante
  polling (179 entradas con peers no vacios), `sync.status=Synced`.
  `best_tip.height` se mantuvo estable en 296372 durante el intervalo.

## Resumen final (2026-03-19)

- Firefox queda como baseline para wasm webnode.
- Chromium fuera del scope a partir de esta fecha.
- Evidencias:
  - headless: `logs/firefox-webnode-smoke-headless-v1.json` (10/10 resolved)
  - non-headless: `logs/firefox-webnode-smoke-nonheadless-v1.json` (3/3 resolved)
  - polling: `logs/firefox-webnode-smoke-polling-v3.json` (5 min, complete=true)
  - conectividad: `logs/firefox-webnode-peers-v1.json` (peers observados)
  - devnet extendido: `logs/firefox-webnode-peers-strong-v1.json`
