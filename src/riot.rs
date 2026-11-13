//! Datos de cuenta desde la API publica de Riot: icono de perfil, nivel y rango.
//!
//! `lol-collector perfiles` consulta a cada jugador por su Riot ID (nombre#etiqueta)
//! y guarda el resultado en SQLite. La web lee de ahi y no llama a Riot.
//!
//! La key se lee de la variable de entorno RIOT_API_KEY o del archivo `data/riot.key`.
//! Nunca se imprime ni se guarda en la base de datos.

use crate::catalogo::{self, Catalogo};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{SecondsFormat, Utc};
use reqwest::{StatusCode, Url, blocking::Client};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::{fs, path::Path, thread, time::Duration};

/// Enrutado regional de account-v1 y plataforma de las cuentas del grupo.
pub(crate) const REGION: &str = "americas";
pub(crate) const PLATAFORMA: &str = "la2";

/// Busca la key en este orden: variable de entorno, archivo `.env` (linea RIOT_API_KEY=...)
/// y, por compatibilidad, `data/riot.key`.
pub(crate) fn leer_key() -> Result<String> {
    let limpia = |s: &str| s.trim().trim_matches(['"', '\'']).trim().to_string();

    if let Ok(k) = std::env::var("RIOT_API_KEY") {
        let k = limpia(&k);
        if !k.is_empty() {
            return Ok(k);
        }
    }
    if let Ok(env) = fs::read_to_string(".env") {
        let k = env
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with('#'))
            .find_map(|l| l.strip_prefix("RIOT_API_KEY="))
            .map(limpia)
            .unwrap_or_default();
        if !k.is_empty() {
            return Ok(k);
        }
    }
    let k = fs::read_to_string("data/riot.key").map(|s| limpia(&s)).unwrap_or_default();
    if k.is_empty() {
        bail!("No encuentro la key de Riot. Ponla en el archivo .env (RIOT_API_KEY=...) o en la variable RIOT_API_KEY");
    }
    Ok(k)
}

pub(crate) struct Api {
    pub(crate) cli: Client,
    key: String,
}

impl Api {
    pub(crate) fn nueva() -> Result<Self> {
        Ok(Self { cli: Client::builder().user_agent("lol-collector").build()?, key: leer_key()? })
    }

    /// GET con la key en la cabecera. `None` si Riot responde 404.
    pub(crate) fn get(&self, url: Url) -> Result<Option<Value>> {
        for _ in 0..3 {
            let r = self.cli.get(url.clone()).header("X-Riot-Token", &self.key).send()?;
            match r.status() {
                StatusCode::OK => return Ok(Some(r.json()?)),
                StatusCode::NOT_FOUND => return Ok(None),
                StatusCode::TOO_MANY_REQUESTS => {
                    let seg = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok()).unwrap_or(5u64);
                    println!("  limite de peticiones; espero {seg} s...");
                    thread::sleep(Duration::from_secs(seg + 1));
                }
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => bail!(
                    "Riot rechazo la key (HTTP {}). Una key de desarrollo caduca cada 24 h: regenerala en developer.riotgames.com.",
                    r.status().as_u16()
                ),
                s => bail!("Riot respondio HTTP {} en {}", s.as_u16(), url.host_str().unwrap_or("?")),
            }
        }
        bail!("Demasiados reintentos por limite de peticiones")
    }
}

pub(crate) struct Rango {
    pub(crate) tier: String,
    pub(crate) division: String,
    pub(crate) lp: i64,
    pub(crate) victorias: i64,
    pub(crate) derrotas: i64,
}

pub(crate) fn rango(entradas: &Value, cola: &str) -> Option<Rango> {
    let e = entradas.as_array()?.iter().find(|e| e["queueType"] == cola)?;
    Some(Rango {
        tier: e["tier"].as_str()?.to_string(),
        division: e["rank"].as_str().unwrap_or("").to_string(),
        lp: e["leaguePoints"].as_i64().unwrap_or(0),
        victorias: e["wins"].as_i64().unwrap_or(0),
        derrotas: e["losses"].as_i64().unwrap_or(0),
    })
}

