# PPQN 480 + `Engine` migrado a `Clock` + debug en la web

**Fecha:** 2026-10-02
**Participantes:** stfg.prof + Claude (Sonnet 5.5)
**Alcance:** revisar dónde está limitado el PPQN y subir la resolución, reescribir `Engine` para que se suscriba a un `Clock` creado dentro de sí, y agregar un modo debug en la web con bpm y marca de tiempo del próximo tick.

## Qué se leyó

- `docs/INDEX.md`, `specs/clock.md`, `docs/sessions/2026-10-01-clock-implementacion.md`, `docs/sessions/README.md`.
- `crates/obrero-core/src/{clock,engine,pattern,lib,view}.rs`, `crates/obrero-core/tests/{engine,clock}.rs`.
- `crates/obrero-wasm/src/{lib,clock,time}.rs`, `web/src/{main,scheduler}.ts`, `README.md`, `.github/workflows/rust_ci.yml`.
- `docs/product/timeline-view-design-spec.md` y `docs/engineering/timeline-view-implementation-plan.md` (solo las menciones de PPQN).

## Decisiones

1. **Dónde estaba limitado el PPQN:** `pattern::PPQN = 24` atado al MIDI clock (1 tick = 1 byte `0xF8`, también en la entrada externa), más `gate_ticks`/`ticks_per_step` en `u8`, `TimeMark.tick_in_beat` en `u8`, `TICKS_PER_BAR = 96`, la lista fija de divisores de 96 y `PerBar` hasta 24.
2. **PPQN = 480** (decisión del usuario). El MIDI clock sigue a 24 PPQN: se emite un `0xF8` cada `TICKS_PER_MIDI_CLOCK = 20` ticks. En clock externo cada `0xF8` corre 20 ticks en ráfaga; interpolar queda diferido.
3. **`Engine` se suscribe a su propio `Clock`** con `Subdivision::EveryTick` y un ring grande (decisión del usuario, que descartó el consumo directo por lote). `Engine` implementa `Sequencer`. Claude había recomendado el lote directo para no tener pérdida de ticks; el usuario eligió la suscripción y pidió ajustar los márgenes para que sea coherente y segura. Se logra así: rings reservados al suscribir y dimensionados por el peor caso de una pasada, atraso y lookahead acotados a 250 ms, y `Engine::advance` parte lookaheads mayores en pasadas.
4. **Todos los márgenes en `crates/obrero-core/src/tuning.rs`**, derivados entre sí, y documentados en el `README.md` (secciones "Resolución temporal (PPQN)" y "Ajustes") para una etapa de fine tuning, como pidió el usuario. `PER_BAR_MAX` sube a 96 (la propuesta de `specs/clock.md` §10, aprobada ahora). `MAX_CATCHUP_US` baja de 1 s a 250 ms, por la capacidad del ring.
5. **El README se tocó solo en zonas sin marcadores de conflicto** (arquitectura y dos secciones nuevas antes de "Workspace raíz"), por pedido explícito del usuario.
6. **Debug web** (`?clockdebug`): panel fijo con bpm, `now`, instante del próximo tick y su diferencia, más logs por consola; el `DebugTap` cuelga del `Clock` del motor. Sin `?clockdebug`, nada cambia.
7. **Sin dependencias nuevas.** El ring pasa de array inline a `Box<[Notice]>` (`alloc`), reservado al suscribir.

## Queda abierto

- **Probar en el navegador con `?clockdebug`**: no se hizo desde la sesión. Se verificó en Node con `performance.now()` real (próximo tick a +1,04 ms del lookahead, `debug_tap`, `set_tempo`).
- **Clock MIDI externo sin interpolar**: cada `0xF8` dispara 20 ticks en el mismo instante.
- **Costo en el ESP32**: 2400 ticks/s a 300 bpm con lookahead 0. El firmware todavía no linkea el core; medirlo cuando lo haga, y bajar `PPQN` en `tuning.rs` si hace falta.
- **Los números del spec del timeline** (96, 768, `n × 6`) están en la base 24 PPQN; se dejó una nota para multiplicar por 20, sin reescribirlos.
- **`web/src/pkg/`** se regeneró con `npm run wasm` (generado, no se edita a mano).
- `docs/INDEX.md` §5 sigue diciendo que el README tiene marcadores de conflicto, y ya no es cierto (ver acto 11). No se corrigió: que lo confirme quien resolvió el conflicto.
- Documentar los ajustes para usuarixs queda para quienes escriban la documentación; el `README` solo describe las variables.

