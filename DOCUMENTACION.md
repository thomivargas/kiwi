# Historial de partidas personalizadas de League of Legends

Documentación viva del proyecto. Actualizar al terminar cada hito.
**No guardar aquí contraseñas, tokens ni API keys.**

## 1. Objetivo
Llevar el historial de las partidas personalizadas que juega el grupo de amigos
y sacar estadísticas (winrate, KDA, campeones más jugados, etc.).

## 2. Decisiones y su motivo

| Decisión | Motivo |
|---|---|
| No usar la API pública de Riot | Las personalizadas no están disponibles (ver sección 3). |
| Leer el cliente de League (LCU) | El cliente local sí guarda las personalizadas. |
| Programa colector en Rust | Un `.exe` único, sin dependencias. |
| Fase actual: todo local (SQLite + web local) | Decisión del usuario (2026-11-12): validar primero sin servidor. |
| Servidor remoto | **Pospuesto.** Ver sección 6. |

## 3. Hallazgos sobre la API de Riot (verificados el 2026-11-12)

- Tipos de key: *Development* (24 h, regenerable), *Personal* (sin verificación,
  no apta para uso público) y *Production* (requiere aprobación de Riot).
- Con una Development key funcionan: cuenta por Riot ID (`account-v1`) e historial
  (`match-v5`), región `americas`, plataforma de la cuenta `LA2`.
- **match-v5 no devuelve partidas personalizadas.** El filtro `type` solo acepta
  `ranked, normal, tourney, tutorial` (no existe `custom`). Una personalizada
  reciente consultada por ID devolvió `404 match file not found`.
- Detalle: el ID `LA1_1636984294` existe, pero es una partida ajena de Arena;
  coincidencia de número, no es del grupo. Los prefijos de plataforma importan.
- Conclusión: **la API pública no sirve para este proyecto.**
  La alternativa oficial (Tournament API) exige production key.

## 4. Fuente de datos: cliente de League (LCU)

API interna, local, no oficial. Solo funciona con el cliente abierto y con sesión iniciada.
Puede cambiar sin aviso. Se usa en modo solo lectura.

- Puerto y token: se leen de la línea de comandos del proceso `LeagueClientUx.exe`
  (`--app-port`, `--remoting-auth-token`). Cambian en cada ejecución del cliente.
  Autenticación HTTP Basic con usuario `riot`; certificado propio (hay que aceptarlo).
- Historial: `GET /lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19`
- Detalle de partida: `GET /lol-match-history/v1/games/{gameId}`
  (incluye `gameType: "CUSTOM_GAME"`, participantes y estadísticas de todos los jugadores).

### Limitación crítica
**El cliente solo expone las últimas 20 partidas.** `begIndex` se ignora (probado con
dos endpoints). Si entre dos ejecuciones se juegan más de 20 partidas, las personalizadas
más viejas **se pierden para siempre**. El colector debe ejecutarse con frecuencia
(idealmente en segundo plano).

Una vez conocido el , el detalle se puede pedir aunque ya no esté en las últimas 20.

### Historial de otros invocadores (probado el 2026-11-12)
- `/lol-summoner/v1/alias/lookup?gameName=X&tagLine=Y` da el puuid de otro jugador, y
  `/lol-match-history/v1/products/lol/{puuid}/matches` devuelve **sus** últimas 20 partidas
  (mismo tope; `begIndex`/`endIndex` se ignoran).
- **Las personalizadas (`CUSTOM_GAME`, q3270) solo aparecen en el historial de la cuenta con la
  que se inició sesión en el cliente.** Vistas desde otra cuenta no salen, aunque ese jugador
  las haya jugado (Askeladd visto desde Tehache: 0 personalizadas; desde su propia sesión: 12).
- Consecuencia: para recuperar personalizadas hay que ejecutar `collect` con la sesión de
  cada jugador. Al guardarse por `gameId` no se duplican. El 2026-11-12 se ejecutó con la
  sesión de Askeladd: +10 partidas (5, 7 y 8 de noviembre).

### Datos observados
- Modo de las personalizadas del grupo: `KIWI`, queue `3270`, `gameType: CUSTOM_GAME`.
- Cada personalizada trae a los participantes (hasta 10; a veces el cliente devuelve menos): **basta que una persona la capture**.

