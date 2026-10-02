# Estilo de código, autoría humana de docs y acoplamiento forzado de trazabilidad

**Fecha:** 2026-10-01
**Participantes:** stfg.prof + Claude (Opus 5.5)
**Alcance:** corregir `CLAUDE.md` §2 (estilo de código, no de comentarios), mover las convenciones de escritura de documentos a un README, fijar que la documentación la escriben humanxs, y forzar que cada acto de Claude quede registrado en `docs/INDEX.md` y `docs/sessions/` en el mismo paso.

## Qué se leyó

- `CLAUDE.md`, `docs/INDEX.md`, `docs/sessions/README.md`, `docs/sessions/2026-10-01-clock-observer-design.md`.
- `README.md` (solo para confirmar que sigue con marcadores de conflicto).

## Decisiones

1. **§2 pasa a ser "Estilo de código"**: el estilo de referencia es el del código circundante; los comentarios son mínimos y solo van en funciones muy abstractas o en texto que una herramienta muestra (`///`, `--help`). Antes decía que el estilo era el de los comentarios ("español técnico, explica el por qué").
2. **Convenciones de escritura de documentos → `docs/README.md`** (nuevo). Pedido: "en el readme". Se eligió `docs/README.md` y no el `README.md` raíz porque este tiene conflictos de merge sin resolver que `CLAUDE.md` prohíbe tocar. El contenido es el que ya estaba en `CLAUDE.md` §2/§4, movido; el encabezado pasa de `Author: <rol> agent` a `Author: <rol>` porque la autoría ahora es humana.
3. **Documentación y tutoriales: humanxs.** Claude no redacta `docs/product/`, `docs/engineering/`, `specs/`, READMEs ni tutoriales; solo escribe trazabilidad (`docs/INDEX.md`, `docs/sessions/`). Lo que haya que documentar va a "Queda abierto".
4. **Acoplamiento forzado**: cada acto de Claude que modifique un archivo se registra en el mismo paso en la bitácora de sesión y en `docs/INDEX.md` §6 (nueva tabla, una fila por acto). Ya no hay sesiones exentas (antes `docs/sessions/README.md` eximía typos y cambios ya acordados).
5. **Se hace cumplir con un hook `Stop`** en `.claude/settings.json` → `.claude/hooks/check-trazabilidad.sh`. Compara mtimes: si algún archivo cambiado del working tree es más nuevo que `docs/INDEX.md` o que la última bitácora, sale con código 2 y el turno no cierra. Se usan mtimes, no "¿está modificado?", para que cada acto pida su propio registro aunque INDEX/sesión ya estuvieran modificados. Corre igual en modo manual, auto o un cambio rápido, porque lo dispara el harness, no la memoria del modelo.

6. **Política de dependencias (`CLAUDE.md` §6, nuevo pilar)**: solo se agregan librerías si la funcionalidad es irremplazable, y tienen que ser compatibles con el target (`no_std`+`alloc` en core, wasm32, ESP-IDF/xtensa), con la licencia GPL-3.0-or-later y con el toolchain. Las versiones `0.x` se cuestionan fuertemente y solo entran por decisión humana explícita. El eje es la fiabilidad y el testeo de herramientas de uso extendido y con soporte de comunidad. Pedido literal del usuario.
7. **Auditoría de dependencias actuales** (solo lectura, no se cambió ninguna): `obrero-core` → `serde 1` (opcional) ✓. `obrero-wasm` → `wasm-bindgen 0.2`, `serde-wasm-bindgen 0.6`. `firmware` → `log 0.4`, `esp-idf-svc 0.51`, `anyhow 1` ✓, `embedded-hal 1` ✓; build: `embuild 0.33`, `dotenvy 0.15`; componente IDF `esp_tinyusb ^1.1` ✓. `web` → solo dev: `typescript ~5.6` ✓, `vite ^6` ✓. Las `0.x` quedan en `docs/INDEX.md` §5, pendientes de ratificación humana.

8. **Compatibilidad ESP32-S3 + web explícita (`CLAUDE.md` §1 y §6)**: antes §1 pedía `no_std` solo para código nuevo de `obrero-core`, y §6 decía "`std` donde aplique" y pedía compatibilidad con un solo target por crate. Ahora todo el código compartido es `no_std` (`core` + `alloc`), no se usa `std` si no es compatible con los dos targets (en `wasm32-unknown-unknown` compila pero falla en runtime), y las librerías que solo andan en un target van únicamente en la capa de plataforma. Pedido literal: "tiene que ser compatible todo con esp32 y web, no hay que usar std si no es compatible".

9. **Corrección: siguen siendo 5 pilares.** Las dependencias no son un pilar nuevo: pasan a ser la subsección "Dependencias" de §1 (coherencia lógica), porque son una cuestión de compatibilidad entre targets. Esto anula el "nuevo pilar" y la numeración §6 de las decisiones 6 y 8. Corrección del usuario: "siguen siendo 5 pilares no seis".

