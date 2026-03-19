# Fix 14: mover la validación del payload fuera del camino crítico

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `mitigación`
Estado real: `propuesto`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Problema

Aunque el digest siga siendo correcto, pagarlo dentro del camino crítico del
bootstrap vuelve el arranque demasiado sensible a performance del navegador.

## Cambio propuesto

- reubicar la validación a una etapa menos crítica del arranque

## Validación

- no implementado
- requeriría smoke funcional y definición de cuándo el nodo se considera
  “suficientemente inicializado”

## Dependencias

- depende de rediseño del bootstrap
- puede convivir con `10`, `11` y `12`

## Riesgo / tradeoff

- el error puede aparecer más tarde en la sesión
- se mejora tiempo de arranque a costa de demorar la detección de corrupción

## Alternativa descartada

Seguir optimizando sólo dentro del bootstrap actual. Se descartó como única
respuesta porque mantiene el costo en la parte más sensible del arranque.
