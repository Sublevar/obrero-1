# Aviso de pérdidas de ticks y pasos (consola, servidor de dev e indicador)

**Fecha:** 2026-10-02
**Participantes:** stfg.prof + Claude (Sonnet 5.5)
**Alcance:** que el modo debug marque si se pierden ticks o pasos: por consola, por el servidor (dormible) y con un indicador en pantalla.

## Qué se leyó

- `crates/obrero-core/src/{engine,clock,tuning}.rs`, `crates/obrero-wasm/src/{lib,debug}.rs`, `web/src/{main,scheduler,ui}.ts`, `web/package.json`, `web/tsconfig.json`, `specs/clock.md`.

## Decisiones

1. **Qué cuenta como pérdida:** ticks descartados por ring lleno (`lost_ticks`, debe ser 0 con los topes), ticks saltados tras un atraso mayor a `MAX_CATCHUP_US` (`skipped_ticks`), pasos del patrón dentro de esos ticks (`skipped_steps`: pasos que no suenan) y avisos perdidos por el `DebugTap`. El gap del pump (tiempo entre pumps) se muestra como causa probable.
2. **Bug encontrado y corregido:** tras un salto del reloj, el `Engine` seguía con su contador de ticks propio y el patrón perdía la grilla. Ahora, en reloj interno, toma el `tick` del aviso: los pasos salteados no suenan y los siguientes caen donde corresponde. En externo se mantiene el contador propio (Stop/Continue).
3. **"El server" = el servidor de desarrollo de Vite** (única interpretación razonable; el usuario no lo precisó). Un plugin de `vite.config.ts` (solo `serve`) recibe un POST a `/__obrero/loss` y lo imprime en la terminal de vite. En builds de producción no se envía nada.
4. **Dormir los avisos:** botón del panel de debug, `window.obreroLoss.muted` o `?losslog=off`. Dormidos, se silencian consola y servidor; los contadores y el indicador siguen.
5. **Los avisos de consola y servidor funcionan siempre** (con o sin `?clockdebug`); el indicador solo en modo debug. Se agrupan (uno cada 250 ms como máximo) para no inundar.
6. **Sin dependencias nuevas.** `web/src/vite-env.d.ts` (tipos de `vite/client`) para `import.meta.env.DEV`.

## Queda abierto

- **Probar en el navegador provocando un atraso real** (pestaña en segundo plano o `debugger`): la lógica se verificó con tests de host, con `loss_report` en Node y con un POST real al endpoint de dev, no con una pestaña viva.
- Si "el server" era otra cosa (p. ej. un servidor propio de logs), el envío se cambia en `lossmonitor.ts`.
- El panel de debug no distingue qué pasó con las notas ya enviadas por Web MIDI con timestamp (eso lo maneja el navegador).

## Artefactos de esta sesión

`crates/obrero-core/src/{engine,lib}.rs`, `crates/obrero-core/tests/engine.rs`, `crates/obrero-wasm/src/lib.rs`, `web/src/{lossmonitor,main,scheduler,vite-env.d}.ts`, `web/vite.config.ts`, `web/src/pkg/` (regenerado), `specs/clock.md`, `docs/INDEX.md`, este archivo.

## Bitácora de actos

1. **`LossReport` y `tick` sincronizado con el reloj** — archivos: `crates/obrero-core/src/engine.rs`, `crates/obrero-core/src/lib.rs`, `crates/obrero-core/tests/engine.rs`. Por qué: contar ticks y pasos perdidos y no desalinear la grilla tras un salto; test del salto (79 pasos sin sonar, el paso 80 suena en su tick).
2. **`WasmEngine::loss_report`** — archivos: `crates/obrero-wasm/src/lib.rs`. Por qué: entregar los contadores a JS.
3. **`LossMonitor`, indicador y endpoint de dev** — archivos: `web/src/lossmonitor.ts`, `web/src/main.ts`, `web/src/scheduler.ts`, `web/src/vite-env.d.ts`, `web/vite.config.ts`. Por qué: avisos por consola y servidor, dormibles, e indicador en el panel.
4. **Spec e INDEX** — archivos: `specs/clock.md`, `docs/INDEX.md`. Por qué: cambio aprobado que impacta la spec (CLAUDE.md §4) y mapa del repo.

## Verificación

`cargo test --workspace --all-features`, `clippy --all-targets` y `fmt --check` limpios; `npm run wasm`, `tsc --noEmit` y `vite build` pasan; en Node, `loss_report` pasó de `[0,0,0]` a `[0, 9406, 78]` tras un atraso de ~10 s; un POST a `/__obrero/loss` apareció en la terminal de `vite dev` con el formato `[obrero] PÉRDIDA: …` y código 204.
