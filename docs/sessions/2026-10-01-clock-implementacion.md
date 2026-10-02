# Implementación del Clock (primera iteración) + deuda `no_std` + `WebTime`

**Fecha:** 2026-10-01
**Participantes:** stfg.prof + Claude (Opus 5.5)
**Alcance:** implementar Time/Bpm/Clock/Subscription/Debug de `specs/clock.md` en `obrero-core`, pagar la deuda `no_std` del core, y exponer el observer en la web vía `obrero-wasm`. `Engine` y firmware no se tocan.

## Qué se leyó

- `specs/clock.md`, `docs/sessions/2026-10-01-clock-observer-design.md`.
- `crates/obrero-core/src/{lib,engine,pattern,midi,view,input}.rs`, `crates/obrero-core/tests/engine.rs`.
- `crates/obrero-wasm/src/lib.rs`, `web/src/{main,scheduler}.ts`, `.github/workflows/rust_ci.yml`, `firmware/.cargo/config.toml`, `firmware/rust-toolchain.toml`.

## Decisiones

1. **Observer pull con `Rc`, no `Subscription<'clock>`.** wasm-bindgen no admite lifetimes, y un struct no puede guardar al mismo tiempo el `Clock` y una suscripción que lo toma prestado. Con `Rc`, la suscripción co-posee el clock, se puede exponer a JS, y el `Drop` (el `.free()` de JS) desuscribe. Límite: un solo hilo, que en la web es siempre y en ESP32 es una task por clock. Decisión del usuario, tras la explicación de qué es `Rc`.
2. **Avisos con timestamp en vez del contador `pending`.** Con solo un contador se pierde el instante de cada tick, y la web programa 100 ms adelante: el jitter sería el del pump (~25 ms). Cada suscripción guarda `Notice { tick, at_us }` en un ring fijo de 16. Si se llena, se cuenta y se informa (`take` devuelve los perdidos).
3. **Aritmética entera exacta.** `Bpm` en centésimas, y el tiempo de cada tick se calcula desde un ancla en `u128`: sin deriva e idéntico en xtensa (sin FPU de 64 bits) y en wasm. Cambiar el bpm re-ancla en el próximo tick no generado. Decisión del usuario.
4. **Fachada `TimeSource`** para **leer** el reloj real, una por plataforma (`WebTime` ahora; `EspTime` cuando el firmware linkee el core). La aritmética sigue siendo única en el core. Decisión del usuario, a partir de su propuesta de "una fachada por target".
5. **El chequeo `no_std` usa el target real.** ESP32/ESP32-S3 son Xtensa: `xtensa-esp32s3-none-elf` con el toolchain `esp`. Antes se había propuesto un target RISC-V como sustituto; el usuario lo corrigió.
6. **Agregados mínimos al spec, por fiabilidad:**
   - `Clock::reset()`, para alinear el compás al dar Play.
   - `MAX_CATCHUP_US` (1 s): ante un atraso mayor (pestaña dormida, laptop suspendida), el clock salta hacia adelante sin perder la grilla, en vez de emitir una ráfaga de ticks vencidos, y lo informa en `skipped_ticks()`.
   - Cambiar el modo precisión re-resuelve todas las suscripciones vivas desde lo que pidieron.
7. **`Sequencer` queda como contrato con la firma corregida**: `on_notices(&[Notice], &mut Vec<TimedMidi>)`, sin implementación.
8. **Sin dependencias nuevas.** `performance.now()` y `console.log` se enlazan a mano con wasm-bindgen, en lugar de usar `web-sys` (0.x).
9. **Las especificaciones no son documentación.** `specs/`, `docs/product/` y `docs/engineering/` son descripciones técnicas: cuando un cambio aprobado las impacta, se actualizan en el mismo paso, también si lo hace Claude. La documentación y los tutoriales todavía no existen y los escribirán humanxs. Corrige `CLAUDE.md` §4 de la sesión `2026-10-01-estilo-y-acoplamiento-trazabilidad.md`, que metía `specs/` dentro de "documentación". Pedido literal: "specs no es documentación como tal, es descripción técnica; si fueron aprobadas impacta allí también […] la documentación no está escrita aún, será para un futuro". Interpretación de Claude, a confirmar: `docs/product/` y `docs/engineering/` también son especificaciones, porque son specs de producto y planes técnicos y la documentación "todavía no existe".
10. **`specs/clock.md` actualizado a lo implementado**, con una tabla (§11) de cambios respecto del diseño original. Las mejoras no aprobadas van en §10 como propuestas, no como diseño.