## 5. Estado actual

### Hecho
- [x] Verificado acceso a la API de Riot y sus límites.
- [x] Verificado que el LCU entrega las personalizadas.
- [x] Colector en Rust (`lol-collector`).
  - Detecta el cliente, lee las últimas 20 partidas y guarda cada personalizada
    como JSON en `data/games/{gameId}.json`.
  - Deduplica por `gameId`. `--all` guarda también los demás modos.
  - 14 personalizadas capturadas (4 desde la cuenta de Tehache + 10 desde la de Askeladd; del 5 al 12 de noviembre).

- [x] SQLite (2026-11-12): `data/lol.db` con tablas `partidas`, `jugadores`,
  `participaciones` y `partidas_crudas` (esquema en `src/db.rs`).
  - `collect` guarda directo en SQLite (ya no escribe JSON suelto) y deduplica por `gameId`.
  - `importar` carga los JSON anteriores de `data/games/`. Los 4 existentes ya están importados.
  - Verificado al crearla: 4 partidas, 9 jugadores, 28 participaciones, 0 claves foráneas rotas. (Estado actual: 14 partidas, 14 jugadores, 122 participaciones.)
  - El nombre del jugador se actualiza solo con la partida más reciente donde aparece.

- [x] Web local "Sala Kiwi" (2026-11-12), modo `serve` en `http://localhost:8080`
  (solo escucha en 127.0.0.1). Diseño recibido en `diseno-sala-kiwi/` ("Escarcha y oro":
  tema oscuro/claro, azul y rojo por equipo, oro solo para MVP/pentas/foco).
  - Pantallas: Inicio, Partida (pestañas Resumen / Detalladas), Jugador, Estadísticas (Ranking) y Equipos (`/equipos`, reparto previo a jugar).
  - Plantillas Askama en `web/templates/`; CSS, JS y tipografías en `web/static/`,
    incrustados dentro del `.exe` (rust-embed). Tipografías descargadas, sin Google Fonts.
  - `assets` descarga de Data Dragon 173 campeones, 863 objetos y 34 hechizos
    (nombres en español + iconos, ~10 MB) a `data/assets/`. Sin eso la web usa iniciales.
  - Verificado: las 4 páginas responden 200, partida inexistente 404, y la ruta de
    imágenes bloquea `../`. Cifras de la web coinciden con las consultas a SQLite.

- [x] Datos de cuenta desde la API de Riot (2026-11-12), comando `perfiles`: icono de perfil,
  nivel y rango solo/dúo y flex. Se guardan en la tabla `perfiles_riot`; la web lee de ahí y
  nunca llama a Riot. Cada jugador se busca por su Riot ID (el `puuid` del cliente NO sirve
  para la API pública). Plataforma `la2`, enrutado `americas`.
  - Key: se lee, en este orden, de la variable de entorno `RIOT_API_KEY`, del archivo
    `.env` (línea `RIOT_API_KEY=...`) o de `data/riot.key`. Nunca se imprime.
    Hoy está en `.env`, que está en `.gitignore` y no se rastrea en git
    (el repositorio git se creó con `cargo new`).
  - Muestra: avatar real (Inicio, Jugador, Estadísticas), nivel y rangos en el perfil, y una
    columna "Rango solo/dúo" en el ranking. Sin perfil, la web se ve como antes (iniciales).
  - Verificado: error claro si falta la key; renderizado con datos de prueba; y **prueba real
    contra Riot el 2026-11-12 con una key de desarrollo: 9 de 9 jugadores actualizados**, con
    9 iconos de perfil descargados. La key se usó solo en memoria (no se guardó en disco).
  - Nota: una key recién generada puede dar 401 hasta que se use una vez en el portal de
    Riot (pasó dos veces). Los iconos nuevos aparecen sin reiniciar `serve` (ver cambios).

- [x] **Rediseño del Inicio** (2026-11-12): se quitaron las tarjetas de partida. Ahora son
  "Noches recientes" (partidas agrupadas por sesión, una fila compacta por partida: hora,
  quién ganó, marcador, duración, campeones de cada equipo y MVP) más una barra lateral con
  "Top del grupo" (por winrate) y "MVP de la noche". Elegido por el usuario entre 3 opciones.
  - Una "noche" cambia de día a las **06:00 hora local** (así una sesión que pasa de medianoche
    no se parte en dos).
  - Muestra las últimas 40 partidas. En pantallas angostas la barra lateral pasa debajo.
  - Las 4 pantallas se revisaron visualmente con capturas reales (2026-11-12) y se ven bien.
