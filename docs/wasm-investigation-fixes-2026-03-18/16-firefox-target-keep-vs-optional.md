# Nota 16: qué fixes seguirían siendo necesarios si el runtime objetivo fuera Firefox

Fecha: `2026-03-19`

Clasificación: `nota de decisión` Estado real: `derivado de evidencia nueva`
Branch / commit base: `codex/wasm-known-fixes`

## Contexto

Después de cerrar el track Firefox en
`/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox`, la evidencia mínima
quedó así:

- mini repro: `20/20` resoluciones en Firefox headless
- timing repro: `20/20` resoluciones en Firefox headless

Artifacts principales de esa validación:

- `/home/mtrapaglia/mina/logs/firefox-track-summary-v2.json`
- `/home/mtrapaglia/mina/logs/firefox-mini-repeat-server-v3.log`
- `/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/wasm-runtime-repro-plan.md`

## Alcance de esta nota

Esta nota no dice qué fixes conviene sacar hoy del branch.

Sólo responde una pregunta más chica:

- si el runtime objetivo fuera Firefox, cuáles de los fixes implementados en
  `codex/wasm-known-fixes` seguirían viéndose como necesarios
- y cuáles pasarían a ser candidatos a simplificación porque hoy parecen más
  ligados al frente Chromium/headless

## Criterio usado

- `mantener`: el fix corrige un bug real de lógica, setup o distribución y no
  depende del browser problemático
- `opcional en Firefox-only`: el fix parece una mitigación del frente
  wasm/browser que hoy quedó más asociado a Chromium/headless que a Firefox
- `ya compartido`: el estado actual del archivo ya coincide entre branches; no
  aparece como discriminante real en esta comparación

## Lectura por fix

| Fix  | Veredicto si el target fuera Firefox            | Motivo                                                                                                      |
| ---- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `01` | `mantener`                                      | corrige el problema de tiempo monotónico por worker; no depende del split Chromium vs Firefox               |
| `02` | `mantener`                                      | hardening/documentación del mismo problema de `01`                                                          |
| `03` | `mantener`                                      | validar `resp.ok()` es corrección de manejo HTTP, no mitigación de browser                                  |
| `05` | `mantener`                                      | generar `.postcard` localmente resuelve distribución de assets, independientemente del browser              |
| `06` | `mantener`                                      | corrige supuestos incorrectos del script de setup                                                           |
| `07` | `mantener`                                      | fail-fast por assets faltantes; sigue siendo deseable aunque Firefox sea estable                            |
| `10` | `opcional en Firefox-only`                      | chunking del hash aparece más como mitigación del frente wasm/browser bajo Chromium headless                |
| `11` | `opcional en Firefox-only`                      | `8 KiB` específico para wasm parece tuning defensivo, no requisito ya demostrado en Firefox                 |
| `12` | `opcional por corrección, útil por performance` | WebCrypto no parece necesario para estabilidad en Firefox, aunque puede seguir siendo conveniente por costo |

## Interpretación

La lectura actual es:

- los fixes de HTTP, assets y setup (`03`, `05`, `06`, `07`) siguen siendo
  necesarios aunque el runtime objetivo fuera Firefox
- el fix de tiempo monotónico (`01`, y su hardening `02`) tampoco debería
  condicionarse al browser porque corrige una clase de bug distinta
- los candidatos reales a simplificación, si existiera una variante
  Firefox-only, serían `10`, `11` y `12`

## Límites de esta conclusión

- esto es una inferencia a partir de repros mínimos, no una validación de
  bootstrap completo del webnode sólo en Firefox
- no prueba que `10`, `11` y `12` sean inútiles en Firefox; sólo que hoy ya no
  aparecen como necesarios para explicar la estabilidad observada en ese browser
- `12` puede seguir justificándose por performance aunque Firefox no lo requiera
  para corrección
- esta nota tampoco cuenta a `17` como fix retenible: la revalidación posterior
  mostró que el override explícito del worker no era necesario para que el
  smoke Firefox resolviera con builder default

## Conclusión práctica

Si el producto sigue soportando Chromium, no hay base suficiente para retirar
`10`, `11` o `12` de este branch.

Si existiera una variante Firefox-only, los únicos candidatos razonables a
revisar o simplificar serían `10`, `11` y `12`.
