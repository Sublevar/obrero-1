# Diseño de Clock/Observer (Time, Bpm, Clock, Sequencer, Debug)

**Fecha:** 2026-10-01
**Participantes:** stfg.prof + Claude (Sonnet 5)
**Alcance:** definir — no implementar — las primitivas A–E pedidas para separar tiempo/bpm/suscripción de la sequenciación actual (`Engine`), y arrancar infraestructura mínima de repo para coherencia/estilo/testeo/documentación/trazabilidad en un proyecto compartido por varias personas con IA.

## Qué se leyó

- `specs/clock.md` (versión original, con pseudocódigo incompleto) — insumo obligatorio de la tarea.
- `README.md`, `Cargo.toml`, `docs/product/*.md`, `docs/engineering/*.md`, `.github/workflows/rust_ci.yml`.
- `crates/obrero-core/src/{engine,pattern,midi,input,view,lib}.rs`, `crates/obrero-core/tests/engine.rs`, `crates/obrero-wasm/src/lib.rs`, `web/src/*.ts`, `firmware/src/main.rs`, `firmware/src/perifericos/encoder.rs`.
- <https://gameprogrammingpatterns.com/observer.html> — riesgos del Observer clásico (push/callback): bloqueo del Subject, deadlock con locks, *lapsed listener problem*, costo de heap para callbacks; recomienda reservarlo para dominios poco relacionados y evitar el camino caliente.
- <https://refactoring.guru/design-patterns/observer> — estructura canónica Subject/Observer/subscribe/unsubscribe/notify, orden de notificación no garantizado.

## Decisiones

1. **Modelo pull, no push**: el `Clock` marca un contador `pending` por suscripción en su tick; el consumidor lo lee cuando a él lo hacen avanzar. Evita bloqueo/reentrancia y heap para callbacks — necesario para `no_std`/ESP32 de larga duración. Es, de hecho, lo que ya insinuaba el pseudocódigo original (`ClockSubscription::get_pending()`).
2. **`Time` es el singleton real** (A), no `Clock`: una marca temporal monotónica en µs, una por programa, sin noción musical. Resuelve la frase cortada del original ("los clocks se derivan de leer el ___" → leer `Time`).
3. **`Bpm` es por instancia de `Clock`**, no global al programa (B) — resuelve la tensión "Time singleton" vs. "puede haber múltiples Bpm" del original.
4. **Subdivisión** (B, resto): `enum Subdivision { PerBar(1..=24), EveryNBars(1..=8) }`, usando `euclidean_hits` (ya en `pattern.rs`) como base de distribución entera sin deriva para `PerBar`, con un modo precisión opt-in que snapea a divisores exactos de 96 (4/4 × 24 PPQN) en vez de rechazar valores no exactos. Decisión del usuario: "prefiero flexibilidad y como mucho permitir activar un modo de precisión que anule o compense cuantizando".
5. **`Clock` expone `&self` en toda su API**, incluido `advance` (C) — necesario porque `Subscription<'clock>` guarda `&'clock Clock` y debe poder coexistir con llamadas repetidas a `advance`. Estado mutable detrás de `Cell`/`RefCell` (sin `alloc`, sin `unsafe`).
6. **Auto-desuscripción vía `Drop`** (C, resto), no una macro custom como pedía el enunciado original — Rust ya da el mecanismo (equivalente al "destructor unregisters" que describe gameprogrammingpatterns.com).
7. **`Sequencer` (D) queda como contrato** (`trait Sequencer { fn on_ticks(...) }`), sin tocar `Engine`/`Pattern`/`midi.rs` — el refactor real y el reemplazo del parser MIDI por una librería externa quedan para una sesión posterior, pedido explícito del usuario.
8. **`Debug` (E)** se suscribe como cualquier consumidor y emite a un `DebugSink` inyectado (no `println!`/`std::io` en el core, por `no_std`).
9. **Ubicación del documento final**: se reescribió `specs/clock.md` en el lugar, no se creó un par `docs/product`+`docs/engineering` nuevo — decisión explícita del usuario.
10. **Alcance de infraestructura esta sesión**: mínimo — `CLAUDE.md` + convención `docs/sessions/`, sin skills/subagentes todavía — decisión explícita del usuario ("ir armando de a poco").
11. **Sin atribución `Co-Authored-By` en commits**: la trazabilidad vive en `docs/sessions/`, no en el trailer del commit — decisión explícita del usuario, corrige lo que se había asumido por defecto en `CLAUDE.md`.
12. **Git queda manual para Claude**: prohibido ejecutar `git add`, `git commit` o `git push` en este repo, incluso vía flujos automáticos del harness — el trabajo termina en el working tree, sin stagear, y la persona decide qué entra y lo sube ella misma. También se documentó como convención del equipo evitar `git add .`/`git add -A` a ciegas (stagear archivo por archivo) — decisión explícita del usuario.

## Queda abierto

- Implementación real de A–E en `crates/obrero-core` (próxima sesión), con los tests descriptos en `specs/clock.md` §9.
- Migración `no_std` del resto de `crates/obrero-core` (deuda preexistente, no introducida por este trabajo).
- Refactor de `Engine` para implementar `Sequencer`, y reemplazo de `midi.rs` por una librería externa.
- Múltiples `Clock` simultáneos (el diseño ya no lo impide, pero no se construye).
- `README.md` tiene marcadores de merge sin resolver — señalado, no corregido (requiere decisión humana sobre qué lado del conflicto es el vigente).
- Infraestructura de agentes/skills por pilar (coherencia, estilo, testeo, documentación) — explícitamente diferida hasta ver qué fricciones aparecen en la práctica de equipo.

## Artefactos de esta sesión

- `docs/INDEX.md` (nuevo) — mapa de partes del repo para humanos y agentes.
- `CLAUDE.md` (nuevo, raíz) — convenciones de los 5 pilares + §0 política de git manual (sin add/commit/push por parte de Claude, sin Co-Authored-By).
- `docs/sessions/README.md` (nuevo) — convención de esta misma carpeta.
- `docs/sessions/2026-10-01-clock-observer-design.md` (este archivo).
- `specs/clock.md` (reescrito) — diseño cerrado de Time/Bpm/Clock/Sequencer/Debug.
