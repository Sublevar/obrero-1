# Índice del repo — obrero-1

**Para humanos y agentes.** Este documento es el mapa de partes del proyecto: qué hay, dónde vive, qué estado tiene y de qué depende. El objetivo es que una sesión (humana o de IA) pueda orientarse leyendo **esto primero**, en vez de recorrer el árbol de archivos carpeta por carpeta.

**Regla de mantenimiento:** cualquier cambio que agregue, borre o renombre un módulo de nivel superior, un crate, o un documento de `docs/`/`specs/` debe actualizar la fila correspondiente en el mismo commit. Un índice desactualizado es peor que no tener índice — si lo tocás, mantenelo honesto.

**Acoplamiento forzado:** además, cada acto de un agente que modifique archivos agrega su fila en §6 en el mismo paso (ver `CLAUDE.md` §5).

---

## 1. Arquitectura en una frase

Un motor de secuenciador MIDI **sans-io** (`crates/obrero-core`) compartido por dos targets: **web** (WASM + Web MIDI) y **ESP32-S3** (USB-MIDI nativo). El core no sabe de threads, timers reales ni I/O — cada plataforma le empuja tiempo (`advance(now, lookahead)`) y le da los bytes a enviar.

```
crates/obrero-core   → motor portable: clock, patrón, parser MIDI, transporte
crates/obrero-wasm   → bindings wasm-bindgen del motor
web/                 → UI TypeScript (vite): grilla, Web MIDI, scheduler
firmware/            → binario ESP32(-S3), toolchain propio (xtensa), fuera del workspace raíz
pcbs/                → diseño KiCad del hardware de control (encoders)
specs/               → specs de arquitectura "vivas", humano+agente
docs/product/        → specs de producto/UX, con autoría y estado explícitos
docs/engineering/    → planes de implementación ligados 1:1 a un spec de producto
docs/sessions/       → bitácora de trazabilidad de sesiones de trabajo con IA
.claude/             → config compartida de Claude Code: hook Stop de trazabilidad
```

---

## 2. Partes del código

| Parte | Qué es | Estado | Puntos de entrada |
|---|---|---|---|
| `crates/obrero-core` | Motor sans-io y `no_std` (`core` + `alloc`): clock interno del `Engine` 24 PPQN, parser MIDI DIN, patrón (grid + euclidiano), transporte start/stop, y el observer `Clock` nuevo (Time, `TimeSource`, Bpm entero, Subdivision, Subscription vía `Rc`, Debug). Testeable en host sin hardware. | Activo. `Engine` todavía usa su propio reloj `f64`; migrarlo a `Clock` está pendiente. | `src/engine.rs` (`Engine::advance`, `Engine::feed_midi_in`), `src/clock.rs` (`Clock::advance`, `Clock::subscribe`, `Subscription::take`), `src/pattern.rs`, `src/midi.rs`, `src/view.rs` (`ViewModel` serializable) |
| `crates/obrero-wasm` | Wrapper `wasm-bindgen` del `Engine` y del `Clock`. Aplana eventos MIDI y avisos del clock a `Float64Array` (sin JSON en el camino caliente); timestamps cruzan la frontera en ms `f64`. `WebTime` lee `performance.now()` con un binding a mano (sin `web-sys`). | Activo | `src/lib.rs` (`WasmEngine`), `src/clock.rs` (`WasmClock`, `WasmSubscription`, `WasmDebugTap`), `src/time.rs` (`WebTime`) |
| `web/` | UI TypeScript + Vite. | Activo | `src/main.ts` (bootstrap), `src/ui.ts` (grilla + DOM, 343 líneas), `src/midi.ts` (Web MIDI I/O), `src/scheduler.ts` ("A Tale of Two Clocks": pump de 25 ms, lookahead 100 ms), `src/pkg/` (⚠ generado por `wasm-pack`, no editar a mano) |
| `firmware/` | Binario ESP32(-S3), `esp-idf-svc` (std vía ESP-IDF/newlib — **no es no_std**), toolchain xtensa propio (excluido del workspace raíz, se compila aparte). | MVP-0 en curso (ver `docs/product/firmware-mvp-0.md`): **`obrero-core` todavía no está linkeado a propósito** — es una etapa deliberada, no un olvido. `main.rs` tiene tasks de demo (`task2`, `task3`) y bloques comentados grandes marcados para descartar. | `src/main.rs`, `src/perifericos/encoder.rs` (encoder rotativo vía 2× 74HC165 en cadena SPI), `src/tasks/control_spi_encoders.rs` |
| `pcbs/pcb_encoders/` | Proyecto KiCad de la placa de encoders + notas de pinout del 74HC165. | Activo | `circuito_obrero-1.kicad_*`, `readme_circuito.md`, `readme_debuc_pcb_encoders.md` |
| `.claude/` | Config de Claude Code del proyecto. Hook `Stop` que bloquea el cierre de turno si un cambio es más nuevo que `docs/INDEX.md` o la última bitácora (CLAUDE.md §5). | Activo | `settings.json`, `hooks/check-trazabilidad.sh` |
| `.github/workflows/rust_ci.yml` | CI: `cargo test --workspace --all-features` + build WASM en stable; `core-no-std-xtensa` compila el core para `xtensa-esp32s3-none-elf` (verifica `no_std`); firmware (`fmt`/`clippy`/`build`) en un job separado con toolchain xtensa. | Activo | — |

