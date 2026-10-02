# Clock externo dentro del `Clock`, rings potencia de 2 y `WasmClock` eliminado

**Fecha:** 2026-10-02
**Participantes:** stfg.prof + Claude (Sonnet 5.5)
**Alcance:** corregir el tamaño de los rings, llevar el reloj MIDI externo al `Clock` nuevo para que haya un solo reloj, y quitar el `Clock` independiente de la web.

## Qué se leyó

- `crates/obrero-core/src/{clock,engine,tuning}.rs`, `crates/obrero-core/tests/{clock,engine}.rs`, `crates/obrero-wasm/src/{lib,clock,time}.rs`, `web/src/main.ts`, `specs/clock.md`, `README.md`, `docs/INDEX.md`, `docs/sessions/2026-10-02-ppqn-480-engine-clock.md`.

## Decisiones

1. **Rings potencia de 2.** El usuario observó que 1209 y 69 no seguían ninguna convención. Aclaró que un ring no tiene por qué seguir la de MIDI: se busca performance y un tamaño predecible. Se redondean a potencia de 2 (índice con máscara, sin división) y `subscribe_with_capacity` redondea cualquier capacidad pedida.
2. **Presupuesto de memoria del ESP32-S3** (pregunta del usuario: "¿no es abrumador?"). Cada aviso pesa 16 B; el tamaño lo fija la ventana (`MAX_CATCHUP_US + MAX_LOOKAHEAD_US`), no el PPQN. Ambas bajan a 100 ms: ring del `Engine` 512 (8 KiB), por suscripción 32 (512 B). Con 250 ms habría sido 2048 (32 KiB), y `tick_buf` duplicaba esa memoria; ya no se reserva por adelantado. Atrasos de más de 100 ms saltan hacia adelante en vez de recuperarse.
3. **Un solo reloj.** El `f64` viejo ya no existía, pero el reloj externo seguía un camino aparte (ráfaga de 20 ticks por `0xF8`). Ahora `Clock::external_pulse` mide el período (suavizado, acotado a 20–300 bpm), ajusta el `Bpm` y reparte los 20 ticks de cada pulso desde su llegada (predicción, latencia 0). El `Engine` consume por la misma suscripción `EveryTick`. `ClockSource` pasó de `engine.rs` a `clock.rs` (re-exportado desde `engine`).
4. **El contador de ticks del motor sigue siendo propio** (no se toma de `Notice::tick`), porque con Stop/Continue externos los pulsos detenidos avanzan el `Clock` pero no el patrón. Un cambio respecto del plan aprobado, por esa razón.
5. **`WasmClock`, `WasmSubscription` y `WebTime` eliminados** (por pedido: el antiguo no se usa más). `WasmDebugTap` queda en `debug.rs`. El trait `TimeSource` sigue en el core para `EspTime`.
6. **Sin dependencias nuevas.**

## Queda abierto

- **Probar el reloj externo con un DAW real en el navegador**: se verificó con tests de host y una prueba en Node (100 bpm medidos con pulsos cada 25 ms).
- **Predicción del reloj externo:** si el tempo del DAW cambia bruscamente, el pulso en curso usa el período anterior hasta el siguiente; `EXTERNAL_SMOOTHING` regula el compromiso entre jitter y seguimiento.
- **Costo en el ESP32:** el firmware todavía no linkea el core; medir cuando lo haga.

(La deuda de los marcadores de conflicto del README quedó cerrada: ver acto 7.)

## Artefactos de esta sesión

`crates/obrero-core/src/{tuning,clock,engine}.rs`, `crates/obrero-core/tests/{clock,engine}.rs`, `crates/obrero-wasm/src/{lib,debug}.rs` (`clock.rs` y `time.rs` borrados), `web/src/pkg/` (regenerado), `README.md`, `specs/clock.md`, `docs/INDEX.md`, `CLAUDE.md`, este archivo.

## Bitácora de actos

1. **Rings potencia de 2, ventanas de 100 ms, `EXTERNAL_SMOOTHING`** — archivos: `crates/obrero-core/src/tuning.rs`. Por qué: tamaños predecibles y presupuesto de memoria del ESP32-S3.
2. **`Clock`: `ClockSource`, `external_pulse`, ring con máscara** — archivos: `crates/obrero-core/src/clock.rs`. Por qué: un solo reloj para interno y externo.
3. **`Engine`: el 0xF8 externo pasa por el `Clock`** — archivos: `crates/obrero-core/src/engine.rs`. Por qué: eliminar el camino paralelo.
4. **Tests** — archivos: `crates/obrero-core/tests/clock.rs`, `crates/obrero-core/tests/engine.rs`. Por qué: 27 + 26 tests; nuevos de clock externo (reparto parejo, tempo medido y rampa, jitter sin deriva, silencio, Continue) y de rings.
5. **Binding wasm sin `WasmClock`** — archivos: `crates/obrero-wasm/src/{lib,debug}.rs`; borrados `crates/obrero-wasm/src/{clock,time}.rs`. Por qué: un solo `Clock` en la web.
6. **README, spec e INDEX** — archivos: `README.md`, `specs/clock.md`, `docs/INDEX.md`. Por qué: cambio aprobado que impacta la spec (CLAUDE.md §4); tabla de ajustes con los valores nuevos.
7. **Deuda del README resuelta, registrada** — archivos: `docs/INDEX.md` (§5), `CLAUDE.md` ("Deuda conocida"). Por qué: stfg.prof confirmó que resolvió los conflictos de merge del README; ya no hay marcadores y la advertencia de no tocarlo quedó obsoleta.

## Verificación

`cargo test --workspace --all-features` (8 + 27 + 26), `clippy --all-targets` y `fmt --check` limpios; core para `xtensa-esp32s3-none-elf` con y sin `serde`; wasm32, `npm run wasm` y `tsc --noEmit` pasan; prueba en Node del reloj externo con el binding.
