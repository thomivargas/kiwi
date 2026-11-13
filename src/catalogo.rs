//! Nombres e iconos de campeones, objetos y hechizos (Data Dragon de Riot).
//!
//! `lol-collector assets` los descarga una sola vez a `data/assets/`.
//! Despues la web funciona sin internet. Si falta algo, la web usa un icono de reemplazo.

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

pub const DIR: &str = "data/assets";
const HILOS: usize = 8;

#[derive(Default, Serialize, Deserialize)]
pub struct Catalogo {
    pub version: String,
    pub campeones: HashMap<i64, String>,
    pub objetos: HashMap<i64, String>,
    pub hechizos: HashMap<i64, String>,
}

impl Catalogo {
    /// Carga el catalogo si existe; si no, devuelve uno vacio (la web usa iconos de reemplazo).
    pub fn cargar() -> Self {
        let c: Catalogo = fs::read_to_string(format!("{DIR}/catalogo.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        c
    }

    /// URL local de la imagen, solo si el archivo existe. Se mira el disco en cada consulta,
    /// asi los iconos descargados con la web abierta (`perfiles`, `assets`) aparecen sin reiniciar.
    pub fn imagen(&self, tipo: &str, id: i64) -> Option<String> {
        let clave = format!("{tipo}/{id}");
        Path::new(&format!("{DIR}/{clave}.png")).is_file().then(|| format!("/assets/{clave}.png"))
    }
}

pub fn descargar() -> Result<()> {
    let cli = Client::builder().user_agent("lol-collector").build()?;
    let get_json = |url: String| -> Result<Value> { Ok(cli.get(url).send()?.error_for_status()?.json()?) };

    let versiones: Vec<String> = serde_json::from_value(get_json("https://ddragon.leagueoflegends.com/api/versions.json".into())?)?;
    let version = versiones.first().context("Riot no devolvio versiones")?.clone();
    println!("Version del catalogo: {version}");
    let base = format!("https://ddragon.leagueoflegends.com/cdn/{version}");

    for tipo in ["campeones", "objetos", "hechizos"] {
        fs::create_dir_all(format!("{DIR}/{tipo}"))?;
    }

    let mut cat = Catalogo { version: version.clone(), ..Default::default() };
    let mut trabajos: Vec<(String, String)> = Vec::new();
    let mut agregar = |url: String, ruta: String| {
        if !Path::new(&ruta).exists() {
            trabajos.push((url, ruta));
        }
    };

    let campeones = get_json(format!("{base}/data/es_ES/champion.json"))?;
    for c in campeones["data"].as_object().context("champion.json sin data")?.values() {
        let (Some(key), Some(id), Some(nombre)) =
            (c["key"].as_str().and_then(|k| k.parse::<i64>().ok()), c["id"].as_str(), c["name"].as_str())
        else {
            continue;
        };
        cat.campeones.insert(key, nombre.to_string());
        agregar(format!("{base}/img/champion/{id}.png"), format!("{DIR}/campeones/{key}.png"));
    }

    let objetos = get_json(format!("{base}/data/es_ES/item.json"))?;
    for (k, o) in objetos["data"].as_object().context("item.json sin data")? {
        let (Ok(id), Some(nombre)) = (k.parse::<i64>(), o["name"].as_str()) else { continue };
        cat.objetos.insert(id, nombre.to_string());
        agregar(format!("{base}/img/item/{id}.png"), format!("{DIR}/objetos/{id}.png"));
    }

    let hechizos = get_json(format!("{base}/data/es_ES/summoner.json"))?;
    for h in hechizos["data"].as_object().context("summoner.json sin data")?.values() {
        let (Some(key), Some(nombre), Some(archivo)) = (
            h["key"].as_str().and_then(|k| k.parse::<i64>().ok()),
            h["name"].as_str(),
            h["image"]["full"].as_str(),
        ) else {
            continue;
        };
        cat.hechizos.insert(key, nombre.to_string());
        agregar(format!("{base}/img/spell/{archivo}"), format!("{DIR}/hechizos/{key}.png"));
    }

    println!(
        "Catalogo: {} campeones, {} objetos, {} hechizos. Imagenes por descargar: {}",
        cat.campeones.len(),
        cat.objetos.len(),
        cat.hechizos.len(),
        trabajos.len()
    );

    let siguiente = AtomicUsize::new(0);
    let fallos = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..HILOS {
            s.spawn(|| {
                loop {
                    let i = siguiente.fetch_add(1, Ordering::Relaxed);
                    let Some((url, ruta)) = trabajos.get(i) else { break };
                    let bytes = cli.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.bytes());
                    match bytes {
                        Ok(b) if fs::write(ruta, &b).is_ok() => {}
                        _ => {
                            fallos.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });

    fs::write(format!("{DIR}/catalogo.json"), serde_json::to_string(&cat)?)?;
    println!("Listo. Descargadas: {} | fallidas: {}", trabajos.len() - fallos.load(Ordering::Relaxed), fallos.load(Ordering::Relaxed));
    Ok(())
}
