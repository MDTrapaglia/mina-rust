# Fix 13: evitar rehashear payloads grandes durante el bootstrap

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `mitigación`
Estado real: `propuesto`
Branch / commit: `sin commit en codex/wasm-known-fixes`

## Problema

Rehashear varios megabytes en el bootstrap del browser vuelve frágil una etapa
del arranque donde cualquier latencia extra impacta directo en UX y timeouts.

## Cambio propuesto

- evitar rehash completo cuando el origen del asset ya esté validado por otra vía

## Validación

- no implementado
- requiere definición previa del modelo de confianza aceptable

## Dependencias

- depende de una decisión explícita sobre trust model y cadena de validación
- complementa, no reemplaza, `03`, `05`, `10` y `12`

## Riesgo / tradeoff

- si la validación alternativa es más débil de lo supuesto, se pierde integridad
- reduce costo de bootstrap a cambio de mover la confianza a otra capa

## Alternativa descartada

Eliminar toda validación en browser. Se descartó porque cambia demasiado el
modelo de seguridad.