## 3. Documentación y specs

| Carpeta | Propósito | Convención de encabezado | Contenido actual |
|---|---|---|---|
| `docs/README.md` | Dónde va cada tipo de documento y qué encabezado lleva. Distingue especificaciones (`specs/`, `docs/product/`, `docs/engineering/`: se actualizan con cada cambio aprobado) de documentación y tutoriales (todavía no existen; los escriben humanxs). | — | — |
| `specs/` | Specs de **arquitectura fundacional**, escritos/iterados conjuntamente por humanos y agentes. Menos formales que `docs/product/` — es el lugar para definir primitivas antes de que exista un "producto" alrededor. | Libre, pero recomendado: Author/Status/Fecha como en `docs/product/`. | `clock.md` — Time / TimeSource / Bpm / Observer(Clock) / Sequencer / Debug, implementado en primera iteración (sesiones `2026-10-01-clock-observer-design.md` y `2026-10-01-clock-implementacion.md`) |
| `docs/product/` | Specs de producto/UX. Autoría y estado explícitos (`proposal`, `agreed baseline`, etc.), para que se sepa si algo es discutible o ya es base acordada. | `**obrero-1, <target>.** Author: <rol> agent, <fecha>. Status: <estado>.` | `firmware-mvp-0.md`, `timeline-view-design-spec.md`, `usb-midi-hardware.md` |
| `docs/engineering/` | Plan de implementación ligado **1:1** a un spec de producto — detalle técnico, decisiones de API, discrepancias spec↔código ya resueltas. | Referencia explícita al spec fuente + su fecha/estado. | `timeline-view-implementation-plan.md` |
| `docs/sessions/` | Trazabilidad: qué se decidió en cada sesión de trabajo asistida por IA, qué se leyó, qué quedó abierto. Ver `docs/sessions/README.md`. | Un archivo por sesión relevante: `YYYY-MM-DD-slug.md`. | `README.md` (convención + plantilla), `2026-10-01-clock-observer-design.md`, `2026-10-01-estilo-y-acoplamiento-trazabilidad.md`, `2026-10-01-clock-implementacion.md` |

## 4. Convenciones ya observadas en el repo (no inventadas, detectadas)

- **Roles de autoría en docs**: los specs de producto/ingeniería ya se firman como "PO agent" o "Technical Manager agent" + fecha + estado. `CLAUDE.md` formaliza esto para que cualquier agente lo siga sin que se lo tengan que explicar cada vez.
- **Estilo de código**: se sigue el del código circundante (`engine.rs`/`pattern.rs` como referencia). Comentarios mínimos: solo en funciones muy abstractas o en texto que muestra una herramienta (`///`, `--help`).
- **Testing**: lógica del core testeada en host sin hardware (`cargo test -p obrero-core`), incluyendo tests de precisión/deriva temporal (`crates/obrero-core/tests/engine.rs`: `no_drift_over_simulated_minutes`).
- **Mensajes de commit**: en español, prefijo tipo `fix:`/`add:`/`refactor:` cuando aplica.

