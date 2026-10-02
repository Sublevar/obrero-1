# docs/ — cómo se escriben los documentos

La documentación y los tutoriales de obrero-1 los escriben humanxs. Los agentes de IA solo escriben trazabilidad (`docs/INDEX.md` y `docs/sessions/`) — ver `CLAUDE.md` §4–§5.

## Dónde va cada cosa

| Tipo | Lugar |
|---|---|
| Producto / UX | `docs/product/` |
| Plan de ingeniería ligado 1:1 a un spec de producto | `docs/engineering/` |
| Arquitectura fundacional, menos formal (se itera antes de que exista un "producto" alrededor, ver `specs/clock.md`) | `specs/` |
| Bitácora de sesión de trabajo con IA | `docs/sessions/` (convención en `docs/sessions/README.md`) |
| Mapa del repo | `docs/INDEX.md` |

## Encabezado

Documentos en `docs/product/` y `docs/engineering/` llevan autoría de rol + fecha + estado:

```
**obrero-1, <alcance>.** Author: <rol>, <fecha>. Status: <proposal | agreed baseline | ready for execution>.
```

Ver `firmware-mvp-0.md`, `timeline-view-design-spec.md`. En `specs/` el encabezado es libre, pero se recomienda el mismo.

Un plan en `docs/engineering/` referencia explícitamente su spec fuente con la fecha/estado de ese spec.

## Índice

Agregar, borrar o renombrar un documento de `docs/` o `specs/` actualiza `docs/INDEX.md` en el mismo commit.
