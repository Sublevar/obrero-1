# obrero-1 — convenciones para trabajar en este repo

Este archivo es para cualquier sesión de Claude Code (o agente equivalente) que trabaje en este repo, compartido entre varias personas en un proyecto de largo aliento. Antes de tocar código, **leé `docs/INDEX.md`** — es el mapa de partes del repo; evita tener que explorar carpeta por carpeta cada sesión y debe estar siempre actualizado.

Cinco pilares que este archivo sostiene: coherencia lógica, estilo de escritura, testeo, documentación, trazabilidad de sesiones.

## 0. Git — carga manual, nunca automática

- Claude **no ejecuta `git add`, `git commit` ni `git push`** en este repo, bajo ninguna circunstancia — ni siquiera si en el momento parece el paso obvio a seguir. Esto incluye cualquier skill o flujo automático del harness que haga add/commit/push por su cuenta (p. ej. el flujo de "creating commits" por defecto): no se usa en este repo.
- El trabajo de Claude termina en el working tree, sin nada agregado al staging area — `git status`/`git diff` quedan disponibles para que la persona los lea sobre archivos sin stagear. La persona decide qué entra, lo stagea, commitea y pushea ella misma, a mano.
- Tampoco para humanos es buena práctica acá un `git add .`/`git add -A` a ciegas: stagear todo sin mirarlo puede meter archivos que no correspondían (secretos, artefactos de build, cambios de otra tarea mezclados). La convención del repo es stagear archivo por archivo, revisando qué entra cada vez — Claude puede señalarlo si ve que se está a punto de hacer un `git add .`, pero la decisión final es de la persona.
- Claude puede *redactar* el mensaje de commit sugerido (en español, con prefijo `fix:`/`add:`/`refactor:` cuando aplica) para que la persona lo use si quiere — pero no ejecuta ninguno de los tres comandos.

## 1. Coherencia lógica

- El core (`crates/obrero-core`) es **sans-io**: no hace threads, no lee timers reales, no hace I/O. Las plataformas (web, firmware) le empujan tiempo (`advance(now, lookahead)`) y le dan los bytes a enviar. Cualquier cambio que tiente a leer un reloj real o abrir un socket/puerto *dentro* de `crates/obrero-core` está rompiendo esa frontera — va en la plataforma, no en el core.
- Disciplina `no_std` para código nuevo en `crates/obrero-core`: el objetivo del proyecto es que la misma lógica corra en web y en ESP32-S3. Código nuevo no debe depender de `std` (usar `core`/`alloc` cuando haga falta heap). El resto del crate todavía no es `no_std` — es deuda conocida (ver `docs/INDEX.md` §5), no una licencia para agregar más.
- Un cambio a un concepto compartido (Clock, Pattern, Midi, ViewModel) se refleja en todos sus consumidores (bindings wasm, UI web, firmware) o se documenta explícitamente como diferido — no se deja a medio migrar sin decirlo.

## 2. Estilo de escritura

- Comentarios de código en español técnico, tersos, explicando el *por qué* (una decisión, una restricción oculta, un trade-off) — no el *qué* (eso ya lo dice el código bien nombrado). Mirar `engine.rs`/`pattern.rs` como referencia de tono.
- Documentos en `docs/product/` y `docs/engineering/` llevan encabezado con autoría de rol + fecha + estado: `**obrero-1, <alcance>.** Author: <rol> agent, <fecha>. Status: <proposal | agreed baseline | ready for execution>.` — ya es la convención que el repo usa (ver `firmware-mvp-0.md`, `timeline-view-design-spec.md`).
- `specs/` es el lugar de menor formalidad para arquitectura fundacional — se puede iterar ahí antes de que exista un "producto" alrededor (ver `specs/clock.md`).
- Mensajes de commit sugeridos en español, con prefijo (`fix:`, `add:`, `refactor:`) cuando aplica — ver §0 sobre quién los ejecuta.

## 3. Testeo

- La lógica del core debe ser testeable en host, sin hardware: `cargo test -p obrero-core`.
- Cualquier feature con requisito de tiempo (clocks, subdivisiones, scheduling) necesita un test de precisión/deriva, no solo un test de que "anda" — ver `crates/obrero-core/tests/engine.rs::no_drift_over_simulated_minutes` como plantilla.
- CI (`.github/workflows/rust_ci.yml`) corre `cargo test --workspace --all-features` + build WASM en stable, y `fmt`/`clippy`/`build` del firmware en su propio toolchain xtensa — un cambio no está terminado si rompe cualquiera de los dos jobs.

## 4. Documentación

- Producto/UX → `docs/product/`. Plan de ingeniería ligado 1:1 a un spec de producto → `docs/engineering/`. Arquitectura fundacional, menos formal → `specs/`. Bitácora de sesión → `docs/sessions/`.
- `docs/INDEX.md` se actualiza en el mismo commit que agrega, borra o renombra un crate, un módulo de nivel superior, o un documento de `docs/`/`specs/`. Un índice desactualizado es peor que no tener índice.

## 5. Trazabilidad de sesiones de IA

- Toda sesión de trabajo asistida por IA que tome decisiones significativas (de arquitectura, de producto, o que determine qué queda fuera de alcance) deja un registro en `docs/sessions/` — ver `docs/sessions/README.md` para la convención y la plantilla. Esto alcanza como trazabilidad: **no se usa atribución `Co-Authored-By` en los commits** — no es información confiable de autoría real y el registro en `docs/sessions/` ya cubre el "qué se decidió y por qué" con más detalle del que cabe en un trailer de commit.

## Deuda conocida (no asumir que está resuelta)

- `README.md` tiene marcadores de conflicto de merge sin resolver (ver `docs/INDEX.md` §5) — no tocar sin que alguien decida qué lado del conflicto es el vigente.
- `crates/obrero-core` no declara `#![no_std]` todavía, pese al objetivo del proyecto — ver §1 arriba.
- `firmware/src/main.rs` tiene tasks de demo y bloques comentados grandes que `docs/product/firmware-mvp-0.md` ya marcó para descartar.
