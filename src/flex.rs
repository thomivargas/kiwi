//! Rankeds flex desde la API publica de Riot (match-v5) y snapshots del rango.
//!
//! `lol-collector flex [--max N]` pide, para cada jugador con perfil (`perfiles`), los ids de sus
//! ultimas partidas flex (cola 440), descarga el detalle de las que aun no estan guardadas y
//! registra el rango actual. Guarda en SQLite; la web nunca llama a Riot.
//! La key se lee igual que en `perfiles`. Nunca se imprime.

use crate::riot::{self, Api, PLATAFORMA, REGION};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Url;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    thread,
    time::Duration,
};

const COLA_FLEX: i64 = 440;
const POR_PAGINA: usize = 100;
/// Partidas por jugador que se piden por defecto (match-v5 pagina de a 100).
const MAX_POR_DEFECTO: usize = 200;
/// Una key de desarrollo permite 100 peticiones cada 2 minutos: se deja ~1,25 s entre llamadas.
const PAUSA: Duration = Duration::from_millis(1250);
/// Bajo este tiempo (o con rendicion temprana) la partida es un remake y no cuenta.
const DURACION_MINIMA: i64 = 300;

/// Guarda el rango flex actual solo si cambio respecto al ultimo registro del jugador.
pub fn registrar_rango(conn: &Connection, id_jugador: i64, rango: Option<&riot::Rango>) -> Result<()> {
    let Some(r) = rango else { return Ok(()) };
    let ultimo: Option<(String, Option<String>, i64, Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT tier, division, lp, victorias, derrotas FROM flex_rangos
             WHERE id_jugador = ?1 ORDER BY fecha DESC LIMIT 1",
            [id_jugador],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?, f.get(4)?)),
        )
        .optional()?;
    let igual = ultimo.is_some_and(|(t, d, lp, v, de)| {
        t == r.tier && d.as_deref().unwrap_or("") == r.division && lp == r.lp && v == Some(r.victorias) && de == Some(r.derrotas)
    });
    if igual {
        return Ok(());
    }
    conn.execute(
        "INSERT OR REPLACE INTO flex_rangos (id_jugador, fecha, tier, division, lp, victorias, derrotas)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id_jugador,
            Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            r.tier,
            r.division,
            r.lp,
            r.victorias,
            r.derrotas
        ],
    )?;
    Ok(())
}

