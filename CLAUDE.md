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
- Disciplina `no_std` para código nuevo en `crates/obrero-core`: el objetivo del proyecto es que la misma lógica corra en web y en ESP32-S3. Código nuevo no debe depender de `std` (usar `core`/`alloc` cuando haga falta heap). El resto del crate todavía no es `no_std` — es deuda conocida (ver `docs/INDEX.md` §5), no una licencia para agregar más.
- Un cambio a un concepto compartido (Clock, Pattern, Midi, ViewModel) se refleja en todos sus consumidores (bindings wasm, UI web, firmware) o se documenta explícitamente como diferido — no se deja a medio migrar sin decirlo.

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

## 4. Documentación

- **La documentación y los tutoriales los escriben humanxs.** Claude no redacta documentación (`docs/product/`, `docs/engineering/`, `specs/`, READMEs) ni tutoriales. Si detecta algo que habría que documentar, lo deja anotado en "Queda abierto" de la bitácora de la sesión para que una persona lo escriba.
- La única escritura en `docs/` que le corresponde a Claude es la de trazabilidad: `docs/INDEX.md` y `docs/sessions/` (ver §5).
- Cómo se escriben los documentos (dónde va cada tipo, encabezado de autoría + fecha + estado) está en `docs/README.md`.
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
- `crates/obrero-core` no declara `#![no_std]` todavía, pese al objetivo del proyecto — ver §1 arriba.
- `firmware/src/main.rs` tiene tasks de demo y bloques comentados grandes que `docs/product/firmware-mvp-0.md` ya marcó para descartar.