## Artefactos de esta sesión

`crates/obrero-core/src/{tuning,clock,engine,pattern,lib}.rs`, `crates/obrero-core/tests/{clock,engine}.rs`, `crates/obrero-wasm/src/{lib,clock}.rs`, `web/src/main.ts`, `web/src/pkg/` (generado), `README.md`, `specs/clock.md`, `docs/product/timeline-view-design-spec.md`, `docs/engineering/timeline-view-implementation-plan.md`, `docs/INDEX.md`, este archivo.

## Bitácora de actos

1. **`tuning.rs`: variables de ajuste** — archivos: `crates/obrero-core/src/tuning.rs` (nuevo), `crates/obrero-core/src/lib.rs`. Por qué: PPQN 480 y todos los topes en un solo lugar, con los rings derivados.
2. **Patrón en `u16` y PPQN 480** — archivos: `crates/obrero-core/src/pattern.rs`. Por qué: `gate_ticks`/`ticks_per_step` en `u16`, defaults derivados del PPQN, `events_at(u64)`.
3. **`Clock`: ring en heap, `EveryTick`, `advance_to`, `next_tick_at_us`, `subscribe_with_capacity`** — archivos: `crates/obrero-core/src/clock.rs`. Por qué: soporte del consumo del `Engine`; divisores de precisión calculados; `tick_in_beat` en `u16`.
4. **`Engine` suscripto a su `Clock`** — archivos: `crates/obrero-core/src/engine.rs`. Por qué: reemplaza el reloj `f64`; implementa `Sequencer`; `0xF8` cada 20 ticks; clock externo en ráfagas de 20 ticks.
5. **Tests** — archivos: `crates/obrero-core/tests/clock.rs`, `crates/obrero-core/tests/engine.rs`. Por qué: 22 + 24 tests; nuevos para `EveryTick`, peor caso del ring, 0xF8 a 24 PPQN, deriva a 300 bpm, atraso largo sin pérdida y suscripciones extra.
6. **Binding wasm** — archivos: `crates/obrero-wasm/src/lib.rs`, `crates/obrero-wasm/src/clock.rs`. Por qué: `bpm()`, `next_tick_ms()`, `debug_tap()`, `gate_ticks` en `u16`, variante nueva.
7. **Modo debug web** — archivos: `web/src/main.ts`. Por qué: panel y logs de bpm y próximo tick sobre el `Engine` real.
8. **README: PPQN y ajustes** — archivos: `README.md`. Por qué: pedido explícito del usuario; fuera de las zonas con conflicto de merge.
9. **Specs actualizados** — archivos: `specs/clock.md`, `docs/product/timeline-view-design-spec.md`, `docs/engineering/timeline-view-implementation-plan.md`. Por qué: cambio aprobado que los impacta (CLAUDE.md §4).
10. **INDEX** — archivos: `docs/INDEX.md`. Por qué: módulo nuevo, filas de partes y de actos (CLAUDE.md §5).
11. **Cambio ajeno detectado en `README.md`** — archivos: `README.md`. Por qué: mientras se trabajaba apareció una edición que no es de Claude: se resolvieron los marcadores de conflicto de merge del README (ya no hay `<<<<<<<`/`=======`/`>>>>>>>`; el lado vigente de `dev.sh` lo decidió esa persona) y se agregó un bullet con un link sobre PPQN, pegado a la sección nueva sin línea en blanco. No se tocó ni se revirtió; se registra sin atribuirlo.

## Verificación

`cargo test --workspace --all-features` (8 + 22 + 24 tests), `cargo clippy --workspace --all-features --all-targets` y `cargo fmt --check` limpios; el core compila para `xtensa-esp32s3-none-elf` con y sin `serde`; `cargo build -p obrero-wasm --target wasm32-unknown-unknown`, `npm run wasm` y `tsc --noEmit` pasan; el binding se probó en Node (`bpm`, `next_tick_ms`, `debug_tap`, `set_tempo`).
