//! Base de datos SQLite (tablas en espanol, ver DOCUMENTACION.md).

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::path::Path;

const ESQUEMA: &str = "
CREATE TABLE IF NOT EXISTS partidas (
    id_partida    INTEGER PRIMARY KEY,
    fecha         TEXT    NOT NULL,
    duracion_seg  INTEGER NOT NULL,
    modo          TEXT,
    tipo          TEXT,
    cola          INTEGER,
    mapa          INTEGER,
    version_juego TEXT,
    plataforma    TEXT
);

CREATE TABLE IF NOT EXISTS jugadores (
    id_jugador           INTEGER PRIMARY KEY AUTOINCREMENT,
    puuid                TEXT NOT NULL UNIQUE,
    nombre               TEXT,
    etiqueta             TEXT,
    visto_por_ultima_vez TEXT
);

CREATE TABLE IF NOT EXISTS participaciones (
    id_partida           INTEGER NOT NULL REFERENCES partidas(id_partida),
    id_jugador           INTEGER NOT NULL REFERENCES jugadores(id_jugador),
    equipo               INTEGER,
    victoria             INTEGER NOT NULL,
    campeon_id           INTEGER,
    asesinatos           INTEGER,
    muertes              INTEGER,
    asistencias          INTEGER,
    nivel                INTEGER,
    oro_ganado           INTEGER,
    curacion             INTEGER,
    dano_a_campeones     INTEGER,
    dano_recibido        INTEGER,
    tropas_asesinadas    INTEGER,
    monstruos_asesinados INTEGER,
    puntaje_vision       INTEGER,
    dobles               INTEGER,
    triples              INTEGER,
    cuadras              INTEGER,
    pentas               INTEGER,
    objeto_0 INTEGER, objeto_1 INTEGER, objeto_2 INTEGER, objeto_3 INTEGER,
    objeto_4 INTEGER, objeto_5 INTEGER, objeto_6 INTEGER,
    aumento_1 INTEGER, aumento_2 INTEGER, aumento_3 INTEGER,
    aumento_4 INTEGER, aumento_5 INTEGER, aumento_6 INTEGER,
    hechizo_1 INTEGER, hechizo_2 INTEGER,
    PRIMARY KEY (id_partida, id_jugador)
);

CREATE TABLE IF NOT EXISTS partidas_crudas (
    id_partida INTEGER PRIMARY KEY REFERENCES partidas(id_partida),
    json       TEXT NOT NULL
);

