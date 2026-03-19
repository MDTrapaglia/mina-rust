# Fix 03: validar `resp.ok()` en el fetch wasm

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 2

Clasificación: `fix real`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `fbad99a22`

## Archivos

- `crates/core/src/http.rs`

## Problema

La ruta wasm consumía el body de una respuesta no exitosa como si fuera payload
válido. Eso mezclaba error HTTP con error de datos.

## Cambio

- se valida `resp.ok()`
- si el status no es exitoso, el fetch falla temprano con error explícito

## Validación

- el commit incluye el `status` en el mensaje de error
- esto elimina el comportamiento previo donde un `404` podía seguir
  procesándose aguas abajo

## Dependencias

- ninguna estricta
- complementa `05`, `06` y `07`

## Riesgo / tradeoff

- el sistema se vuelve más estricto y pierde cualquier fallback implícito que
  antes ocultaba assets faltantes
- errores de distribución quedan expuestos antes, lo cual es bueno para
  diagnóstico pero más duro para entornos rotos

## Alternativa descartada

Dejar fallar la decodificación más abajo. Se descartó porque vuelve opaco el
origen real del problema.
