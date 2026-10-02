# docs/ — cómo se escriben los documentos

Acá conviven dos cosas distintas (ver `CLAUDE.md` §4):

- **Especificaciones** (`specs/`, `docs/product/`, `docs/engineering/`): descripciones técnicas. Se actualizan junto con cada cambio aprobado que las impacte, también cuando el cambio lo hace un agente de IA.
- **Documentación y tutoriales**: todavía no existen. Los van a escribir humanxs.

La trazabilidad (`docs/INDEX.md`, `docs/sessions/`) la mantiene quien hace el cambio, persona o agente.

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

Ver `firmware-mvp-0.md`, `timeline-view-design-spec.md`. En `specs/` el encabezado es libre, pero se recomienda el mismo. Al actualizar una spec por un cambio aprobado, se actualizan también su fecha y su estado.

Un plan en `docs/engineering/` referencia explícitamente su spec fuente con la fecha/estado de ese spec.

## Índice

Agregar, borrar o renombrar un documento de `docs/` o `specs/` actualiza `docs/INDEX.md` en el mismo commit.
