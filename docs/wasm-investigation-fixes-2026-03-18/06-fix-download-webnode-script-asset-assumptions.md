# Fix 06: corregir supuestos de `download-webnode.sh`

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 2

Clasificación: `hardening`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `e5e03d93c`

## Archivos

- `frontend/scripts/download-webnode.sh`

## Problema

El script asumía que los `.postcard` estaban publicados en el release y además
aceptaba descargas sin `curl -f`, lo que dejaba pasar `404` como archivos
descargados.

## Cambio

- usa `curl -fsSL`
- deja de bajar `.postcard` desde el release
- los genera localmente según la red seleccionada
- valida existencia y tamaño no vacío de los assets esperados

## Validación

- `bash -n frontend/scripts/download-webnode.sh` pasó
- el script quedó limitado explícitamente a versiones conocidas

## Dependencias

- usa la herramienta de `05`

## Riesgo / tradeoff

- el script es más opinado y soporta explícitamente menos combinaciones de
  versión/red
- si cambia la convención de nombres, habrá que actualizarlo

## Alternativa descartada

Dejar el script igual y documentar el paso manual. Se descartó porque la fuente
del error seguiría viva en automatizaciones.
