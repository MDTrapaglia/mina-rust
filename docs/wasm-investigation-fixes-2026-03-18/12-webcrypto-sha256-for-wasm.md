# Fix 12: usar WebCrypto para SHA-256 en wasm

Fuente: `../wasm-investigation-conclusions-2026-03-18.md`, Issue 4

Clasificación: `fix real`
Estado real: `implementado`
Branch / commit: `codex/wasm-known-fixes`, `3e7db1033`

## Archivos

- `crates/ledger/src/proofs/verifiers.rs`
- `crates/ledger/Cargo.toml`
- `Cargo.lock`

## Problema

`sha2` puro en wasm/browser seguía siendo un cuello de performance para payloads
grandes aun con chunking.

## Cambio

- la ruta wasm prefiere `crypto.subtle.digest('SHA-256', ...)`
- si WebCrypto no está disponible, mantiene fallback al hasher Rust chunked

## Validación

- el código quedó implementado con fallback explícito
- se inició `make build-wasm` sobre `codex/wasm-known-fixes` y la compilación
  alcanzó la cadena `wasm-bindgen` / `js-sys`, pero la corrida fue interrumpida
  antes del resultado final
- la validación de cierre de compilación wasm sigue pendiente

## Dependencias

- depende de `10` como fallback conservador
- depende de APIs del browser expuestas vía `wasm-bindgen` y `js-sys`

## Riesgo / tradeoff

- introduce dependencia explícita en una API del navegador
- puede haber diferencias de disponibilidad según entorno/browser
- si WebCrypto falla, el sistema vuelve al camino Rust y recupera el costo previo

## Alternativa descartada

Seguir afinando sólo `sha2` en Rust. Se descartó como solución principal porque
el browser ya ofrece una implementación nativa de SHA-256 mejor alineada al caso
de uso.