pub fn perfiles(conn: &Connection) -> Result<()> {
    let api = Api::nueva()?;

    // Version de Data Dragon para los iconos de perfil.
    let mut version = Catalogo::cargar().version;
    if version.is_empty() {
        let v: Vec<String> = api.cli.get("https://ddragon.leagueoflegends.com/api/versions.json").send()?.error_for_status()?.json()?;
        version = v.into_iter().next().context("Sin versiones de Data Dragon")?;
    }
    let dir_iconos = format!("{}/perfiles", catalogo::DIR);
    fs::create_dir_all(&dir_iconos)?;

    let mut st = conn.prepare("SELECT id_jugador, COALESCE(nombre,''), COALESCE(etiqueta,'') FROM jugadores ORDER BY id_jugador")?;
    let jugadores: Vec<(i64, String, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;

    let (mut ok, mut sin_cuenta) = (0, 0);
    for (id, nombre, etiqueta) in jugadores {
        if nombre.is_empty() || etiqueta.is_empty() {
            continue;
        }
        let nombre_completo = format!("{nombre}#{etiqueta}");

        let mut url = Url::parse(&format!("https://{REGION}.api.riotgames.com"))?;
        url.path_segments_mut()
            .map_err(|_| anyhow!("URL invalida"))?
            .extend(["riot", "account", "v1", "accounts", "by-riot-id", nombre.as_str(), etiqueta.as_str()]);
        let Some(cuenta) = api.get(url)? else {
            println!("- {nombre_completo}: Riot no encuentra esa cuenta (¿cambio de Riot ID?)");
            sin_cuenta += 1;
            continue;
        };
        let puuid = cuenta["puuid"].as_str().context("Riot no devolvio puuid")?.to_string();

        let host = format!("https://{PLATAFORMA}.api.riotgames.com");
        let Some(invocador) = api.get(Url::parse(&format!("{host}/lol/summoner/v4/summoners/by-puuid/{puuid}"))?)? else {
            println!("- {nombre_completo}: sin datos de invocador en {PLATAFORMA}");
            sin_cuenta += 1;
            continue;
        };
        let entradas = api.get(Url::parse(&format!("{host}/lol/league/v4/entries/by-puuid/{puuid}"))?)?.unwrap_or(Value::Null);

        let icono = invocador["profileIconId"].as_i64().unwrap_or(0);
        let nivel = invocador["summonerLevel"].as_i64().unwrap_or(0);
        let (solo, flex) = (rango(&entradas, "RANKED_SOLO_5x5"), rango(&entradas, "RANKED_FLEX_SR"));

        // Icono de perfil (solo si aun no esta descargado).
        let ruta = format!("{dir_iconos}/{icono}.png");
        if !Path::new(&ruta).exists() {
            let bytes = api
                .cli
                .get(format!("https://ddragon.leagueoflegends.com/cdn/{version}/img/profileicon/{icono}.png"))
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.bytes());
            if let Ok(b) = bytes {
                let _ = fs::write(&ruta, &b);
            }
        }

        let campo = |r: &Option<Rango>| -> (Option<String>, Option<String>, Option<i64>, Option<i64>, Option<i64>) {
            match r {
                Some(r) => (Some(r.tier.clone()), Some(r.division.clone()), Some(r.lp), Some(r.victorias), Some(r.derrotas)),
                None => (None, None, None, None, None),
            }
        };
        let (s, f) = (campo(&solo), campo(&flex));
        conn.execute(
            "INSERT INTO perfiles_riot (id_jugador, puuid_riot, icono_perfil, nivel,
                solo_tier, solo_division, solo_lp, solo_victorias, solo_derrotas,
                flex_tier, flex_division, flex_lp, flex_victorias, flex_derrotas, actualizado)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id_jugador) DO UPDATE SET
                puuid_riot = excluded.puuid_riot, icono_perfil = excluded.icono_perfil, nivel = excluded.nivel,
                solo_tier = excluded.solo_tier, solo_division = excluded.solo_division, solo_lp = excluded.solo_lp,
                solo_victorias = excluded.solo_victorias, solo_derrotas = excluded.solo_derrotas,
                flex_tier = excluded.flex_tier, flex_division = excluded.flex_division, flex_lp = excluded.flex_lp,
                flex_victorias = excluded.flex_victorias, flex_derrotas = excluded.flex_derrotas,
                actualizado = excluded.actualizado",
            params![
                id, puuid, icono, nivel, s.0, s.1, s.2, s.3, s.4, f.0, f.1, f.2, f.3, f.4,
                Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
            ],
        )?;

        crate::flex::registrar_rango(conn, id, flex.as_ref())?;

        let texto = solo.map_or("sin clasificar".to_string(), |r| format!("{} {} {} LP", r.tier, r.division, r.lp));
        println!("+ {nombre_completo}: nivel {nivel}, solo/dúo {texto}");
        ok += 1;
    }

    println!("\nPerfiles actualizados: {ok} | sin cuenta: {sin_cuenta}");
    Ok(())
}

/// Da de alta un jugador por su Riot ID (`nombre#etiqueta`) aunque no haya jugado personalizadas,
/// y trae su perfil. Su `puuid` en `jugadores` es el de Riot; si luego aparece en una personalizada,
/// el cliente lo identifica con otro puuid y habria que fusionar a mano.
pub fn agregar(conn: &Connection, riot_id: &str) -> Result<()> {
    let (nombre, etiqueta) = riot_id.rsplit_once('#').context("Usa el formato nombre#etiqueta")?;
    let api = Api::nueva()?;
    let mut url = Url::parse(&format!("https://{REGION}.api.riotgames.com"))?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("URL invalida"))?
        .extend(["riot", "account", "v1", "accounts", "by-riot-id", nombre, etiqueta]);
    let cuenta = api.get(url)?.with_context(|| format!("Riot no encuentra la cuenta {riot_id}"))?;
    let puuid = cuenta["puuid"].as_str().context("Riot no devolvio puuid")?;

    let existe: Option<i64> =
        conn.query_row("SELECT id_jugador FROM perfiles_riot WHERE puuid_riot = ?1", [puuid], |r| r.get(0)).ok();
    if let Some(id) = existe {
        println!("{riot_id} ya esta guardado (id {id}).");
        return Ok(());
    }
    conn.execute(
        "INSERT INTO jugadores (puuid, nombre, etiqueta, visto_por_ultima_vez) VALUES (?1, ?2, ?3, ?4)",
        params![puuid, cuenta["gameName"].as_str().unwrap_or(nombre), cuenta["tagLine"].as_str().unwrap_or(etiqueta), Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)],
    )?;
    let id: i64 = conn.query_row("SELECT id_jugador FROM jugadores WHERE puuid = ?1", [puuid], |r| r.get(0))?;
    println!("Jugador agregado con id {id}.");
    perfiles(conn)
}
