# docs/sessions/ — trazabilidad de sesiones de trabajo con IA

Un archivo por sesión de trabajo asistida por IA que tomó decisiones significativas: de arquitectura, de producto, o que fijó qué queda explícitamente fuera de alcance. No es para cada sesión — una sesión que solo corrige un typo o aplica un cambio ya acordado en otro lado no necesita entrada. Es para cuando alguien, meses después, va a preguntarse "¿por qué quedó así?" y la respuesta no está en el código ni en el commit.

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
```

## Por qué existe esto

El repo ya tenía una convención parecida para `docs/product/`/`docs/engineering/` (autoría de rol + fecha + estado en el encabezado de cada doc) — eso documenta *qué* se decidió. Esto documenta *el proceso de la sesión* que llegó a esa decisión, incluyendo lo que se leyó y lo que se descartó, que no siempre queda en el documento final.