10. **Dependencias actuales ratificadas** (decisión humana, stfg.prof), incluidas las `0.x`: `wasm-bindgen 0.2`, `serde-wasm-bindgen 0.6`, `log 0.4`, `esp-idf-svc 0.51`, `embuild 0.33` y `dotenvy 0.15`. Solo se migran si una sesión posterior lo decide porque hay una opción mejor. Pedido literal: "las dependencias actuales quedan ratificadas a menos que en una sesión posterior se decida migrarlas de haber mejores opciones".

11. **`std` en la web: evitar los casos problemáticos, permitir solo excepcionalmente (`CLAUDE.md` §1).** `std` existe en `wasm32-unknown-unknown` pero es parcial: tiempo real, hilos, `fs`/`net`/`process` fallan en runtime y `Mutex`/canales solo valen para un hilo. Se prohíben en código web. Un uso excepcional de `std` exige que sea útil y viable en ESP32-S3 y web, fuera del core `no_std`, con cada API comprobada en runtime y justificada en la sesión. Pedido literal: "evitar los casos problemáticos y solo permitir en casos excepcionales donde sea útil y viable std".

## Queda abierto

- El hook ve cualquier cambio en el working tree, incluidos los de una persona hechos a mano durante la sesión: Claude los tiene que registrar como "cambio ajeno detectado". Si resulta molesto, una alternativa es registrar el estado al inicio de la sesión con un hook `SessionStart` y comparar contra eso.
- `obrero-core` declara `serde` sin `default-features = false`: la feature `serde` arrastra `std`, en contra de §1. El arreglo probable es `default-features = false, features = ["derive", "alloc"]`; no se tocó porque cambia código y requiere verificar el build de `obrero-wasm`.
- Para borrados (sin mtime) solo se exige que INDEX y sesión estén modificados, sin chequear el orden.
- El hook se puede desactivar desde `/hooks` o con `disableAllHooks`; la regla de no esquivarlo está en `CLAUDE.md`, no la impone la técnica.
## Artefactos de esta sesión

`CLAUDE.md`, `docs/README.md` (nuevo), `docs/sessions/README.md`, `docs/INDEX.md`, `.claude/settings.json` (nuevo), `.claude/hooks/check-trazabilidad.sh` (nuevo), este archivo. Segundo pedido: `CLAUDE.md` §6, `docs/INDEX.md` §5/§6.

## Bitácora de actos

1. **Hook de trazabilidad** — archivos: `.claude/settings.json`, `.claude/hooks/check-trazabilidad.sh`. Por qué: la regla de acoplamiento tiene que cumplirse aunque el modelo la olvide.
2. **`CLAUDE.md` §2/§4/§5 reescritos** — archivos: `CLAUDE.md`. Por qué: estilo de código en vez de comentarios; documentación humana; acoplamiento forzado.
3. **Convenciones de escritura de documentos movidas a `docs/README.md`** — archivos: `docs/README.md`. Por qué: pedido explícito; el `README.md` raíz no se puede tocar.
4. **Convención de sesiones sin exenciones + sección "Bitácora de actos" en la plantilla** — archivos: `docs/sessions/README.md`. Por qué: cada acto deja registro.
5. **INDEX: `.claude/`, `docs/README.md`, esta sesión, §4 corregido y §6 "Registro de actos de IA" nuevo** — archivos: `docs/INDEX.md`. Por qué: mantener el mapa y la cadena de actos.
6. **`CLAUDE.md` §6 "Dependencias" + pilares 5→6** — archivos: `CLAUDE.md`. Por qué: explicitar que las librerías solo entran si son irremplazables y compatibles, y que las `0.x` requieren decisión humana.
7. **Auditoría de dependencias y lista de `0.x` pendientes de ratificación** — archivos: `docs/INDEX.md` (§5). Por qué: la regla nueva aplica también a lo que ya está; se señala, no se cambia.
8. **Compatibilidad ESP32-S3 + web y prohibición de `std` incompatible, explicitadas** — archivos: `CLAUDE.md` (§1, §6). Por qué: la regla existía solo a medias y §6 la contradecía.
9. **Señalado `serde` con `std` en `obrero-core`** — archivos: `docs/INDEX.md` (§5). Por qué: viola la regla recién explicitada; se señala, no se corrige.
10. **Vuelven a ser 5 pilares; "Dependencias" pasa a §1** — archivos: `CLAUDE.md`, `docs/INDEX.md` (§5, referencia). Por qué: corrección del usuario.
11. **Dependencias actuales ratificadas** — archivos: `CLAUDE.md` (§1 Dependencias), `docs/INDEX.md` (sale de §5). Por qué: decisión humana.
12. **`CLAUDE.md` §1: casos problemáticos de `std` en web prohibidos; uso excepcional condicionado** — archivos: `CLAUDE.md`. Por qué: pedido del usuario, tras aclarar qué partes de `std` fallan en `wasm32-unknown-unknown`.
