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
| `crates/obrero-core` | Motor sans-io: clock interno 24 PPQN, parser MIDI DIN, patrón (grid + euclidiano), transporte start/stop. Testeable en host sin hardware. | Activo. **Gap:** no declara `#![no_std]` todavía (usa `Vec` vía prelude de `std`) — ver §5. | `src/engine.rs` (`Engine::advance`, `Engine::feed_midi_in`), `src/pattern.rs`, `src/midi.rs`, `src/view.rs` (`ViewModel` serializable) |
| `crates/obrero-wasm` | Wrapper `wasm-bindgen` del `Engine`. Aplana eventos MIDI a `Float64Array` (sin JSON en el camino caliente); timestamps cruzan la frontera en ms `f64`. | Activo | `src/lib.rs` (`WasmEngine`) |
| `web/` | UI TypeScript + Vite. | Activo | `src/main.ts` (bootstrap), `src/ui.ts` (grilla + DOM, 343 líneas), `src/midi.ts` (Web MIDI I/O), `src/scheduler.ts` ("A Tale of Two Clocks": pump de 25 ms, lookahead 100 ms), `src/pkg/` (⚠ generado por `wasm-pack`, no editar a mano) |
| `firmware/` | Binario ESP32(-S3), `esp-idf-svc` (std vía ESP-IDF/newlib — **no es no_std**), toolchain xtensa propio (excluido del workspace raíz, se compila aparte). | MVP-0 en curso (ver `docs/product/firmware-mvp-0.md`): **`obrero-core` todavía no está linkeado a propósito** — es una etapa deliberada, no un olvido. `main.rs` tiene tasks de demo (`task2`, `task3`) y bloques comentados grandes marcados para descartar. | `src/main.rs`, `src/perifericos/encoder.rs` (encoder rotativo vía 2× 74HC165 en cadena SPI), `src/tasks/control_spi_encoders.rs` |
| `pcbs/pcb_encoders/` | Proyecto KiCad de la placa de encoders + notas de pinout del 74HC165. | Activo | `circuito_obrero-1.kicad_*`, `readme_circuito.md`, `readme_debuc_pcb_encoders.md` |
| `.claude/` | Config de Claude Code del proyecto. Hook `Stop` que bloquea el cierre de turno si un cambio es más nuevo que `docs/INDEX.md` o la última bitácora (CLAUDE.md §5). | Activo | `settings.json`, `hooks/check-trazabilidad.sh` |
| `.github/workflows/rust_ci.yml` | CI: `cargo test --workspace --all-features` + build WASM en stable; firmware (`fmt`/`clippy`/`build`) en un job separado con toolchain xtensa. | Activo | — |

## 3. Documentación y specs

| Carpeta | Propósito | Convención de encabezado | Contenido actual |
|---|---|---|---|
| `docs/README.md` | Cómo se escriben los documentos: dónde va cada tipo, encabezado de autoría + fecha + estado. La documentación y los tutoriales los escriben humanxs. | — | — |
| `specs/` | Specs de **arquitectura fundacional**, escritos/iterados conjuntamente por humanos y agentes. Menos formales que `docs/product/` — es el lugar para definir primitivas antes de que exista un "producto" alrededor. | Libre, pero recomendado: Author/Status/Fecha como en `docs/product/`. | `clock.md` — Time / Bpm / Observer(Clock) / Sequencer / Debug (ver sesión `docs/sessions/2026-10-01-clock-observer-design.md`) |
| `docs/product/` | Specs de producto/UX. Autoría y estado explícitos (`proposal`, `agreed baseline`, etc.), para que se sepa si algo es discutible o ya es base acordada. | `**obrero-1, <target>.** Author: <rol> agent, <fecha>. Status: <estado>.` | `firmware-mvp-0.md`, `timeline-view-design-spec.md`, `usb-midi-hardware.md` |
| `docs/engineering/` | Plan de implementación ligado **1:1** a un spec de producto — detalle técnico, decisiones de API, discrepancias spec↔código ya resueltas. | Referencia explícita al spec fuente + su fecha/estado. | `timeline-view-implementation-plan.md` |
| `docs/sessions/` | Trazabilidad: qué se decidió en cada sesión de trabajo asistida por IA, qué se leyó, qué quedó abierto. Ver `docs/sessions/README.md`. | Un archivo por sesión relevante: `YYYY-MM-DD-slug.md`. | `README.md` (convención + plantilla), `2026-10-01-clock-observer-design.md`, `2026-10-01-estilo-y-acoplamiento-trazabilidad.md` |

## 4. Convenciones ya observadas en el repo (no inventadas, detectadas)

- **Roles de autoría en docs**: los specs de producto/ingeniería ya se firman como "PO agent" o "Technical Manager agent" + fecha + estado. `CLAUDE.md` formaliza esto para que cualquier agente lo siga sin que se lo tengan que explicar cada vez.
- **Estilo de código**: se sigue el del código circundante (`engine.rs`/`pattern.rs` como referencia). Comentarios mínimos: solo en funciones muy abstractas o en texto que muestra una herramienta (`///`, `--help`).
- **Testing**: lógica del core testeada en host sin hardware (`cargo test -p obrero-core`), incluyendo tests de precisión/deriva temporal (`crates/obrero-core/tests/engine.rs`: `no_drift_over_simulated_minutes`).
- **Mensajes de commit**: en español, prefijo tipo `fix:`/`add:`/`refactor:` cuando aplica.

## 5. Problemas conocidos (señalados, no corregidos en esta sesión)

- **`README.md` tiene marcadores de conflicto de merge sin resolver** (`<<<<<<<`/`=======`/`>>>>>>>` en las líneas ~28, ~57-65, ~88-97, de merges viejos `e3618c6` y `56ef925`). No se tocó porque requiere una decisión humana sobre qué lado del conflicto (comandos de `dev.sh`) es el vigente.
- **`crates/obrero-core` no es `no_std` todavía**, pese a que el objetivo declarado del proyecto es correr la misma lógica en web y en ESP32-S3 sin asumir `std`. Hoy compila porque nunca pide nada que `std` no dé (es "sans-io" de hecho), pero no hay `#![no_std]` ni `extern crate alloc` — es deuda, no una garantía. Código nuevo (como el de `specs/clock.md`) debería nacer `no_std`-limpio; migrar el resto del crate es un trabajo aparte.
- **`serde` en `obrero-core` arrastra `std`**: se declara sin `default-features = false` (`crates/obrero-core/Cargo.toml`), así que con la feature `serde` el core depende de `std`, en contra de `CLAUDE.md` §1. Pendiente; probablemente con `default-features = false, features = ["derive", "alloc"]`.
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
