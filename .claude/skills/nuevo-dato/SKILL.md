---
name: nuevo-dato
description: Use when adding a new data point from the League client (LCU) or the Riot API to Sala Kiwi, such as a new stat, a new ranked field or a new command that fetches and stores data.
---

# Nuevo dato — Sala Kiwi

Flujo: **fuente → comando → SQLite → consulta → plantilla**. La web nunca llama a Riot.

1. Confirmar en `DOCUMENTACION.md` (secciones 3 y 4) que la fuente expone el dato y sus límites (el LCU solo da las últimas 20 partidas).
2. Fuente: cliente en `src/lcu.rs`, API de Riot en `src/riot.rs`. La key se lee del entorno; nunca imprimirla ni registrarla.
3. Persistir en `src/db.rs`, aditivo (ver skill `base-de-datos`). Guardar el dato crudo si la fuente es perecedera.
4. Exponer con un comando (nuevo o existente en `src/main.rs`), no desde la web.
5. Consulta en `src/consultas.rs` (o `flex_consultas.rs`), plantilla y estilos según skill `pulir-ui`.
6. Probar con una copia de la base; los pasos que necesitan cliente o key, decir que no se verificaron si no hay.
7. Registrar en `DOCUMENTACION.md` (secciones 5 y 9).
