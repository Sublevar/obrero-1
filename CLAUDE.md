# obrero-1 — convenciones para trabajar en este repo

Este archivo es para cualquier sesión de Claude Code (o agente equivalente) que trabaje en este repo, compartido entre varias personas en un proyecto de largo aliento. Antes de tocar código, **leé `docs/INDEX.md`** — es el mapa de partes del repo; evita tener que explorar carpeta por carpeta cada sesión y debe estar siempre actualizado.

Cinco pilares que este archivo sostiene: coherencia lógica, estilo de código, testeo, documentación, trazabilidad de sesiones.

## 0. Git — carga manual, nunca automática

- Claude **no ejecuta `git add`, `git commit` ni `git push`** en este repo, bajo ninguna circunstancia — ni siquiera si en el momento parece el paso obvio a seguir. Esto incluye cualquier skill o flujo automático del harness que haga add/commit/push por su cuenta (p. ej. el flujo de "creating commits" por defecto): no se usa en este repo.
- El trabajo de Claude termina en el working tree, sin nada agregado al staging area — `git status`/`git diff` quedan disponibles para que la persona los lea sobre archivos sin stagear. La persona decide qué entra, lo stagea, commitea y pushea ella misma, a mano.
- Tampoco para humanos es buena práctica acá un `git add .`/`git add -A` a ciegas: stagear todo sin mirarlo puede meter archivos que no correspondían (secretos, artefactos de build, cambios de otra tarea mezclados). La convención del repo es stagear archivo por archivo, revisando qué entra cada vez — Claude puede señalarlo si ve que se está a punto de hacer un `git add .`, pero la decisión final es de la persona.
- Claude puede *redactar* el mensaje de commit sugerido (en español, con prefijo `fix:`/`add:`/`refactor:` cuando aplica) para que la persona lo use si quiere — pero no ejecuta ninguno de los tres comandos.

## 1. Coherencia lógica

- El core (`crates/obrero-core`) es **sans-io**: no hace threads, no lee timers reales, no hace I/O. Las plataformas (web, firmware) le empujan tiempo (`advance(now, lookahead)`) y le dan los bytes a enviar. Cualquier cambio que tiente a leer un reloj real o abrir un socket/puerto *dentro* de `crates/obrero-core` está rompiendo esa frontera — va en la plataforma, no en el core.
- **Todo tiene que ser compatible con ESP32-S3 y con web.** La misma lógica corre en los dos targets, así que el código compartido (`crates/obrero-core` y cualquier código que consuman ambas plataformas) es `no_std`: solo `core`, y `alloc` cuando haga falta heap.
- **`std` es parcial en `wasm32-unknown-unknown`: se evita.** Existe, pero buena parte compila y falla en runtime. Está prohibido en cualquier código que corra en la web:
  - tiempo real: `Instant::now()` y `SystemTime::now()` entran en panic. El tiempo lo empuja la plataforma (ver sans-io arriba);
  - hilos: `thread::spawn` falla. En la web no hay hilos sin Web Workers + `SharedArrayBuffer` + nightly, y nada del repo los usa;
  - `fs`, `net`, `process`: devuelven `Unsupported`;
  - `Mutex`, `RwLock` y canales: compilan, pero solo valen como si hubiera un único hilo. No se confía en ellos para sincronizar nada.
- **Uso excepcional de `std`:** solo si es útil y viable en los dos targets a la vez, y siempre fuera del core `no_std` (en la capa de plataforma o detrás de una feature opcional que el core no activa por defecto). Antes de usarlo, Claude comprueba que cada API concreta funciona en runtime en `xtensa-esp32s3` y en `wasm32-unknown-unknown`, no solo que compila, y deja la justificación en la bitácora de la sesión. Ante la duda, `core`/`alloc`. Las APIs propias de un solo target (p. ej. hilos en el firmware, que sí tiene FreeRTOS) solo en la capa de plataforma de ese target, como frontera.
- `obrero-core` declara `#![no_std]`; el job de CI `core-no-std-xtensa` lo compila para `xtensa-esp32s3-none-elf`, así que usar `std` en el core rompe el build.
- Un cambio a un concepto compartido (Clock, Pattern, Midi, ViewModel) se refleja en todos sus consumidores (bindings wasm, UI web, firmware) o se documenta explícitamente como diferido — no se deja a medio migrar sin decirlo.

### Dependencias — solo si son irremplazables

El criterio es la fiabilidad y el testeo: herramientas de uso extendido, mantenidas y con soporte de la comunidad. Una dependencia es código ajeno que el repo pasa a cargar en web y en ESP32-S3 durante años.

- **No se agrega una librería salvo que la funcionalidad sea irremplazable**, es decir, que implementarla en el repo tenga un costo o un riesgo claramente mayor (p. ej. bindings de plataforma, USB/TinyUSB, `wasm-bindgen`). Primero se busca en `core`/`alloc` y en lo que ya está en el árbol de dependencias, no en `std` (ver arriba).
- **Tiene que ser compatible** con:
  - **ESP32-S3 y web a la vez**, si la usa código compartido: `no_std` + `alloc` (`default-features = false` si su default arrastra `std`), y que compile y funcione en `xtensa-esp32s3` y en `wasm32-unknown-unknown`. Una librería que solo anda en un target puede ir únicamente en la capa de plataforma de ese target (`obrero-wasm` o `firmware/`), nunca en el core;
  - la licencia del repo, GPL-3.0-or-later;
  - el toolchain del repo: `stable` en el workspace, `rust-version` del firmware.