- [x] **Caché del navegador:** los enlaces a CSS/JS llevan `?v=<número>` que cambia en cada
  arranque de `serve`. Sin eso el navegador conservaba el CSS viejo (max-age 1 h) y la página
  nueva se veía sin estilos.

- [x] **Ranking con puntos del grupo** (2026-11-12), página `/estadisticas` (menú "Ranking").
  Inspirado en una captura de referencia del usuario (ladder de LoL), pero con puntos propios
  en lugar de LP de Riot. Podio con los 3 primeros (oro/plata/bronce: puntos, V–D, partidas,
  4 campeones más jugados, KDA) y tabla completa ordenable (puntos, partidas, winrate con
  barra, KDA, daño medio, más jugados). El rango de Riot queda como dato chico bajo el nombre,
  **sin LP** (se confundiría con los puntos). El perfil del jugador muestra sus puntos y puesto,
  y el "Top del grupo" del Inicio también usa puntos.
  - **Sistema de puntos: Elo** (decidido con el usuario). Todos empiezan con **1000**, K = **32**.
    Se recalcula desde todo el historial en orden cronológico (no hay tabla de puntos): para
    cada partida, la media de puntos de cada equipo da el resultado esperado
    `E = 1 / (1 + 10^((R_rival - R_propio)/400))` y cada jugador del equipo suma/resta
    `K * (resultado - E)`. Partidas sin ganador claro no cuentan.
  - Se probó un peso por rango de Riot (alto rango gana menos) y el usuario lo descartó: se volvió al Elo simple. El modal de ayuda quedó con texto mínimo.
  - Verificado: los puntos de los 9 jugadores coinciden con un cálculo independiente en Node.
  - Se quitaron de la página las tarjetas de líderes y las columnas de dobles/triples/cuádruples/
    pentas (siguen en el perfil de cada jugador).
  - Los puntos se muestran redondeados, pero el orden usa el valor exacto: dos jugadores pueden
    verse con los mismos puntos y quedar en distinto puesto.

- [x] **Ajustes de interfaz** (2026-11-12):
  - **Inicio sin tarjetas de resumen.** Queda el "Historial" de todas las partidas, agrupado por
    noche, con **paginación por noches** (5 por página, para no partir una noche entre dos
    páginas) y la barra lateral (Top del grupo por puntos, MVP de la noche). `/?pagina=N`;
    un número fuera de rango se ajusta al más cercano y un valor inválido cae en la página 1.
    El paginador solo aparece si hay más de una página. Probado con una copia de la base con
    14 noches (3 páginas); esa copia ya se borró.
  - **Botón de ayuda "?" en el Ranking** que abre una ventana (`<dialog>` nativo: Esc, clic
    fuera o botón cerrar) explicando los puntos. Ahí se movió la nota que estaba bajo la tabla.
  - **Sin pie de página** en ninguna pantalla. OJO: ese pie incluía la leyenda "Esta web no está
    respaldada por Riot Games…", que la política de Riot pide en productos publicados. Mientras
    la web sea solo local/privada no hay problema; si algún día se publica, hay que reponerla.
  - Se quitaron textos de pista: "Pulsa una columna para ordenar", "Haz clic en una columna…",
    "Barra = winrate…", "Últimas N de M", "Datos de cuenta de Riot actualizados…" y el
    "Historial del grupo" del Inicio.
  - **No verificado visualmente:** la ventana modal y el paginador (la extensión del navegador
    se desconectó). Sí se verificó la estructura HTML, el CSS/JS servidos y la lógica del paginador.

