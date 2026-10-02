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

## Queda abierto

- El hook ve cualquier cambio en el working tree, incluidos los de una persona hechos a mano durante la sesión: Claude los tiene que registrar como "cambio ajeno detectado". Si resulta molesto, una alternativa es registrar el estado al inicio de la sesión con un hook `SessionStart` y comparar contra eso.
- Para borrados (sin mtime) solo se exige que INDEX y sesión estén modificados, sin chequear el orden.
- El hook se puede desactivar desde `/hooks` o con `disableAllHooks`; la regla de no esquivarlo está en `CLAUDE.md`, no la impone la técnica.
## Artefactos de esta sesión

`CLAUDE.md`, `docs/README.md` (nuevo), `docs/sessions/README.md`, `docs/INDEX.md`, `.claude/settings.json` (nuevo), `.claude/hooks/check-trazabilidad.sh` (nuevo), este archivo.

## Bitácora de actos

1. **Hook de trazabilidad** — archivos: `.claude/settings.json`, `.claude/hooks/check-trazabilidad.sh`. Por qué: la regla de acoplamiento tiene que cumplirse aunque el modelo la olvide.
2. **`CLAUDE.md` §2/§4/§5 reescritos** — archivos: `CLAUDE.md`. Por qué: estilo de código en vez de comentarios; documentación humana; acoplamiento forzado.
3. **Convenciones de escritura de documentos movidas a `docs/README.md`** — archivos: `docs/README.md`. Por qué: pedido explícito; el `README.md` raíz no se puede tocar.
4. **Convención de sesiones sin exenciones + sección "Bitácora de actos" en la plantilla** — archivos: `docs/sessions/README.md`. Por qué: cada acto deja registro.
5. **INDEX: `.claude/`, `docs/README.md`, esta sesión, §4 corregido y §6 "Registro de actos de IA" nuevo** — archivos: `docs/INDEX.md`. Por qué: mantener el mapa y la cadena de actos.