- **Versiones `0.x`, sin release 1.0**: su uso se cuestiona fuertemente. Solo entran por **decisión humana explícita**. Claude no agrega por su cuenta una dependencia `0.x`: la propone con sus alternativas (implementarla en el repo, una librería ≥1.0) y la persona decide. La decisión se registra en la bitácora de la sesión.
- **Antes de proponer una dependencia**, Claude evalúa y deja por escrito en la sesión: versión y estabilidad, mantenimiento activo, adopción y uso en la comunidad, tests/CI propios, dependencias transitivas que arrastra y compatibilidad con los targets.
- Toda dependencia nueva, o todo salto de versión mayor, es un acto con registro (ver §5) que incluye el porqué y las alternativas descartadas. Si cambia el árbol de un crate, también se actualiza `docs/INDEX.md`.
- **Las dependencias actuales quedan ratificadas** por decisión humana (sesión `docs/sessions/2026-10-01-estilo-y-acoplamiento-trazabilidad.md`), incluidas las `0.x`: `wasm-bindgen`, `serde-wasm-bindgen`, `log`, `esp-idf-svc`, `embuild` y `dotenvy`. Solo se migran si una sesión posterior lo decide porque hay una opción mejor, y esa decisión se registra como cualquier otra.

## 2. Estilo de código

- El estilo es el del **código**, no el de los comentarios: código nuevo se escribe como el que lo rodea — nombres, idioma, estructura, manejo de errores. `engine.rs`/`pattern.rs` son la referencia de forma. `rustfmt`/`clippy` son el piso, no el criterio.
- Comentarios: mínimos y concisos. Solo van donde hacen falta:
  - funciones muy abstractas, donde nombre + firma no alcanzan para entender el contrato;
  - texto que una herramienta muestra como documentación (doc comments `///` para `cargo doc`, textos de `--help` o similares).

  Fuera de eso, no se comenta lo que el código ya dice.


## 3. Testeo

- La lógica del core debe ser testeable en host, sin hardware: `cargo test -p obrero-core`.
- Cualquier feature con requisito de tiempo (clocks, subdivisiones, scheduling) necesita un test de precisión/deriva, no solo un test de que "anda" — ver `crates/obrero-core/tests/engine.rs::no_drift_over_simulated_minutes` como plantilla.
- CI (`.github/workflows/rust_ci.yml`) corre `cargo test --workspace --all-features` + build WASM en stable, y `fmt`/`clippy`/`build` del firmware en su propio toolchain xtensa — un cambio no está terminado si rompe cualquiera de los dos jobs.

## 4. Especificaciones y documentación

Son dos cosas distintas.

- **Especificaciones** (`specs/`, `docs/product/`, `docs/engineering/`): describen técnicamente qué se construye y por qué. No son documentación. Cuando un cambio **aprobado por una persona en la sesión** impacta una especificación, Claude la actualiza en el mismo paso que el código. Eso incluye el estado y la fecha del encabezado, y cuenta como un acto con registro (§5). Lo que no fue aprobado no entra en una spec: va a "Queda abierto" de la sesión.
- **Documentación y tutoriales** (guías de uso, de desarrollo, READMEs orientados a usuarixs): todavía no existe y la escriben humanxs. Claude no la redacta; si detecta algo que habría que documentar, lo anota en "Queda abierto".
- Además, Claude escribe la trazabilidad: `docs/INDEX.md` y `docs/sessions/` (§5).
- Dónde va cada tipo y qué encabezado lleva está en `docs/README.md`.
- `docs/INDEX.md` se actualiza en el mismo commit que agrega, borra o renombra un crate, un módulo de nivel superior, o un documento de `docs/`/`specs/`. Un índice desactualizado es peor que no tener índice.

## 5. Trazabilidad — acoplamiento forzado

- **Todo acto de Claude que modifique un archivo del repo** se registra en el mismo paso (la misma respuesta, antes de cerrar el turno) en los dos lugares:
  1. una entrada en la "Bitácora de actos" del archivo de sesión en `docs/sessions/` (creándolo si es el primer acto de la sesión — ver `docs/sessions/README.md`);
  2. una fila en `docs/INDEX.md` §6 que apunte a esa sesión, más la actualización de las filas del índice si el acto cambió la estructura.
- Sin excepciones: aplica igual con permisos en modo manual, en un "cambio rápido", en un typo o en un cambio pedido en una sola línea. Un acto sin registro corta la cadena, y la cadena no se corta.
- Lo hace cumplir el hook `Stop` de `.claude/settings.json` (`.claude/hooks/check-trazabilidad.sh`): si algún archivo cambiado es más nuevo que `docs/INDEX.md` o que la última bitácora, el turno no cierra. Si el hook bloquea, se registra el acto — no se desactiva el hook ni se esquiva (p. ej. tocando mtimes).
- Si el working tree trae cambios que no hizo Claude (de una persona), igual se registran, marcados como "cambio ajeno detectado", sin atribuírselos.
- Esto alcanza como trazabilidad: **no se usa atribución `Co-Authored-By` en los commits** — no es información confiable de autoría real y el registro en `docs/sessions/` ya cubre el "qué se decidió y por qué" con más detalle del que cabe en un trailer de commit.

## Deuda conocida (no asumir que está resuelta)

- `README.md` tiene marcadores de conflicto de merge sin resolver (ver `docs/INDEX.md` §5) — no tocar sin que alguien decida qué lado del conflicto es el vigente.
- `firmware/src/main.rs` tiene tasks de demo y bloques comentados grandes que `docs/product/firmware-mvp-0.md` ya marcó para descartar.
