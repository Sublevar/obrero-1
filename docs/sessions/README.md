# docs/sessions/ — trazabilidad de sesiones de trabajo con IA

Un archivo por sesión de trabajo asistida por IA que modificó al menos un archivo del repo. **No hay sesiones exentas**: un typo, un cambio rápido o un cambio ya acordado en otro lado también se registran, como un acto más en la "Bitácora de actos" (acoplamiento forzado, ver `CLAUDE.md` §5). Las secciones de decisiones y lo que queda abierto pueden ser una línea si la sesión fue chica; la bitácora no se omite nunca.

Cada acto se registra en el mismo paso en que se hace, en dos lugares: su entrada acá y su fila en `docs/INDEX.md` §6. El hook `Stop` de `.claude/settings.json` bloquea el cierre del turno si falta alguno de los dos.

## Nombre de archivo

`YYYY-MM-DD-slug.md` — fecha de la sesión (no de cuando se escribe, si difieren), slug corto describiendo el tema.

## Qué contiene (plantilla)

```markdown
# <título corto>

**Fecha:** YYYY-MM-DD
**Participantes:** <quién pidió el trabajo> + <qué agente/modelo>
**Alcance:** una frase — qué se decidió hacer en esta sesión.

## Qué se leyó

Lista de lo que se consultó como insumo: archivos del repo, specs previos, páginas externas de referencia. Si algo externo influyó una decisión, va acá con el link.

## Decisiones

Lista de las decisiones tomadas, cada una con su "por qué" en una línea si no es obvio. Si una decisión resuelve una ambigüedad de un documento anterior, decirlo explícitamente (qué decía antes, qué dice ahora).

## Queda abierto

Lo que se identificó pero se dejó deliberadamente para después, y por qué no se resolvió ahora.

## Artefactos de esta sesión

Qué archivos se crearon/editaron como resultado (specs, docs, código).

## Bitácora de actos

Una entrada por acto, en orden, agregada en el mismo paso en que se hace el acto. Cada una tiene su fila en `docs/INDEX.md` §6.

1. **<qué se hizo>** — archivos: `<rutas>`. Por qué: <una línea>.
```

## Por qué existe esto

El repo ya tenía una convención parecida para `docs/product/`/`docs/engineering/` (autoría de rol + fecha + estado en el encabezado de cada doc) — eso documenta *qué* se decidió. Esto documenta *el proceso de la sesión* que llegó a esa decisión, incluyendo lo que se leyó y lo que se descartó, que no siempre queda en el documento final.
