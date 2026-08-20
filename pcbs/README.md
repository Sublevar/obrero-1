

Ajustar la configuración de backups (KiCad V10 o +)

En las versiones más recientes (KiCad V10), KiCad tiene un sistema de control de versiones local que crea la carpeta .history. Puedes desactivar esta función para que no se genere en primer lugar:

Abre KiCad y ve al menú Preferencias → Preferencias.
Busca la sección Control de versiones.
Desmarca la opción "Habilitar seguimiento Git" (o similar, "Enable Git tracking").
Ten en cuenta que, aunque desactives esta opción, es muy recomendable mantener la carpeta .history en tu .gitignore como respaldo.