## 5. Problemas conocidos (señalados, no corregidos en esta sesión)

- **`README.md` tiene marcadores de conflicto de merge sin resolver** (`<<<<<<<`/`=======`/`>>>>>>>` en las líneas ~28, ~57-65, ~88-97, de merges viejos `e3618c6` y `56ef925`). No se tocó porque requiere una decisión humana sobre qué lado del conflicto (comandos de `dev.sh`) es el vigente.
- **CI del firmware instala `buildtargets: esp32`** pero `firmware/.cargo/config.toml` compila para `xtensa-esp32s3-espidf`. Señalado, no corregido.
- **`firmware/src/main.rs`** tiene tasks de demo (`task2`, `task3`) y bloques comentados grandes que `docs/product/firmware-mvp-0.md` ya marcó para descartar — pendiente de limpieza.

## 6. Registro de actos de IA

Una fila por acto de un agente que modificó archivos, agregada en el mismo paso que la entrada en la bitácora de la sesión (CLAUDE.md §5). Si una fila no tiene su entrada en la sesión, o al revés, la cadena está cortada.

| Fecha | Sesión | Acto | Archivos |
|---|---|---|---|
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 1. Hook `Stop` de trazabilidad | `.claude/settings.json`, `.claude/hooks/check-trazabilidad.sh` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 2. `CLAUDE.md` §2/§4/§5: estilo de código, docs humanas, acoplamiento forzado | `CLAUDE.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 3. Convenciones de escritura de documentos movidas a `docs/README.md` | `docs/README.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 4. Sesiones sin exenciones + "Bitácora de actos" en la plantilla | `docs/sessions/README.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 5. INDEX: `.claude/`, `docs/README.md`, sesión nueva, §4 y §6 | `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 6. `CLAUDE.md` §6 "Dependencias": solo si son irremplazables y compatibles; las `0.x` requieren decisión humana | `CLAUDE.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 7. Auditoría de dependencias: lista de `0.x` pendientes en §5 | `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 8. `CLAUDE.md` §1/§6: todo compatible con ESP32-S3 y web; no se usa `std` si no es compatible | `CLAUDE.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 9. Señalado `serde` con `std` en `obrero-core` (§5) | `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 10. Vuelven a ser 5 pilares: "Dependencias" pasa a subsección de §1 (coherencia lógica) | `CLAUDE.md`, `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 11. Dependencias actuales ratificadas por decisión humana; sale de §5 | `CLAUDE.md`, `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` | 12. `CLAUDE.md` §1: `std` problemático en web prohibido, uso excepcional condicionado | `CLAUDE.md` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 1. Deuda `no_std` pagada (core + serde) | `crates/obrero-core/src/{lib,engine,pattern,view,midi}.rs`, `crates/obrero-core/Cargo.toml` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 2. `euclidean_hit` extraída en `pattern.rs` | `crates/obrero-core/src/pattern.rs` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 3. `clock.rs`: Time, `TimeSource`, Bpm, Subdivision, Clock, Subscription, Sequencer, Debug | `crates/obrero-core/src/clock.rs`, `crates/obrero-core/src/lib.rs` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 4. Tests del clock (18) | `crates/obrero-core/tests/clock.rs` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 5. `WebTime` + `WasmClock`/`WasmSubscription`/`WasmDebugTap` | `crates/obrero-wasm/src/{time,clock,lib}.rs` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 6. Hook `?clockdebug` en la web | `web/src/main.ts` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 7. Job de CI `core-no-std-xtensa` | `.github/workflows/rust_ci.yml` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 8. Deuda resuelta en `CLAUDE.md` + INDEX actualizado | `CLAUDE.md`, `docs/INDEX.md` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 9. `CLAUDE.md` §4 + `docs/README.md`: especificaciones ≠ documentación | `CLAUDE.md`, `docs/README.md` |
| 2026-10-01 | `2026-10-01-clock-implementacion.md` | 10. `specs/clock.md` actualizado a la implementación aprobada | `specs/clock.md` |