-- Datos de cuenta traidos de la API de Riot (comando `perfiles`).
CREATE TABLE IF NOT EXISTS perfiles_riot (
    id_jugador      INTEGER PRIMARY KEY REFERENCES jugadores(id_jugador),
    puuid_riot      TEXT NOT NULL,
    icono_perfil    INTEGER,
    nivel           INTEGER,
    solo_tier       TEXT,
    solo_division   TEXT,
    solo_lp         INTEGER,
    solo_victorias  INTEGER,
    solo_derrotas   INTEGER,
    flex_tier       TEXT,
    flex_division   TEXT,
    flex_lp         INTEGER,
    flex_victorias  INTEGER,
    flex_derrotas   INTEGER,
    actualizado     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_participaciones_jugador ON participaciones(id_jugador);

-- Rankeds flex traidas de match-v5 (comando `flex`). El id es el de Riot (p. ej. LA2_123456).
-- Solo se guardan las participaciones de jugadores del grupo.
CREATE TABLE IF NOT EXISTS flex_partidas (
    id_partida    TEXT PRIMARY KEY,
    fecha         TEXT NOT NULL,
    duracion_seg  INTEGER NOT NULL,
    version_juego TEXT,
    remake        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS flex_participaciones (
    id_partida       TEXT    NOT NULL REFERENCES flex_partidas(id_partida),
    id_jugador       INTEGER NOT NULL REFERENCES jugadores(id_jugador),
    equipo           INTEGER,
    victoria         INTEGER NOT NULL,
    campeon_id       INTEGER,
    posicion         TEXT,
    asesinatos       INTEGER,
    muertes          INTEGER,
    asistencias      INTEGER,
    dano_a_campeones INTEGER,
    oro_ganado       INTEGER,
    tropas           INTEGER,
    puntaje_vision   INTEGER,
    PRIMARY KEY (id_partida, id_jugador)
);

-- Una fila cada vez que cambia el rango flex de un jugador (Riot no entrega historial de LP).
CREATE TABLE IF NOT EXISTS flex_rangos (
    id_jugador INTEGER NOT NULL REFERENCES jugadores(id_jugador),
    fecha      TEXT    NOT NULL,
    tier       TEXT    NOT NULL,
    division   TEXT,
    lp         INTEGER NOT NULL,
    victorias  INTEGER,
    derrotas   INTEGER,
    PRIMARY KEY (id_jugador, fecha)
);

CREATE INDEX IF NOT EXISTS idx_flex_participaciones_jugador ON flex_participaciones(id_jugador);
";

pub fn abrir(ruta: &Path) -> Result<Connection> {
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(ruta).with_context(|| format!("No pude abrir {}", ruta.display()))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(ESQUEMA)?;
    Ok(conn)
}

fn n(v: &Value) -> Option<i64> {
    v.as_i64()
}

/// Guarda una partida completa (partida, jugadores, participaciones y JSON crudo)
/// en una sola transaccion. Devuelve `false` si la partida ya estaba guardada.
pub fn guardar_partida(conn: &mut Connection, g: &Value) -> Result<bool> {
    let id = g["gameId"].as_i64().context("La partida no tiene gameId")?;

    let existe: Option<i64> = conn
        .query_row("SELECT 1 FROM partidas WHERE id_partida = ?1", [id], |r| r.get(0))
        .optional()?;
    if existe.is_some() {
        return Ok(false);
    }

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO partidas (id_partida, fecha, duracion_seg, modo, tipo, cola, mapa, version_juego, plataforma)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            g["gameCreationDate"].as_str().unwrap_or_default(),
            n(&g["gameDuration"]).unwrap_or(0),
            g["gameMode"].as_str(),
            g["gameType"].as_str(),
            n(&g["queueId"]),
            n(&g["mapId"]),
            g["gameVersion"].as_str(),
            g["platformId"].as_str(),
        ],
    )?;

    let identidades = g["participantIdentities"].as_array().context("Sin participantIdentities")?;
    let participantes = g["participants"].as_array().context("Sin participants")?;
    let ahora = g["gameCreationDate"].as_str().unwrap_or_default();

    for p in participantes {
        let pid = &p["participantId"];
        let jugador = identidades
            .iter()
            .find(|i| &i["participantId"] == pid)
            .map(|i| &i["player"])
            .context("Participante sin identidad")?;
        let puuid = jugador["puuid"].as_str().unwrap_or_default();
        if puuid.is_empty() {
            continue;
        }

        // Alta o actualizacion del jugador (el nombre puede cambiar).
        // Solo se actualiza el nombre si esta partida es mas reciente que la ultima vista.
        tx.execute(
            "INSERT INTO jugadores (puuid, nombre, etiqueta, visto_por_ultima_vez)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(puuid) DO UPDATE SET
               nombre = CASE WHEN excluded.visto_por_ultima_vez >= visto_por_ultima_vez
                             THEN excluded.nombre ELSE nombre END,
               etiqueta = CASE WHEN excluded.visto_por_ultima_vez >= visto_por_ultima_vez
                               THEN excluded.etiqueta ELSE etiqueta END,
               visto_por_ultima_vez = MAX(visto_por_ultima_vez, excluded.visto_por_ultima_vez)",
            params![puuid, jugador["gameName"].as_str(), jugador["tagLine"].as_str(), ahora],
        )?;
        let id_jugador: i64 =
            tx.query_row("SELECT id_jugador FROM jugadores WHERE puuid = ?1", [puuid], |r| r.get(0))?;

        let s = &p["stats"];
        let item = |i: u8| n(&s[format!("item{i}")]);
        let aumento = |i: u8| n(&s[format!("playerAugment{i}")]);
        tx.execute(
            "INSERT INTO participaciones (
                id_partida, id_jugador, equipo, victoria, campeon_id,
                asesinatos, muertes, asistencias, nivel, oro_ganado, curacion,
                dano_a_campeones, dano_recibido, tropas_asesinadas, monstruos_asesinados,
                puntaje_vision, dobles, triples, cuadras, pentas,
                objeto_0, objeto_1, objeto_2, objeto_3, objeto_4, objeto_5, objeto_6,
                aumento_1, aumento_2, aumento_3, aumento_4, aumento_5, aumento_6,
                hechizo_1, hechizo_2
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35
             )",
            params![
                id,
                id_jugador,
                n(&p["teamId"]),
                s["win"].as_bool().unwrap_or(false) as i64,
                n(&p["championId"]),
                n(&s["kills"]),
                n(&s["deaths"]),
                n(&s["assists"]),
                n(&s["champLevel"]),
                n(&s["goldEarned"]),
                n(&s["totalHeal"]),
                n(&s["totalDamageDealtToChampions"]),
                n(&s["totalDamageTaken"]),
                n(&s["totalMinionsKilled"]),
                n(&s["neutralMinionsKilled"]),
                n(&s["visionScore"]),
                n(&s["doubleKills"]),
                n(&s["tripleKills"]),
                n(&s["quadraKills"]),
                n(&s["pentaKills"]),
                item(0), item(1), item(2), item(3), item(4), item(5), item(6),
                aumento(1), aumento(2), aumento(3), aumento(4), aumento(5), aumento(6),
                n(&p["spell1Id"]),
                n(&p["spell2Id"]),
            ],
        )?;
    }

    tx.execute(
        "INSERT INTO partidas_crudas (id_partida, json) VALUES (?1, ?2)",
        params![id, serde_json::to_string(g)?],
    )?;
    tx.commit()?;
    Ok(true)
}

/// Totales para mostrar tras cada ejecucion: (partidas, jugadores, participaciones).
pub fn totales(conn: &Connection) -> Result<(i64, i64, i64)> {
    let c = |t: &str| conn.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get::<_, i64>(0));
    Ok((c("partidas")?, c("jugadores")?, c("participaciones")?))
}
