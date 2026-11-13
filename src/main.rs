//! Historial de partidas personalizadas de League.
//!
//! Uso:
//!   lol-collector collect            # lee el cliente y guarda las personalizadas en SQLite
//!   lol-collector collect --all      # guarda todos los modos
//!   lol-collector importar [carpeta] # importa JSON ya capturados (def. data/games)
//!   lol-collector assets             # descarga nombres e iconos (una sola vez)
//!   lol-collector perfiles           # trae icono, nivel y rango desde la API de Riot (necesita key)
//!   lol-collector agregar nombre#tag # da de alta a un jugador por Riot ID (necesita key)
//!   lol-collector flex [--max N] [--jugadores 1,2]     # rankeds flex y rango desde la API de Riot (necesita key y `perfiles`)
//!   lol-collector serve [--puerto N] # web local en http://localhost:8080
//!
//! La base de datos queda en `data/lol.db`.

mod catalogo;
mod consultas;
mod db;
mod flex;
mod flex_consultas;
mod flex_graficos;
mod lcu;
mod riot;
mod web;

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

const RUTA_DB: &str = "data/lol.db";
const USO: &str = "Uso: lol-collector collect [--all] | importar [carpeta] | assets | perfiles | agregar nombre#etiqueta | flex [--max N] [--jugadores 1,2] | serve [--puerto N]";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("collect") => collect(args.any(|a| a == "--all")),
        Some("importar") => importar(Path::new(&args.next().unwrap_or_else(|| "data/games".into()))),
        Some("assets") => catalogo::descargar(),
        Some("perfiles") => riot::perfiles(&db::abrir(Path::new(RUTA_DB))?),
        Some("agregar") => riot::agregar(&db::abrir(Path::new(RUTA_DB))?, &args.next().context("Falta el Riot ID (nombre#etiqueta)")?),
        Some("flex") => {
            let (mut max, mut solo) = (None, None);
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--max" => max = Some(args.next().context("--max necesita un numero")?.parse()?),
                    "--jugadores" => {
                        let ids = args.next().context("--jugadores necesita ids separados por coma")?;
                        solo = Some(ids.split(',').map(|i| i.trim().parse()).collect::<Result<Vec<i64>, _>>()?);
                    }
                    _ => bail!("Argumento desconocido: {a}
{USO}"),
                }
            }
            flex::flex(&mut db::abrir(Path::new(RUTA_DB))?, max, solo)
        }
        Some("serve") => {
            let mut puerto: u16 = 8080;
            while let Some(a) = args.next() {
                match a.as_str() {
                    "--puerto" => puerto = args.next().context("--puerto necesita un numero")?.parse()?,
                    _ => bail!("Argumento desconocido: {a}\n{USO}"),
                }
            }
            serve(puerto)
        }
        _ => bail!("{USO}"),
    }
}

fn serve(puerto: u16) -> Result<()> {
    let conn = db::abrir(Path::new(RUTA_DB))?;
    let cat = catalogo::Catalogo::cargar();
    if cat.campeones.is_empty() {
        println!("Aviso: sin nombres ni iconos. Ejecuta `lol-collector assets` para descargarlos.");
    }
    // El runtime se crea aqui (y no con #[tokio::main]) porque `collect` usa reqwest bloqueante.
    tokio::runtime::Builder::new_multi_thread().enable_all().build()?.block_on(web::servir(conn, cat, puerto))
}

fn collect(todas: bool) -> Result<()> {
    let lcu = lcu::Lcu::detectar()?;
    let mut conn = db::abrir(Path::new(RUTA_DB))?;

    // El LCU solo expone las ultimas ~20 partidas (begIndex se ignora),
    // asi que hay que ejecutar el colector con frecuencia.
    let pagina = lcu.get("/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex=19")?;
    let juegos = pagina["games"]["games"].as_array().cloned().unwrap_or_default();

    let (mut revisadas, mut nuevas, mut existentes) = (0, 0, 0);
    for j in &juegos {
        revisadas += 1;
        if !todas && j["gameType"] != "CUSTOM_GAME" {
            continue;
        }
        let Some(id) = j["gameId"].as_u64() else { continue };
        let detalle = lcu.get(&format!("/lol-match-history/v1/games/{id}"))?;
        if db::guardar_partida(&mut conn, &detalle)? {
            nuevas += 1;
            println!("+ {id}  {}  {}", detalle["gameType"], detalle["gameCreationDate"]);
        } else {
            existentes += 1;
        }
    }

    println!("\nRevisadas: {revisadas} | nuevas: {nuevas} | ya guardadas: {existentes}");
    resumen(&conn)
}

fn importar(carpeta: &Path) -> Result<()> {
    let mut conn = db::abrir(Path::new(RUTA_DB))?;
    let (mut nuevas, mut existentes) = (0, 0);
    for entrada in fs::read_dir(carpeta)? {
        let ruta = entrada?.path();
        if ruta.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let g: serde_json::Value = serde_json::from_str(&fs::read_to_string(&ruta)?)?;
        if db::guardar_partida(&mut conn, &g)? {
            nuevas += 1;
            println!("+ {}", ruta.display());
        } else {
            existentes += 1;
        }
    }
    println!("\nImportadas: {nuevas} | ya existian: {existentes}");
    resumen(&conn)
}

fn resumen(conn: &rusqlite::Connection) -> Result<()> {
    let (partidas, jugadores, participaciones) = db::totales(conn)?;
    println!("Base de datos: {partidas} partidas, {jugadores} jugadores, {participaciones} participaciones");
    Ok(())
}
