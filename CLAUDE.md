# Sala Kiwi — contexto para agentes

Historial y estadísticas de las partidas personalizadas de League of Legends de un grupo de amigos. Colector en Rust que lee el cliente de League (LCU), guarda en SQLite y sirve una web local.

## Lectura por alcance

Leer solo lo necesario:

- Siempre: este archivo.
- Decisiones, hallazgos, reglas de cálculo y pendientes: `DOCUMENTACION.md` (secciones 2, 5 y 9). Leerlo antes de tocar algo no trivial.
- Flujo, tablas y pantallas: `diagrama-proyecto.html`.
- Diseño visual ("Escarcha y oro"): `diseno-sala-kiwi/`. Respetarlo en pantallas nuevas.
- Testing y validación: `.claude/rules/testing.md`.
- Base de datos: `.claude/rules/base-de-datos.md` y skill `base-de-datos`.
- Cambios de UI: skill `pulir-ui`.
- Traer un dato nuevo de Riot o del cliente: skill `nuevo-dato`.
- Commits y PR: `.claude/rules/commits.md` (sin atribución a IA).
- Contribuir: `CONTRIBUTING.md`.

No leer todo `DOCUMENTACION.md` por defecto. El código manda sobre la documentación: buscar primero con `rg` en `src/` y `web/`.

## Repositorio

```text
src/main.rs            comandos: collect, importar, assets, perfiles, agregar, flex, serve
src/lcu.rs             acceso al cliente de League
src/db.rs              esquema SQLite y guardado
src/riot.rs            API de Riot (perfiles: icono, nivel, rango solo/dúo y flex)
src/flex*.rs           rankeds flex: captura, consultas y gráficos
src/consultas.rs       consultas y datos formateados para plantillas
src/web.rs             axum + rutas
src/catalogo.rs        nombres e iconos (Data Dragon)
web/templates/         Askama (base, macros, inicio, partida, jugador, estadisticas, equipos, flex)
web/static/            styles.css, app.js, fuentes (incrustados en el .exe con rust-embed)
data/                  lol.db, assets/, games/ (local, ignorado por git)
```

Un solo crate `lol-collector` (edition 2024). Ejecutar los comandos desde la raíz del repo.

## Comandos

Si `cargo` no está en el PATH: `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.

```powershell
cargo build --release
.\target\release\lol-collector.exe serve        # http://localhost:8080 (solo 127.0.0.1)
.\target\release\lol-collector.exe collect      # requiere cliente de League abierto
.\target\release\lol-collector.exe perfiles     # requiere RIOT_API_KEY
```

- Windows no deja recompilar con `serve` en ejecución: `Stop-Process -Name lol-collector` antes.
- Reiniciar `serve` solo tras recompilar; `collect`, `perfiles` y `assets` no lo requieren.
- Lista completa de comandos: `DOCUMENTACION.md` sección 7.

## Trabajo proporcional

- Clasificar SMALL / NORMAL / CRITICAL según si el cambio toca la base de datos (criterio en `.claude/rules/testing.md`). Antes de correr builds o pruebas pesadas, proponerlos al usuario y esperar su aprobación.
- Leer primero símbolos, funciones, tipos y dependencias directas. Ampliar solo ante una dependencia real.
- Reutilizar lecturas y validaciones mientras sigan vigentes; agrupar lecturas independientes.
- Resolver la necesidad actual. Una mejora no pedida se informa como «Mejora futura», no se implementa.
- Subagentes solo con ventaja concreta (investigación independiente, contexto excesivo, revisión de un ámbito sensible). Sin ceremonia de specs ni planes para cambios pequeños.
- Terminar cuando el comportamiento pedido y las validaciones aprobadas estén completos. Decir explícitamente qué se corrió y qué no se pudo verificar.

## Reglas

- **Secretos:** la key de Riot vive en `.env` (o `RIOT_API_KEY`, o `data/riot.key`). Nunca leerla en voz alta, imprimirla, copiarla a docs, commits ni logs. No tocar `.gitignore` para exponerla. `.claude/settings.json` bloquea leer y editar esos archivos.
- **La web nunca llama a Riot:** lee de SQLite (`perfiles_riot`, etc.). Los datos de Riot se traen con el comando `perfiles` (o uno análogo) y se guardan.
- **Histórico propio:** el LCU solo expone las últimas 20 partidas; todo lo que dependa de historia debe persistirse en la base, no recalcularse desde fuente externa. Antes de migrar o reescribir `lol.db`, hacer copia (`data/lol.db.bak-*`).
- Esquema y cambios de tablas en `src/db.rs`; no inventar columnas fuera de ahí. Preferir cambios aditivos.
- Fechas en UTC en la base; se muestran en hora local. Una "noche" cambia a las 06:00 local.
- Reglas de cálculo ya decididas (Elo K=32 inicio 1000, MVP 50/35/15, KDA): ver `DOCUMENTACION.md` sección 5. No cambiarlas sin pedirlo.
- Rangos de Riot: mostrar sin LP junto a los puntos del grupo para no confundirlos.
- Una responsabilidad por módulo: consultas nuevas en `consultas.rs`, rutas en `web.rs`, plantilla propia en `web/templates/`, estilos en una sección nueva de `styles.css`.
- Sin textos de pista ni pies de página innecesarios: que el diseño explique.
- Datos personales: no subir bases, JSON de partidas ni capturas con nombres reales al repositorio.
- Actualizar `DOCUMENTACION.md` (secciones 5 y 9) al terminar cada hito.
- Commits y PR: nunca mencionar que están hechos con IA (ni `Co-Authored-By` ni "Generated with"). Ver `.claude/rules/commits.md`.
- Idioma de código visible al usuario, nombres de tablas y documentación: español.
