# Base de datos

- Fuente del esquema: `src/db.rs`. Usa `CREATE TABLE IF NOT EXISTS`, así que una base nueva se crea sola al primer comando.
- Preferir cambios aditivos (tabla o columna nueva). No renombrar ni borrar columnas con datos.
- Cada colaborador tiene su propia `data/lol.db`; no existe una base compartida ni se sube al repo.
- El LCU solo da las últimas 20 partidas: lo que se pierde no se recupera. Nunca borrar `partidas_crudas`.
- Antes de cualquier migración que modifique filas, copia: `data/lol.db.bak-<motivo>`.
- Fechas en UTC. La "noche" cambia a las 06:00 local.
- La web solo lee de SQLite; los datos de Riot entran con comandos (`perfiles`, `flex`, `agregar`).
- Procedimiento detallado: skill `base-de-datos`.