- [x] **Rankeds flex** (2026-11-12): rutas `/flex` (grupo) y `/flex/{id}` (jugador), menú "Flex".
  Datos de Riot **match-v5** (cola 440), no del LCU. Comando `flex [--max N] [--jugadores ids]`: por cada jugador con perfil (o solo los ids indicados)
  pide hasta N ids (def. 200), descarga el detalle de los nuevos (pausa de 1,25 s por la key de desarrollo;
  cada partida se guarda sola, se puede cortar y retomar) y registra el rango flex actual.
  - Tablas: `flex_partidas`, `flex_participaciones` (solo jugadores del grupo; los remakes se guardan
    marcados y no cuentan) y `flex_rangos` (una fila cada vez que cambia el rango; `perfiles` también la alimenta).
  - **Riot no da historial de LP:** la evolución del rango solo existe desde la primera ejecución;
    el gráfico aparece cuando hay 2 o más registros. Hay que ejecutar `flex`/`perfiles` con regularidad.
  - Grupo: clasificación por rango (racha, forma, más jugados), evolución de rango, partidas por semana,
    mapa de calor día/hora, mejores dúos (≥3 partidas en el mismo equipo) y últimas partidas.
    Jugador: rango, KDA, daño, farmeo, rachas, winrate acumulado, campeones, posiciones e historial.
  - Código: `src/flex.rs` (descarga), `src/flex_consultas.rs` (datos), `src/flex_graficos.rs` (SVG
    con colores del tema), plantillas `flex*.html`, sección 20 de `styles.css`.
  - Primera prueba real: `--max 5` con 11 jugadores (se excluyeron los ids 3, 32 y 33): 29 partidas, 96 participaciones.
    No verificado visualmente: la extensión del navegador no estaba conectada.

### Reglas de cálculo (decididas con el usuario)
- **MVP** de cada partida: mejor puntaje combinado de TODOS los jugadores (no solo del
  ganador): 50 % KDA + 35 % daño a campeones + 15 % oro, cada uno relativo al máximo de
  esa partida. KDA = (asesinatos + asistencias) / max(muertes, 1).
- Color del KDA: dorado desde 5,0; normal desde 3,0.
- Líderes de "Estadísticas": exigen al menos 2 partidas (si nadie llega, entran todos).
- Fechas: se guardan en UTC y se muestran en la hora local del PC.

### Pendiente
- [ ] **Confirmar que la key del `.env` es la Personal key.** El usuario indicó que lo es;
  conviene comprobar en el portal que no tiene fecha de caducidad de 24 h (una key de
  desarrollo sí caduca, y entonces `perfiles` daría 401). Si un jugador cambia de Riot ID,
  repetir `perfiles`.
- [ ] **Revisión del usuario**: las capturas las revisé yo, pero falta que el usuario abra la
  web y diga qué ajustar (detalles ya vistos: la insignia "0 Pentas" sale dorada aunque sea 0;
  en el MVP de la noche, un empate se resuelve de forma arbitraria).
- [ ] Aumentos (ARAM con aumentos): solo se ven como cuadros de color; no hay nombre ni
  icono (Data Dragon no los incluye).
- [ ] 7 de las 14 partidas tienen menos de 10 jugadores guardados (el cliente no devolvió todas las identidades); cuentan para los puntos solo con los jugadores que sí están.
- [ ] Cada amigo con partidas personalizadas debería ejecutar `collect` con su sesión (o pasar su base) para recuperar las que se salieron de los últimos 20 de otras cuentas.
- [ ] Ejecución en segundo plano del colector (detectar fin de partida).

## 6. Servidor (pospuesto)
Publicar la web en un servidor remoto queda para más adelante: falta decidir dominio, forma de despliegue y base de datos. Si la web se publica, reponer la leyenda legal de Riot.

## 7. Cómo trabajar con el proyecto

```powershell
# cargo no está en el PATH de algunas sesiones; añadirlo así:
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

cd C:\Users\th\Desktop\svg\collector
cargo build --release
.\target\release\lol-collector.exe collect          # lee el cliente, guarda personalizadas
.\target\release\lol-collector.exe collect --all    # todos los modos
.\target\release\lol-collector.exe importar [carpeta]  # importa JSON (def. data/games)
.\target\release\lol-collector.exe assets           # descarga nombres e iconos (una vez)
.\target\release\lol-collector.exe perfiles         # icono, nivel y rango desde Riot (necesita key)
.\target\release\lol-collector.exe flex [--max N] [--jugadores 1,2]  # rankeds flex y rango (necesita key y perfiles)
.\target\release\lol-collector.exe serve [--puerto N]  # web en http://localhost:8080
```

