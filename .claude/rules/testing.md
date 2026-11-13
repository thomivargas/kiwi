# Testing y validación — política canónica

`CLAUDE.md` define alcance y eficiencia; esta es la única fuente normativa de validación local.

## Aprobación antes de correr

Antes de correr `cargo build --release`, tests o cualquier prueba pesada, proponer al usuario la lista concreta (comando, alcance, riesgo que cubre) y esperar su aprobación. `cargo check` es ligero y se puede correr sin preguntar. Si algo se descarta, informar en el cierre qué quedó sin verificar.

## Riesgo y alcance

Clasificar por **si el cambio toca la base de datos**:

| Nivel | Criterio | Validación a proponer |
|---|---|---|
| SMALL | No toca la base: plantillas, estilos, textos, formato, consultas de solo lectura sin cambio de esquema | `cargo check` y verificación visual si hay UI |
| NORMAL | Base de forma aditiva: tabla o columna nueva, escrituras nuevas | `cargo check`, `cargo test` y prueba sobre una copia de `lol.db` |
| CRITICAL | Modifica filas existentes: backfills, migraciones que cambian datos, recálculo de Elo/puntos | Copia previa `data/lol.db.bak-*`, prueba sobre la copia, comparar resultados antes y después |

## Desarrollo y cierre

- Un bug necesita reproducción (test o caso concreto sobre datos reales) antes del fix.
- Reglas de cálculo (Elo, MVP, KDA): cubrir con test unitario sobre valores conocidos; no se cambian sin pedirlo (`DOCUMENTACION.md` sección 5).
- No crear tests para getters, formato trivial, plantillas o glue sin lógica.
- Cambios de UI: abrir `serve` y revisar la pantalla afectada (ver skill `pulir-ui`). Si no se puede, decirlo explícitamente.
- `collect` y `perfiles` dependen del cliente de League y de la key de Riot: no se pueden verificar sin ellos; informarlo.
- Un resultado verde sigue válido mientras no cambie código relacionado. No repetir por rutina.
- Windows bloquea recompilar con `serve` abierto: `Stop-Process -Name lol-collector` antes.

| Qué | Comando (desde la raíz) |
|---|---|
| Tipos y errores | `cargo check` |
| Tests | `cargo test` |
| Lints | `cargo clippy` |
| Build | `cargo build --release` |