## Queda abierto

- **CI del firmware**: el job `firmware-checks` instala `buildtargets: esp32`, pero `firmware/.cargo/config.toml` compila para `xtensa-esp32s3-espidf`. No se tocó; lo tiene que revisar una persona.
- **El job nuevo `core-no-std-xtensa` no se pudo correr en GitHub desde la sesión.** Los mismos comandos pasan localmente con el toolchain `esp`.
- **Falta la verificación en el navegador** con `?clockdebug` (ver bitácora). El binding se verificó en Node con `performance.now()` real.
- **Mejoras sugeridas, sin dar por hecha su utilidad:**
  - `PerBar` hasta 96: el tope de 24 del spec no se condice con su justificación por PPQN;
  - start/stop de transporte en `Clock`;
  - seguir clock MIDI externo desde `Clock`;
  - migrar `Engine` a `Clock`, lo que también lo saca de `f64`;
  - `EspTime`.
- Clippy marca un `if` colapsable en `engine.rs:349`. Es preexistente y no se tocó.

## Artefactos de esta sesión

`crates/obrero-core/{Cargo.toml, src/lib.rs, src/engine.rs, src/pattern.rs, src/view.rs, src/midi.rs}`, `crates/obrero-core/src/clock.rs` (nuevo), `crates/obrero-core/tests/clock.rs` (nuevo), `crates/obrero-wasm/src/lib.rs`, `crates/obrero-wasm/src/{clock,time}.rs` (nuevos), `web/src/main.ts`, `.github/workflows/rust_ci.yml`, `CLAUDE.md`, `docs/README.md`, `specs/clock.md`, `docs/INDEX.md`, este archivo.

## Bitácora de actos

1. **Deuda `no_std` pagada** — archivos: `crates/obrero-core/src/{lib,engine,pattern,view,midi}.rs`, `crates/obrero-core/Cargo.toml`. Por qué: `#![no_std]` + `extern crate alloc`, `format_midi_msg` a `alloc`, `serde` con `default-features = false, features = ["derive", "alloc"]`. Los 28 tests previos pasan sin cambios, y compila para `xtensa-esp32s3-none-elf` con y sin serde.
2. **`euclidean_hit` extraída** — archivos: `crates/obrero-core/src/pattern.rs`. Por qué: la misma fórmula que `euclidean_hits`, consultable por tick en O(1) para `Subdivision::PerBar`.
3. **`clock.rs`** — archivos: `crates/obrero-core/src/clock.rs`, `crates/obrero-core/src/lib.rs`. Por qué: Time, `TimeSource`, Bpm, Subdivision, Clock, Subscription, Sequencer y Debug, según las decisiones 1–7.
4. **Tests del clock** — archivos: `crates/obrero-core/tests/clock.rs`. Por qué: 18 tests. Incluyen deriva en 5 min (ventanas de la web) y en 1 h, cambio de bpm, `PerBar(7)`, precisión, `EveryNBars`, 32 suscripciones, `ClockFull` y liberación, desborde contado, salto por atraso, reset y `DebugTap`.
5. **`WebTime` + binding wasm** — archivos: `crates/obrero-wasm/src/{time,clock,lib}.rs`. Por qué: `WasmClock`, `WasmSubscription` (`.free()` desuscribe) y `WasmDebugTap` con `console.log`. Verificado en Node: negras cada 500.000 ms a 120 bpm, `.free()` libera el slot y los errores llegan a JS.
6. **Hook `?clockdebug` en la web** — archivos: `web/src/main.ts`. Por qué: probar el observer en el navegador sin cambiar nada cuando el parámetro no está. Expone `window.obreroClock`.
7. **Job de CI `core-no-std-xtensa`** — archivos: `.github/workflows/rust_ci.yml`. Por qué: que el compilador haga cumplir `no_std` contra el chip real.
8. **Deuda resuelta en `CLAUDE.md` e INDEX actualizado** — archivos: `CLAUDE.md`, `docs/INDEX.md`. Por qué: `no_std` y `serde` ya no son deuda; nuevos módulos y job en el mapa; observación del CI del firmware.
9. **`CLAUDE.md` §4 y `docs/README.md`: especificaciones separadas de documentación** — archivos: `CLAUDE.md`, `docs/README.md`. Por qué: corrección del usuario (decisión 9).
10. **`specs/clock.md` actualizado a la implementación aprobada** — archivos: `specs/clock.md`. Por qué: el spec describía un diseño que ya no es el implementado (decisión 10).
