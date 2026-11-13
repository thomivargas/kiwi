---
name: base-de-datos
description: Use when changing the SQLite schema, saving new data, migrating or recalculating tables in Sala Kiwi's lol.db, or when a contributor needs to create their own database.
---

# Base de datos — Sala Kiwi

Reglas cortas: `.claude/rules/base-de-datos.md`. Esquema: `src/db.rs`.

## Crear una base propia (colaboradores)

No hay que migrar nada: la base se crea sola en `data/lol.db` al ejecutar un comando.

```powershell
cargo run --release -- assets      # nombres e iconos (una vez)
cargo run --release -- collect     # requiere el cliente de League abierto y con sesión
cargo run --release -- serve       # http://localhost:8080
```

Tu base solo tendrá las partidas que tu cliente todavía expone (últimas 20). El histórico del dueño no se comparte; si hace falta, el dueño pasa JSON con `importar`.

## Cambiar el esquema

1. Editar solo `src/db.rs`, de forma aditiva (`CREATE TABLE IF NOT EXISTS`, columna nueva con valor por defecto).
2. Copia previa: `Copy-Item data\lol.db data\lol.db.bak-<motivo>`.
3. Probar sobre la copia; comprobar que una base vacía también arranca.
4. Si el cambio modifica filas existentes es CRITICAL (`.claude/rules/testing.md`): comparar resultados antes y después.
5. Anotar el cambio en `DOCUMENTACION.md` (secciones 5 y 9).

## Nunca

- Borrar o reescribir `partidas_crudas`.
- Subir `lol.db`, sus copias `.bak-*` ni los JSON de `data/games/` (contienen nombres reales).