pub fn flex(conn: &mut Connection, max: Option<usize>, solo: Option<Vec<i64>>) -> Result<()> {
    let max = max.unwrap_or(MAX_POR_DEFECTO).max(1);
    let api = Api::nueva()?;

    let jugadores: Vec<(i64, String, String)> = {
        let mut st = conn.prepare(
            "SELECT j.id_jugador, COALESCE(j.nombre,''), p.puuid_riot
             FROM perfiles_riot p JOIN jugadores j USING (id_jugador) ORDER BY j.id_jugador",
        )?;
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?
    };
    let jugadores: Vec<_> = jugadores.into_iter().filter(|(id, _, _)| solo.as_ref().is_none_or(|s| s.contains(id))).collect();
    if jugadores.is_empty() {
        bail!("No hay perfiles para procesar. Ejecuta primero `lol-collector perfiles` o revisa --jugadores.");
    }
    let por_puuid: HashMap<String, i64> = jugadores.iter().map(|(id, _, puuid)| (puuid.clone(), *id)).collect();

    // 1) Ids de partidas nuevas (sin repetir entre jugadores).
    let mut pendientes: Vec<String> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for (id_jugador, nombre, puuid) in &jugadores {
        let mut nuevas = 0;
        let mut total = 0;
        let mut inicio = 0;
        while inicio < max {
            let cuenta = POR_PAGINA.min(max - inicio);
            let url = Url::parse(&format!(
                "https://{REGION}.api.riotgames.com/lol/match/v5/matches/by-puuid/{puuid}/ids?queue={COLA_FLEX}&start={inicio}&count={cuenta}"
            ))?;
            thread::sleep(PAUSA);
            let ids: Vec<String> = match api.get(url)? {
                Some(v) => serde_json::from_value(v).context("Lista de partidas con formato inesperado")?,
                None => Vec::new(),
            };
            total += ids.len();
            for id in &ids {
                // Ya resuelta: tiene la participacion de este jugador o es un remake. Si la partida estaba
                // guardada por otro jugador pero no por este, se vuelve a pedir para agregar su participacion.
                let guardada: Option<i64> = conn
                    .query_row(
                        "SELECT 1 FROM flex_participaciones WHERE id_partida = ?1 AND id_jugador = ?2
                         UNION SELECT 1 FROM flex_partidas WHERE id_partida = ?1 AND remake = 1",
                        params![id, id_jugador],
                        |r| r.get(0),
                    )
                    .optional()?;
                if guardada.is_none() && vistos.insert(id.clone()) {
                    pendientes.push(id.clone());
                    nuevas += 1;
                }
            }
            if ids.len() < cuenta {
                break;
            }
            inicio += cuenta;
        }
        println!("- {nombre}: {total} partidas flex ({nuevas} por descargar)");
    }

    // 2) Detalle de cada partida nueva. Cada una se guarda por separado: si se corta, se retoma.
    let (mut guardadas, mut ajenas) = (0, 0);
    let total = pendientes.len();
    for (i, id) in pendientes.iter().enumerate() {
        thread::sleep(PAUSA);
        let url = Url::parse(&format!("https://{REGION}.api.riotgames.com/lol/match/v5/matches/{id}"))?;
        let Some(detalle) = api.get(url)? else {
            println!("  [{}/{total}] {id}: Riot no la devuelve", i + 1);
            continue;
        };
        match guardar_partida(conn, &detalle, &por_puuid)? {
            Some(n) => {
                guardadas += 1;
                println!("  [{}/{total}] {id}: {n} del grupo", i + 1);
            }
            None => ajenas += 1,
        }
    }

    // 3) Rango flex actual de cada jugador.
    let host = format!("https://{PLATAFORMA}.api.riotgames.com");
    for (id, nombre, puuid) in &jugadores {
        thread::sleep(PAUSA);
        let entradas = api.get(Url::parse(&format!("{host}/lol/league/v4/entries/by-puuid/{puuid}"))?)?.unwrap_or(Value::Null);
        registrar_rango(conn, *id, riot::rango(&entradas, "RANKED_FLEX_SR").as_ref()).with_context(|| format!("Rango de {nombre}"))?;
    }

    let (partidas, participaciones): (i64, i64) = (
        conn.query_row("SELECT COUNT(*) FROM flex_partidas WHERE remake = 0", [], |r| r.get(0))?,
        conn.query_row("SELECT COUNT(*) FROM flex_participaciones", [], |r| r.get(0))?,
    );
    println!("\nDescargadas: {guardadas} | descartadas (otra cola): {ajenas}");
    println!("Base de datos: {partidas} partidas flex, {participaciones} participaciones del grupo");
    Ok(())
}

/// Guarda una partida flex y las participaciones de los jugadores del grupo.
/// Devuelve cuantos jugadores del grupo habia, o `None` si no es flex.
fn guardar_partida(conn: &mut Connection, d: &Value, por_puuid: &HashMap<String, i64>) -> Result<Option<usize>> {
    let info = &d["info"];
    let id = d["metadata"]["matchId"].as_str().context("Partida sin matchId")?;
    if info["queueId"].as_i64() != Some(COLA_FLEX) {
        return Ok(None);
    }
    let fecha = DateTime::<Utc>::from_timestamp_millis(info["gameCreation"].as_i64().context("Sin gameCreation")?)
        .context("Fecha invalida")?
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let duracion = info["gameDuration"].as_i64().unwrap_or(0);
    let participantes = info["participants"].as_array().context("Sin participants")?;
    let remake = duracion < DURACION_MINIMA || participantes.iter().any(|p| p["gameEndedInEarlySurrender"].as_bool() == Some(true));

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR IGNORE INTO flex_partidas (id_partida, fecha, duracion_seg, version_juego, remake)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, fecha, duracion, info["gameVersion"].as_str(), remake as i64],
    )?;

    let mut del_grupo = 0;
    for p in participantes {
        let Some(&id_jugador) = p["puuid"].as_str().and_then(|u| por_puuid.get(u)) else { continue };
        del_grupo += 1;
        if remake {
            continue;
        }
        let n = |c: &str| p[c].as_i64().unwrap_or(0);
        tx.execute(
            "INSERT OR IGNORE INTO flex_participaciones (
                id_partida, id_jugador, equipo, victoria, campeon_id, posicion,
                asesinatos, muertes, asistencias, dano_a_campeones, oro_ganado, tropas, puntaje_vision
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                id_jugador,
                n("teamId"),
                p["win"].as_bool().unwrap_or(false) as i64,
                n("championId"),
                p["teamPosition"].as_str().filter(|s| !s.is_empty()),
                n("kills"),
                n("deaths"),
                n("assists"),
                n("totalDamageDealtToChampions"),
                n("goldEarned"),
                n("totalMinionsKilled") + n("neutralMinionsKilled"),
                n("visionScore"),
            ],
        )?;
    }
    tx.commit()?;
    Ok(Some(del_grupo))
}