Para `perfiles`, la key va en `.env` como `RIOT_API_KEY=...` (o en la variable de
entorno de la sesión). No la pegues en el chat ni la subas a ningún repositorio. 

Nota: Windows no deja recompilar mientras `serve` está en ejecución; detenerlo antes
(`Stop-Process -Name lol-collector`).

`collect` requiere el cliente de League abierto y con sesión iniciada.
`importar` no lo necesita. No hace falta reiniciar `serve` tras `collect`, `perfiles` o `assets` (las imágenes se buscan en disco en cada consulta). Solo hay que reiniciarlo si se recompila. Se ejecuta desde la raíz del repo (la base se crea en `data/lol.db`).

## 8. Estructura
```
├── Cargo.toml
├── askama.toml         # carpeta de plantillas: web/templates
├── src/main.rs         # comandos: collect, importar, assets, serve
├── src/lcu.rs          # acceso al cliente de League
├── src/db.rs           # esquema y guardado en SQLite
├── src/catalogo.rs     # nombres e iconos (Data Dragon)
├── src/riot.rs         # API de Riot: perfiles (icono, nivel, rango)
├── src/consultas.rs    # consultas + datos formateados para las plantillas
├── src/web.rs          # servidor web (axum) y rutas
├── web/templates/      # base, macros, inicio, partida, jugador, estadisticas, equipos
├── web/static/         # styles.css, app.js, fonts.css y fonts/
└── data/
    ├── lol.db          # base de datos
    ├── assets/         # iconos descargados (campeones, objetos, hechizos)
    └── games/          # JSON de las primeras capturas (ya importados)
```

`diagrama-proyecto.html` (en la raíz) muestra el flujo, las tablas, las pantallas
y la diferencia con la fase del VPS.

## 9. Registro de cambios
- 2026-11-12: verificación de la API de Riot; descubrimiento del LCU; primer colector;
  decisión de trabajar en local.
- 2026-11-12: diagrama del proyecto aprobado en lo esencial; implementada la base SQLite
  y el comando `importar`. Archify no se instaló (bloqueado por el sistema); el diagrama
  se hizo con la skill de diagramas integrada.
- 2026-11-12: ruta nueva **/equipos** (menú "Equipos"): se marcan jugadores y la web propone
  3 repartos en dos equipos con la menor diferencia de puntos Elo (todas las combinaciones,
  máx. 12 jugadores; con número impar el equipo menor es n/2). Por equipo muestra media,
  probabilidad de ganar y los puntos que gana/pierde cada integrante (K=32, misma fórmula del
  ranking). Es una consulta GET (`?j=1&j=2...`), no guarda nada. Código: `equipos` en
  `consultas.rs`, plantilla `equipos.html`, sección 19 de `styles.css`. Verificado contra un
  cálculo independiente (diferencia mínima con 8 jugadores). Se probó y descartó un peso por
  rango de Riot en los puntos; el modal de ayuda quedó con texto mínimo.
- 2026-11-12: se probó el historial de otros invocadores por LCU (ver sección 4: las personalizadas
  solo salen desde la propia cuenta). Con la sesión de Askeladd se ejecutó `collect`: 14 partidas,
  14 jugadores, 122 participaciones (copia previa en `data/lol.db.bak-antes-collect`). Se ejecutó
  `perfiles` para los 14 (5 nuevos: FferR, bauticapo1, Palmer, Kurasáo, Riven Plate). Los avatares no
  se veían hasta reiniciar el servidor (imágenes escaneadas solo al arrancar). Corregido después: `Catalogo::imagen`
  ahora comprueba en disco si existe el archivo en cada consulta (sin lista en memoria), así
  los iconos nuevos aparecen sin reiniciar. Verificado ocultando y devolviendo un icono con el
  servidor en marcha.

- 2026-11-12: preparación para repositorio público: docs movidos a la raíz del repo, README, CONTRIBUTING, LICENSE (MIT), `.env.example`, CI (`cargo check`), skills (`verificar`, `base-de-datos`, `pulir-ui`, `nuevo-dato`), reglas y agentes de revisión en `.claude/`, `AGENTS.md`. Se retiraron de la documentación los datos de infraestructura privada.
