# Fix 15: correr smoke en Firefox para aislar patología específica de browser

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `diagnóstico`
Estado real: `validado`
Branch: `codex/wasm-known-fixes`

## Problema

Todavía puede quedar duda entre un problema general de wasm/browser y un
comportamiento más específico de Chromium.

## Cambio ejecutado

- se corrió el smoke real en Firefox headless
- luego se recompiló `mina-node-web` desde la rama basada en known fixes
- durante una etapa intermedia se probó un override explícito del worker, luego
  retirado al comprobar que no era necesario para corrección

## Resultado observado

- Firefox dejó de ser sólo control negativo:
  - `run(null, [], [], null)` resolvió en ~`9s`
  - `rpcStatus()` respondió dentro del smoke
- repetibilidad inicial:
  - `5/5` corridas `resolved`
  - `0/5` timeouts
  - `run()` entre ~`8.7s` y `9.0s`
- el estado observado del nodo fue:
  - `transition_frontier.sync.status = "Idle"`
  - `transition_frontier.sync.phase = "Bootstrap"`

Artefactos útiles:

- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-smoke-server-v3.log`
- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-default-builder-repeat-v1.json`
- `/home/mtrapaglia/mina/logs/firefox-webnode-known-fixes-default-builder-server-v1.log`

## Validación

- el smoke en Firefox sí separó hipótesis
- además mostró que Firefox puede usarse como baseline más robusto para el
  bring-up del web node
- la revalidación posterior mostró que Firefox también pasa con el builder
  default, sin necesitar el override documentado transitoriamente en `17`

## Dependencias

- ninguna estricta
- idealmente se corre después de `03`, `05`, `10` y `12` para observar el camino
  más normal posible

## Riesgo / tradeoff

- consume tiempo de diagnóstico y puede sumar variabilidad de entorno
- no prueba por sí solo que todo el soporte browser quede resuelto
- aun con Firefox estable en este smoke, sigue faltando explicar por completo
  la diferencia frente al frente Chromium/headless

## Alternativa descartada

Seguir iterando sólo con Chromium. Se descartó como estrategia única porque deja
sin aislar la dimensión browser-specific.
