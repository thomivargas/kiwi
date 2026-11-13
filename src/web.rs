//! Servidor web local (Sala Kiwi). Solo escucha en 127.0.0.1.

use crate::{
    catalogo::{self, Catalogo},
    consultas::{self, VEquipos, VEstadisticas, VInicio, VJugador, VPartida},
    flex_consultas::{self, VFlex, VFlexJugador},
};
use anyhow::Result;
use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, RawQuery, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use rusqlite::Connection;
use serde::Deserialize;
use rust_embed::RustEmbed;
use std::sync::{Arc, Mutex};

/// CSS, JS y tipografias, incluidos dentro del programa.
#[derive(RustEmbed)]
#[folder = "web/static/"]
struct Estatico;

struct Estado {
    conn: Mutex<Connection>,
    cat: Catalogo,
}

/// Cambia en cada arranque; se agrega a los enlaces de CSS y JS para que el navegador
/// no use una copia vieja guardada en cache.
fn version() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()).to_string()
    })
}

type Compartido = Arc<Estado>;
type Resp = Result<Html<String>, (StatusCode, String)>;

fn error_interno(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {e}"))
}

fn no_encontrado(que: &str) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("{que} no encontrada"))
}

fn render<T: Template>(t: T) -> Resp {
    t.render().map(Html).map_err(error_interno)
}

#[derive(Template)]
#[template(path = "inicio.html")]
struct InicioT {
    pagina: &'static str,
    v: &'static str,
    d: VInicio,
}

#[derive(Template)]
#[template(path = "partida.html")]
struct PartidaT {
    pagina: &'static str,
    v: &'static str,
    p: VPartida,
}

#[derive(Template)]
#[template(path = "jugador.html")]
struct JugadorT {
    pagina: &'static str,
    v: &'static str,
    j: VJugador,
}

#[derive(Template)]
#[template(path = "estadisticas.html")]
struct EstadisticasT {
    pagina: &'static str,
    v: &'static str,
    e: VEstadisticas,
}

#[derive(Template)]
#[template(path = "equipos.html")]
struct EquiposT {
    pagina: &'static str,
    v: &'static str,
    e: VEquipos,
}

#[derive(Template)]
#[template(path = "flex.html")]
struct FlexT {
    pagina: &'static str,
    v: &'static str,
    f: VFlex,
}

#[derive(Template)]
#[template(path = "flex_jugador.html")]
struct FlexJugadorT {
    pagina: &'static str,
    v: &'static str,
    j: VFlexJugador,
}

#[derive(Deserialize)]
struct ParamsInicio {
    /// Texto, no numero: un valor invalido (`?pagina=abc`) cae en la pagina 1 en vez de dar error.
    pagina: Option<String>,
}

async fn inicio(State(s): State<Compartido>, Query(q): Query<ParamsInicio>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let d = consultas::inicio(&conn, &s.cat, q.pagina.and_then(|p| p.parse().ok()).unwrap_or(1)).map_err(error_interno)?;
    render(InicioT { pagina: "inicio", v: version(), d })
}

async fn partida(State(s): State<Compartido>, Path(id): Path<i64>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let p = consultas::partida(&conn, &s.cat, id).map_err(error_interno)?.ok_or_else(|| no_encontrado("Partida"))?;
    render(PartidaT { pagina: "partida", v: version(), p })
}

async fn jugador(State(s): State<Compartido>, Path(id): Path<i64>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let j = consultas::jugador(&conn, &s.cat, id, 10).map_err(error_interno)?.ok_or_else(|| no_encontrado("Jugador"))?;
    render(JugadorT { pagina: "jugador", v: version(), j })
}

async fn estadisticas(State(s): State<Compartido>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let e = consultas::estadisticas(&conn, &s.cat).map_err(error_interno)?;
    render(EstadisticasT { pagina: "estadisticas", v: version(), e })
}

/// Jugadores elegidos: `?j=1&j=2...` (la clave se repite, por eso se lee la consulta cruda).
async fn equipos(State(s): State<Compartido>, RawQuery(q): RawQuery) -> Resp {
    let elegidos: Vec<i64> = q
        .unwrap_or_default()
        .split('&')
        .filter_map(|kv| kv.strip_prefix("j=")?.parse().ok())
        .collect();
    let conn = s.conn.lock().map_err(error_interno)?;
    let e = consultas::equipos(&conn, &s.cat, &elegidos).map_err(error_interno)?;
    render(EquiposT { pagina: "equipos", v: version(), e })
}

async fn flex(State(s): State<Compartido>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let f = flex_consultas::flex(&conn, &s.cat).map_err(error_interno)?;
    render(FlexT { pagina: "flex", v: version(), f })
}

async fn flex_jugador(State(s): State<Compartido>, Path(id): Path<i64>) -> Resp {
    let conn = s.conn.lock().map_err(error_interno)?;
    let j = flex_consultas::jugador(&conn, &s.cat, id).map_err(error_interno)?.ok_or_else(|| no_encontrado("Jugador"))?;
    render(FlexJugadorT { pagina: "flex", v: version(), j })
}

async fn estatico(Path(ruta): Path<String>) -> Response {
    match Estatico::get(&ruta) {
        Some(f) => (
            [
                (header::CONTENT_TYPE, mime_guess::from_path(&ruta).first_or_octet_stream().to_string()),
                (header::CACHE_CONTROL, "public, max-age=3600".to_string()),
            ],
            f.data.into_owned(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Imagenes descargadas (data/assets). Solo acepta `tipo/archivo.png` con caracteres seguros.
async fn assets(Path(ruta): Path<String>) -> Response {
    let partes: Vec<&str> = ruta.split('/').collect();
    let seguro = partes.len() == 2
        && partes
            .iter()
            .all(|p| !p.is_empty() && !p.contains("..") && p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')));
    if !seguro {
        return StatusCode::NOT_FOUND.into_response();
    }
    match std::fs::read(format!("{}/{ruta}", catalogo::DIR)) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png".to_string()),
                (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn servir(conn: Connection, cat: Catalogo, puerto: u16) -> Result<()> {
    let estado = Arc::new(Estado { conn: Mutex::new(conn), cat });
    let app = Router::new()
        .route("/", get(inicio))
        .route("/partida/{id}", get(partida))
        .route("/jugador/{id}", get(jugador))
        .route("/estadisticas", get(estadisticas))
        .route("/equipos", get(equipos))
        .route("/flex", get(flex))
        .route("/flex/{id}", get(flex_jugador))
        .route("/static/{*ruta}", get(estatico))
        .route("/assets/{*ruta}", get(assets))
        .with_state(estado);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", puerto)).await?;
    println!("Sala Kiwi en http://localhost:{puerto}  (Ctrl+C para cerrar)");
    axum::serve(listener, app).await?;
    Ok(())
}
