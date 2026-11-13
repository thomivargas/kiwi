//! Consultas a SQLite y preparacion de los datos para las plantillas.
//! Todo sale ya formateado en espanol (miles con punto, decimales con coma, hora local).

use crate::catalogo::Catalogo;
use anyhow::Result;
use chrono::{DateTime, Datelike, Local, Timelike};
use rusqlite::{Connection, params};
use std::collections::HashMap;

// ------------------------------------------------------------------ formato

pub(crate) const MESES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
const MESES_LARGOS: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
];

pub fn miles(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 {
        out.push('-');
    }
    out.chars().rev().collect()
}

pub fn dec(x: f64, d: usize) -> String {
    format!("{x:.d$}").replace('.', ",")
}

/// Decimal con punto, para atributos HTML/CSS (`--pct`, `data-value`).
pub fn punto(x: f64, d: usize) -> String {
    format!("{x:.d$}")
}

pub(crate) fn duracion(seg: i64) -> String {
    format!("{}:{:02}", seg / 60, seg % 60)
}

pub(crate) fn local(s: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Local))
}

fn hora(d: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", d.hour(), d.minute())
}

fn fecha_larga(s: &str) -> String {
    local(s).map_or_else(|| s.to_string(), |d| format!("{} {} {} · {}", d.day(), MESES[d.month0() as usize], d.year(), hora(&d)))
}

pub(crate) fn fecha_corta(s: &str) -> String {
    local(s).map_or_else(|| s.to_string(), |d| format!("{} {} · {}", d.day(), MESES[d.month0() as usize], hora(&d)))
}

fn fecha_con_coma(s: &str) -> String {
    local(s).map_or_else(|| s.to_string(), |d| format!("{} {} {}, {}", d.day(), MESES[d.month0() as usize], d.year(), hora(&d)))
}

pub(crate) fn iso(s: &str) -> String {
    local(s).map_or_else(|| s.to_string(), |d| d.format("%Y-%m-%dT%H:%M").to_string())
}

pub(crate) fn aria_fecha(s: &str) -> String {
    local(s).map_or_else(|| s.to_string(), |d| format!("{} de {}, {}", d.day(), MESES_LARGOS[d.month0() as usize], hora(&d)))
}

fn hue(n: i64) -> u32 {
    ((n.unsigned_abs() * 47) % 360) as u32
}

pub(crate) fn hue_texto(t: &str) -> u32 {
    (t.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b))) % 360) as u32
}

pub(crate) fn iniciales(t: &str) -> String {
    t.chars().filter(|c| c.is_alphanumeric()).take(2).collect::<String>().to_uppercase()
}

pub(crate) fn ratio(k: i64, d: i64, a: i64) -> f64 {
    (k + a) as f64 / d.max(1) as f64
}

pub(crate) fn clase_kda(r: f64) -> &'static str {
    if r >= 5.0 {
        "kda-ratio--great"
    } else if r >= 3.0 {
        "kda-ratio--good"
    } else {
        ""
    }
}

fn equipo(eq: i64) -> (&'static str, &'static str, &'static str) {
    if eq == 100 { ("blue", "Equipo azul", "Azul") } else { ("red", "Equipo rojo", "Rojo") }
}

fn modo_etiqueta(modo: &str) -> String {
    if modo == "KIWI" { "ARAM con aumentos".into() } else { modo.to_string() }
}

// ------------------------------------------------------------ piezas visuales

pub struct Num {
    pub v: i64,
    pub t: String,
}

impl Num {
    fn new(v: i64) -> Self {
        Self { v, t: miles(v) }
    }
}

/// Icono cuadrado (campeon o hechizo): imagen si existe, si no iniciales o numero.
pub struct VIcono {
    pub img: Option<String>,
    pub texto: String,
    pub hue: u32,
    pub nombre: String,
}

pub struct VObjeto {
    pub img: Option<String>,
    pub hue: u32,
    pub nombre: String,
    pub vacio: bool,
    pub trinket: bool,
}

pub struct VAumento {
    pub hue: u32,
    pub nombre: String,
}

pub(crate) fn ic_campeon(c: &Catalogo, id: i64) -> VIcono {
    let nombre = c.campeones.get(&id);
    VIcono {
        img: c.imagen("campeones", id),
        texto: nombre.map_or_else(|| id.to_string(), |n| iniciales(n)),
        hue: hue(id),
        nombre: nombre.cloned().unwrap_or_else(|| format!("Campeón {id}")),
    }
}

fn ic_hechizo(c: &Catalogo, id: i64) -> VIcono {
    let nombre = c.hechizos.get(&id);
    VIcono {
        img: c.imagen("hechizos", id),
        texto: nombre.map_or_else(|| id.to_string(), |n| iniciales(n)),
        hue: hue(id),
        nombre: nombre.cloned().unwrap_or_else(|| format!("Hechizo {id}")),
    }
}

fn ic_objeto(c: &Catalogo, id: i64, trinket: bool) -> VObjeto {
    VObjeto {
        img: c.imagen("objetos", id),
        hue: hue(id),
        nombre: c.objetos.get(&id).cloned().unwrap_or_else(|| format!("Objeto {id}")),
        vacio: id == 0,
        trinket,
    }
}

// ------------------------------------------------------------------ datos base

/// Una fila de `participaciones` junto con su jugador.
struct Part {
    id_jugador: i64,
    nombre: String,
    etiqueta: String,
    equipo: i64,
    victoria: bool,
    campeon: i64,
    k: i64,
    d: i64,
    a: i64,
    nivel: i64,
    oro: i64,
    curacion: i64,
    dano: i64,
    recibido: i64,
    tropas: i64,
    monstruos: i64,
    vision: i64,
    dobles: i64,
    triples: i64,
    cuadras: i64,
    pentas: i64,
    objetos: [i64; 7],
    aumentos: [i64; 6],
    hechizos: [i64; 2],
}

const SQL_PART: &str = "SELECT j.id_jugador, j.nombre, j.etiqueta, p.equipo, p.victoria, p.campeon_id,
    p.asesinatos, p.muertes, p.asistencias, p.nivel, p.oro_ganado, p.curacion, p.dano_a_campeones,
    p.dano_recibido, p.tropas_asesinadas, p.monstruos_asesinados, p.puntaje_vision,
    p.dobles, p.triples, p.cuadras, p.pentas,
    p.objeto_0, p.objeto_1, p.objeto_2, p.objeto_3, p.objeto_4, p.objeto_5, p.objeto_6,
    p.aumento_1, p.aumento_2, p.aumento_3, p.aumento_4, p.aumento_5, p.aumento_6,
    p.hechizo_1, p.hechizo_2
    FROM participaciones p JOIN jugadores j USING (id_jugador)
    WHERE p.id_partida = ?1
    ORDER BY p.equipo, p.dano_a_campeones DESC";

fn participantes(conn: &Connection, id_partida: i64) -> Result<Vec<Part>> {
    let mut st = conn.prepare(SQL_PART)?;
    let filas = st.query_map([id_partida], |r| {
        let g = |i: usize| -> rusqlite::Result<i64> { Ok(r.get::<_, Option<i64>>(i)?.unwrap_or(0)) };
        let mut objetos = [0; 7];
        for (i, o) in objetos.iter_mut().enumerate() {
            *o = g(21 + i)?;
        }
        let mut aumentos = [0; 6];
        for (i, o) in aumentos.iter_mut().enumerate() {
            *o = g(28 + i)?;
        }
        Ok(Part {
            id_jugador: g(0)?,
            nombre: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            etiqueta: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            equipo: g(3)?,
            victoria: g(4)? != 0,
            campeon: g(5)?,
            k: g(6)?,
            d: g(7)?,
            a: g(8)?,
            nivel: g(9)?,
            oro: g(10)?,
            curacion: g(11)?,
            dano: g(12)?,
            recibido: g(13)?,
            tropas: g(14)?,
            monstruos: g(15)?,
            vision: g(16)?,
            dobles: g(17)?,
            triples: g(18)?,
            cuadras: g(19)?,
            pentas: g(20)?,
            objetos,
            aumentos,
            hechizos: [g(34)?, g(35)?],
        })
    })?;
    Ok(filas.collect::<rusqlite::Result<_>>()?)
}

/// MVP de la partida: mejor puntaje combinado de TODOS los jugadores (no solo del equipo ganador).
/// puntaje = 50 % KDA + 35 % dano a campeones + 15 % oro, cada uno relativo al maximo de la partida.
fn mvp(parts: &[Part]) -> Option<i64> {
    let max = |f: &dyn Fn(&Part) -> f64| parts.iter().map(f).fold(0.0_f64, f64::max).max(1.0);
    let (mk, md, mo) = (
        max(&|p| ratio(p.k, p.d, p.a)),
        max(&|p| p.dano as f64),
        max(&|p| p.oro as f64),
    );
    parts
        .iter()
        .map(|p| (p.id_jugador, 0.5 * ratio(p.k, p.d, p.a) / mk + 0.35 * p.dano as f64 / md + 0.15 * p.oro as f64 / mo))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

fn kills(parts: &[Part], eq: i64) -> i64 {
    parts.iter().filter(|p| p.equipo == eq).map(|p| p.k).sum()
}

fn gana(parts: &[Part], eq: i64) -> bool {
    parts.iter().any(|p| p.equipo == eq && p.victoria)
}

fn resumen_resultado(parts: &[Part]) -> String {
    let (ka, kr) = (kills(parts, 100), kills(parts, 200));
    if gana(parts, 100) {
        format!("ganó el equipo azul, {ka}–{kr}")
    } else if gana(parts, 200) {
        format!("ganó el equipo rojo, {kr}–{ka}")
    } else {
        "sin resultado".into()
    }
}

struct Cabecera {
    id: i64,
    fecha: String,
    duracion: i64,
    modo: String,
}

fn cabecera(conn: &Connection, id: i64) -> Result<Option<Cabecera>> {
    let mut st = conn.prepare("SELECT id_partida, fecha, duracion_seg, COALESCE(modo, '') FROM partidas WHERE id_partida = ?1")?;
    let mut filas = st.query_map([id], |r| Ok(Cabecera { id: r.get(0)?, fecha: r.get(1)?, duracion: r.get(2)?, modo: r.get(3)? }))?;
    Ok(filas.next().transpose()?)
}

// -------------------------------------------------------------------- resumen por jugador

struct Resumen {
    id: i64,
    nombre: String,
    etiqueta: String,
    visto: String,
    partidas: i64,
    victorias: i64,
    k: i64,
    d: i64,
    a: i64,
    dano: i64,
    dobles: i64,
    triples: i64,
    cuadras: i64,
    pentas: i64,
}

impl Resumen {
    fn winrate(&self) -> f64 {
        100.0 * self.victorias as f64 / self.partidas.max(1) as f64
    }
    fn kda(&self) -> f64 {
        ratio(self.k, self.d, self.a)
    }
    fn media(&self, x: i64) -> f64 {
        x as f64 / self.partidas.max(1) as f64
    }
    fn medias_texto(&self) -> String {
        format!("{} / {} / {}", dec(self.media(self.k), 1), dec(self.media(self.d), 1), dec(self.media(self.a), 1))
    }
}

fn resumenes(conn: &Connection) -> Result<Vec<Resumen>> {
    let mut st = conn.prepare(
        "SELECT j.id_jugador, COALESCE(j.nombre,''), COALESCE(j.etiqueta,''), COALESCE(j.visto_por_ultima_vez,''),
                COUNT(*), SUM(p.victoria), SUM(p.asesinatos), SUM(p.muertes), SUM(p.asistencias),
                SUM(p.dano_a_campeones), SUM(p.dobles), SUM(p.triples), SUM(p.cuadras), SUM(p.pentas)
         FROM participaciones p JOIN jugadores j USING (id_jugador)
         GROUP BY j.id_jugador",
    )?;
    let filas = st.query_map([], |r| {
        let g = |i: usize| -> rusqlite::Result<i64> { Ok(r.get::<_, Option<i64>>(i)?.unwrap_or(0)) };
        Ok(Resumen {
            id: g(0)?,
            nombre: r.get(1)?,
            etiqueta: r.get(2)?,
            visto: r.get(3)?,
            partidas: g(4)?,
            victorias: g(5)?,
            k: g(6)?,
            d: g(7)?,
            a: g(8)?,
            dano: g(9)?,
            dobles: g(10)?,
            triples: g(11)?,
            cuadras: g(12)?,
            pentas: g(13)?,
        })
    })?;
    Ok(filas.collect::<rusqlite::Result<_>>()?)
}

// ------------------------------------------------------- perfil (API de Riot)

pub(crate) struct RangoRiot {
    pub(crate) tier: String,
    pub(crate) division: String,
    pub(crate) lp: i64,
    pub(crate) victorias: i64,
    pub(crate) derrotas: i64,
}

pub(crate) struct PerfilRiot {
    pub(crate) icono: i64,
    pub(crate) nivel: i64,
    pub(crate) solo: Option<RangoRiot>,
    pub(crate) flex: Option<RangoRiot>,
}

impl PerfilRiot {
    pub(crate) fn avatar(&self, cat: &Catalogo) -> Option<String> {
        cat.imagen("perfiles", self.icono)
    }
}

/// Perfiles guardados por `lol-collector perfiles`, por id de jugador. Vacio si nunca se ejecuto.
pub(crate) fn perfiles_riot(conn: &Connection) -> Result<HashMap<i64, PerfilRiot>> {
    let mut st = conn.prepare(
        "SELECT id_jugador, icono_perfil, nivel, solo_tier, solo_division, solo_lp, solo_victorias, solo_derrotas,
                flex_tier, flex_division, flex_lp, flex_victorias, flex_derrotas
         FROM perfiles_riot",
    )?;
    let rango = |r: &rusqlite::Row, i: usize| -> rusqlite::Result<Option<RangoRiot>> {
        let Some(tier) = r.get::<_, Option<String>>(i)? else { return Ok(None) };
        Ok(Some(RangoRiot {
            tier,
            division: r.get::<_, Option<String>>(i + 1)?.unwrap_or_default(),
            lp: r.get::<_, Option<i64>>(i + 2)?.unwrap_or(0),
            victorias: r.get::<_, Option<i64>>(i + 3)?.unwrap_or(0),
            derrotas: r.get::<_, Option<i64>>(i + 4)?.unwrap_or(0),
        }))
    };
    let filas = st.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            PerfilRiot {
                icono: r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                nivel: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                solo: rango(r, 3)?,
                flex: rango(r, 8)?,
            },
        ))
    })?;
    Ok(filas.collect::<rusqlite::Result<_>>()?)
}

fn tier_es(t: &str) -> Option<&'static str> {
    Some(match t {
        "IRON" => "Hierro",
        "BRONZE" => "Bronce",
        "SILVER" => "Plata",
        "GOLD" => "Oro",
        "PLATINUM" => "Platino",
        "EMERALD" => "Esmeralda",
        "DIAMOND" => "Diamante",
        "MASTER" => "Maestro",
        "GRANDMASTER" => "Gran Maestro",
        "CHALLENGER" => "Retador",
        _ => return None,
    })
}

pub(crate) fn texto_rango(r: &RangoRiot) -> String {
    let tier = tier_es(&r.tier).map_or_else(|| r.tier.clone(), str::to_string);
    let sin_division = r.division.is_empty() || matches!(r.tier.as_str(), "MASTER" | "GRANDMASTER" | "CHALLENGER");
    if sin_division { format!("{tier} · {} LP", r.lp) } else { format!("{tier} {} · {} LP", r.division, r.lp) }
}

/// Solo la division ("Esmeralda III"), sin LP: en el ranking del grupo el LP de Riot se
/// confundiria con los puntos propios.
pub(crate) fn texto_division(r: &RangoRiot) -> String {
    let tier = tier_es(&r.tier).map_or_else(|| r.tier.clone(), str::to_string);
    let sin_division = r.division.is_empty() || matches!(r.tier.as_str(), "MASTER" | "GRANDMASTER" | "CHALLENGER");
    if sin_division { tier } else { format!("{tier} {}", r.division) }
}

/// Rango de una cola ya formateado para mostrar.
pub struct VRango {
    pub cola: &'static str,
    pub texto: String,
    pub detalle: String,
}

pub(crate) fn v_rango(cola: &'static str, r: &Option<RangoRiot>) -> VRango {
    match r {
        Some(r) => VRango {
            cola,
            texto: texto_rango(r),
            detalle: format!("{} % · {} V · {} D", dec(100.0 * r.victorias as f64 / (r.victorias + r.derrotas).max(1) as f64, 0), r.victorias, r.derrotas),
        },
        None => VRango { cola, texto: "Sin clasificar".into(), detalle: String::new() },
    }
}

// ======================================================================= INICIO

/// Una "noche" es una sesion de juego. El dia cambia a las 06:00 hora local, asi una
/// sesion que pasa de medianoche no se parte en dos.
const HORA_CORTE_NOCHE: i64 = 6;
/// El historial se pagina por noches (no por partidas) para no partir una noche en dos paginas.
const NOCHES_POR_PAGINA: usize = 5;
const DIAS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];

pub struct VMvpFila {
    pub id: i64,
    pub nombre: String,
}

/// Una partida como fila compacta dentro de una noche.
pub struct VFilaPartida {
    pub id: i64,
    pub hora: String,
    pub iso: String,
    pub duracion: String,
    pub gana_azul: bool,
    pub gana_rojo: bool,
    pub kills_azul: i64,
    pub kills_rojo: i64,
    pub azul: Vec<VIcono>,
    pub rojo: Vec<VIcono>,
    pub mvp: Option<VMvpFila>,
    pub aria: String,
}

pub struct VNoche {
    pub titulo: String,
    pub partidas: usize,
    pub azul: i64,
    pub rojo: i64,
    pub filas: Vec<VFilaPartida>,
}

pub struct VTop {
    pub pos: usize,
    pub id: i64,
    pub nombre: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
    pub puntos: i64,
    pub partidas: i64,
}

pub struct VMvpNoche {
    pub id: i64,
    pub nombre: String,
    pub etiqueta: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
    pub veces: usize,
    pub noche: String,
}

/// Un elemento del paginador: un numero de pagina o un hueco ("...").
pub struct VPag {
    pub n: usize,
    pub actual: bool,
    pub hueco: bool,
}

pub struct VInicio {
    pub total: i64,
    pub paginas: usize,
    pub anterior: Option<usize>,
    pub siguiente: Option<usize>,
    pub numeros: Vec<VPag>,
    pub noches: Vec<VNoche>,
    pub top: Vec<VTop>,
    pub mvp_noche: Option<VMvpNoche>,
}

fn clave_noche(fecha: &str) -> Option<(chrono::NaiveDate, String)> {
    let d = local(fecha)? - chrono::Duration::hours(HORA_CORTE_NOCHE);
    let titulo = format!("{} {} {}", DIAS[d.weekday().num_days_from_monday() as usize], d.day(), MESES[d.month0() as usize]);
    Some((d.date_naive(), titulo))
}

fn fila_partida(conn: &Connection, cat: &Catalogo, id: i64) -> Result<Option<VFilaPartida>> {
    let (Some(c), parts) = (cabecera(conn, id)?, participantes(conn, id)?) else { return Ok(None) };
    let id_mvp = mvp(&parts);
    let iconos = |eq: i64| parts.iter().filter(|p| p.equipo == eq).map(|p| ic_campeon(cat, p.campeon)).collect::<Vec<_>>();
    let hora_txt = local(&c.fecha).map(|d| hora(&d)).unwrap_or_default();
    Ok(Some(VFilaPartida {
        id: c.id,
        iso: iso(&c.fecha),
        duracion: duracion(c.duracion),
        gana_azul: gana(&parts, 100),
        gana_rojo: gana(&parts, 200),
        kills_azul: kills(&parts, 100),
        kills_rojo: kills(&parts, 200),
        azul: iconos(100),
        rojo: iconos(200),
        mvp: id_mvp.and_then(|i| parts.iter().find(|p| p.id_jugador == i)).map(|p| VMvpFila { id: p.id_jugador, nombre: p.nombre.clone() }),
        aria: format!("Partida de las {hora_txt}, {}", resumen_resultado(&parts)),
        hora: hora_txt,
    }))
}

/// Numeros de pagina a mostrar: primera, ultima y las vecinas de la actual, con huecos.
fn numeros_paginador(pagina: usize, paginas: usize) -> Vec<VPag> {
    let mut mostrar: Vec<usize> = vec![1, paginas, pagina.saturating_sub(1).max(1), pagina, (pagina + 1).min(paginas)];
    mostrar.sort_unstable();
    mostrar.dedup();
    let mut out = Vec::new();
    let mut previo = 0;
    for n in mostrar {
        if previo != 0 && n > previo + 1 {
            out.push(VPag { n: 0, actual: false, hueco: true });
        }
        out.push(VPag { n, actual: n == pagina, hueco: false });
        previo = n;
    }
    out
}

pub fn inicio(conn: &Connection, cat: &Catalogo, pagina: usize) -> Result<VInicio> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM partidas", [], |r| r.get(0))?;

    // Todas las partidas (solo id y fecha, es barato), agrupadas en noches de la mas reciente a la mas antigua.
    let mut st = conn.prepare("SELECT id_partida, fecha FROM partidas ORDER BY fecha DESC")?;
    let todas: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    let mut grupos: Vec<(chrono::NaiveDate, String, Vec<i64>)> = Vec::new();
    for (id, fecha) in &todas {
        let Some((dia, titulo)) = clave_noche(fecha) else { continue };
        if grupos.last().is_some_and(|g| g.0 == dia) {
            if let Some(g) = grupos.last_mut() {
                g.2.push(*id);
            }
        } else {
            grupos.push((dia, titulo, vec![*id]));
        }
    }

    let paginas = grupos.len().div_ceil(NOCHES_POR_PAGINA).max(1);
    let pagina = pagina.clamp(1, paginas);

    let mut noches = Vec::new();
    for (_, titulo, ids) in grupos.iter().skip((pagina - 1) * NOCHES_POR_PAGINA).take(NOCHES_POR_PAGINA) {
        let mut filas = Vec::new();
        for id in ids {
            if let Some(f) = fila_partida(conn, cat, *id)? {
                filas.push(f);
            }
        }
        noches.push(VNoche {
            titulo: titulo.clone(),
            partidas: filas.len(),
            azul: filas.iter().filter(|f| f.gana_azul).count() as i64,
            rojo: filas.iter().filter(|f| f.gana_rojo).count() as i64,
            filas,
        });
    }

    let res = resumenes(conn)?;
    let perfiles = perfiles_riot(conn)?;
    let avatar = |id: i64| perfiles.get(&id).and_then(|p| p.avatar(cat));

    // Mini ranking: por puntos del grupo (Elo).
    let puntos = puntos_elo(conn)?;
    let pts = |id: i64| puntos.get(&id).copied().unwrap_or(ELO_INICIAL);
    let mut orden: Vec<&Resumen> = res.iter().collect();
    orden.sort_by(|a, b| pts(b.id).total_cmp(&pts(a.id)).then(b.winrate().total_cmp(&a.winrate())));
    let top = orden
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, r)| VTop {
            pos: i + 1,
            id: r.id,
            nombre: r.nombre.clone(),
            iniciales: iniciales(&r.nombre),
            hue: hue_texto(&r.nombre),
            avatar: avatar(r.id),
            puntos: pts(r.id).round() as i64,
            partidas: r.partidas,
        })
        .collect();

    // MVP de la noche mas reciente (aunque se este viendo otra pagina): quien mas veces fue MVP.
    let mvp_noche = match grupos.first() {
        Some((_, titulo, ids)) => {
            let mut cuenta: Vec<(i64, usize)> = Vec::new();
            for id in ids {
                if let Some(m) = fila_partida(conn, cat, *id)?.and_then(|f| f.mvp) {
                    match cuenta.iter_mut().find(|(j, _)| *j == m.id) {
                        Some(c) => c.1 += 1,
                        None => cuenta.push((m.id, 1)),
                    }
                }
            }
            cuenta.into_iter().max_by_key(|&(_, v)| v).and_then(|(id, veces)| {
                let r = res.iter().find(|r| r.id == id)?;
                Some(VMvpNoche {
                    id,
                    nombre: r.nombre.clone(),
                    etiqueta: r.etiqueta.clone(),
                    iniciales: iniciales(&r.nombre),
                    hue: hue_texto(&r.nombre),
                    avatar: avatar(id),
                    veces,
                    noche: titulo.clone(),
                })
            })
        }
        None => None,
    };

    Ok(VInicio {
        total,
        paginas,
        anterior: (pagina > 1).then(|| pagina - 1),
        siguiente: (pagina < paginas).then(|| pagina + 1),
        numeros: numeros_paginador(pagina, paginas),
        noches,
        top,
        mvp_noche,
    })
}

// ======================================================================= PARTIDA

pub struct VFila {
    pub id_jugador: i64,
    pub nombre: String,
    pub etiqueta: String,
    pub es_mvp: bool,
    pub penta: bool,
    pub cuadra: bool,
    pub campeon: VIcono,
    pub nivel: i64,
    pub hechizos: Vec<VIcono>,
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub ratio: String,
    pub kda_texto: String,
    pub kda_clase: &'static str,
    pub dano: String,
    pub dano_pct: String,
    pub oro: String,
    pub oro_pct: String,
    pub objetos: Vec<VObjeto>,
    pub aumentos: Vec<VAumento>,
}

pub struct VEquipo {
    pub clase: &'static str,
    pub nombre: &'static str,
    pub gana: bool,
    pub kills: i64,
    pub muertes: i64,
    pub asistencias: i64,
    pub dano: String,
    pub oro: String,
    pub filas: Vec<VFila>,
}

pub struct VDetalle {
    pub id_jugador: i64,
    pub nombre: String,
    pub campeon: VIcono,
    pub clase: &'static str,
    pub corto: &'static str,
    pub curacion: Num,
    pub recibido: Num,
    pub tropas: Num,
    pub monstruos: Num,
    pub vision: Num,
    pub dobles: Num,
    pub triples: Num,
    pub cuadras: Num,
    pub pentas: Num,
}

pub struct VPartida {
    pub id: i64,
    pub titulo: String,
    pub fecha: String,
    pub iso: String,
    pub duracion: String,
    pub modo: String,
    pub modo_etiqueta: String,
    pub formato: String,
    pub kills_azul: i64,
    pub kills_rojo: i64,
    pub gana_azul: bool,
    pub gana_rojo: bool,
    pub aria_marcador: String,
    pub mvp: Option<VFila>,
    pub mvp_linea: String,
    pub equipos: Vec<VEquipo>,
    pub detalle: Vec<VDetalle>,
}

fn fila(p: &Part, cat: &Catalogo, max_dano: f64, max_oro: f64, id_mvp: Option<i64>) -> VFila {
    let r = ratio(p.k, p.d, p.a);
    VFila {
        id_jugador: p.id_jugador,
        nombre: p.nombre.clone(),
        etiqueta: p.etiqueta.clone(),
        es_mvp: id_mvp == Some(p.id_jugador),
        penta: p.pentas > 0,
        cuadra: p.cuadras > 0 && p.pentas == 0,
        campeon: ic_campeon(cat, p.campeon),
        nivel: p.nivel,
        hechizos: p.hechizos.iter().map(|&h| ic_hechizo(cat, h)).collect(),
        k: p.k,
        d: p.d,
        a: p.a,
        ratio: dec(r, 2),
        kda_texto: format!("{} KDA", dec(r, 2)),
        kda_clase: clase_kda(r),
        dano: miles(p.dano),
        dano_pct: punto(100.0 * p.dano as f64 / max_dano, 0),
        oro: miles(p.oro),
        oro_pct: punto(100.0 * p.oro as f64 / max_oro, 0),
        objetos: p.objetos.iter().enumerate().map(|(i, &o)| ic_objeto(cat, o, i == 6)).collect(),
        aumentos: p.aumentos.iter().filter(|&&a| a > 0).map(|&a| VAumento { hue: hue(a), nombre: format!("Aumento {a}") }).collect(),
    }
}

pub fn partida(conn: &Connection, cat: &Catalogo, id: i64) -> Result<Option<VPartida>> {
    let Some(c) = cabecera(conn, id)? else { return Ok(None) };
    let parts = participantes(conn, id)?;
    let id_mvp = mvp(&parts);
    let max_dano = parts.iter().map(|p| p.dano as f64).fold(1.0, f64::max);
    let max_oro = parts.iter().map(|p| p.oro as f64).fold(1.0, f64::max);

    let (ka, kr) = (kills(&parts, 100), kills(&parts, 200));
    let (ga, gr) = (gana(&parts, 100), gana(&parts, 200));
    let n = |eq: i64| parts.iter().filter(|p| p.equipo == eq).count();

    let mut equipos = Vec::new();
    for eq in [100, 200] {
        let (clase, nombre, _) = equipo(eq);
        let del_equipo: Vec<&Part> = parts.iter().filter(|p| p.equipo == eq).collect();
        if del_equipo.is_empty() {
            continue;
        }
        equipos.push(VEquipo {
            clase,
            nombre,
            gana: gana(&parts, eq),
            kills: del_equipo.iter().map(|p| p.k).sum(),
            muertes: del_equipo.iter().map(|p| p.d).sum(),
            asistencias: del_equipo.iter().map(|p| p.a).sum(),
            dano: miles(del_equipo.iter().map(|p| p.dano).sum()),
            oro: miles(del_equipo.iter().map(|p| p.oro).sum()),
            filas: del_equipo.iter().map(|p| fila(p, cat, max_dano, max_oro, id_mvp)).collect(),
        });
    }

    let mvp_part = id_mvp.and_then(|i| parts.iter().find(|p| p.id_jugador == i));
    let mvp_linea = mvp_part.map_or_else(String::new, |p| {
        format!("{} · nivel {} · {}", ic_campeon(cat, p.campeon).nombre, p.nivel, equipo(p.equipo).1.to_lowercase())
    });

    let mut orden: Vec<&Part> = parts.iter().collect();
    orden.sort_by(|a, b| b.recibido.cmp(&a.recibido));
    let detalle = orden
        .iter()
        .map(|p| VDetalle {
            id_jugador: p.id_jugador,
            nombre: p.nombre.clone(),
            campeon: ic_campeon(cat, p.campeon),
            clase: equipo(p.equipo).0,
            corto: equipo(p.equipo).2,
            curacion: Num::new(p.curacion),
            recibido: Num::new(p.recibido),
            tropas: Num::new(p.tropas),
            monstruos: Num::new(p.monstruos),
            vision: Num::new(p.vision),
            dobles: Num::new(p.dobles),
            triples: Num::new(p.triples),
            cuadras: Num::new(p.cuadras),
            pentas: Num::new(p.pentas),
        })
        .collect();

    Ok(Some(VPartida {
        id: c.id,
        titulo: if ga {
            "Victoria del equipo azul".into()
        } else if gr {
            "Victoria del equipo rojo".into()
        } else {
            "Partida sin resultado".into()
        },
        fecha: fecha_larga(&c.fecha),
        iso: iso(&c.fecha),
        duracion: duracion(c.duracion),
        modo_etiqueta: modo_etiqueta(&c.modo),
        modo: c.modo.clone(),
        formato: format!("{} vs {}", n(100), n(200)),
        kills_azul: ka,
        kills_rojo: kr,
        gana_azul: ga,
        gana_rojo: gr,
        aria_marcador: format!("Marcador: equipo azul {ka}, equipo rojo {kr}"),
        mvp: mvp_part.map(|p| fila(p, cat, max_dano, max_oro, id_mvp)),
        mvp_linea,
        equipos,
        detalle,
    }))
}

// ======================================================================= JUGADOR

pub struct VCampeonUsado {
    pub icono: VIcono,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub kda: String,
    pub pct: String,
    pub pct_entero: String,
}

pub struct VHistorial {
    pub id_partida: i64,
    pub gana: bool,
    pub campeon: VIcono,
    pub mvp: bool,
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub kda: String,
    pub kda_clase: &'static str,
    pub dano: String,
    pub objetos: Vec<VObjeto>,
    pub fecha: String,
    pub iso: String,
    pub aria_fecha: String,
    pub duracion: String,
}

pub struct VJugador {
    pub puntos: i64,
    pub puesto_puntos: usize,
    pub nombre: String,
    pub etiqueta: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
    pub nivel: Option<i64>,
    pub solo: Option<VRango>,
    pub flex: Option<VRango>,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub visto: String,
    pub visto_iso: String,
    pub wr_texto: String,
    pub wr_css: String,
    pub kda: String,
    pub kda_detalle: String,
    pub dano_medio: String,
    pub puesto_dano: String,
    pub dobles: i64,
    pub triples: i64,
    pub cuadras: i64,
    pub pentas: i64,
    pub campeones: Vec<VCampeonUsado>,
    pub historial: Vec<VHistorial>,
}

pub fn jugador(conn: &Connection, cat: &Catalogo, id: i64, limite: usize) -> Result<Option<VJugador>> {
    let mut todos = resumenes(conn)?;
    todos.sort_by(|a, b| b.media(b.dano).total_cmp(&a.media(a.dano)));
    let Some(pos) = todos.iter().position(|x| x.id == id) else { return Ok(None) };
    let puesto = pos + 1;
    let r = &todos[pos];

    let mut st = conn.prepare(
        "SELECT campeon_id, COUNT(*), SUM(victoria), SUM(asesinatos), SUM(muertes), SUM(asistencias)
         FROM participaciones WHERE id_jugador = ?1 GROUP BY campeon_id
         ORDER BY COUNT(*) DESC, SUM(victoria) DESC LIMIT 5",
    )?;
    let campeones = st
        .query_map([id], |row| {
            let g = |i: usize| -> rusqlite::Result<i64> { Ok(row.get::<_, Option<i64>>(i)?.unwrap_or(0)) };
            Ok((g(0)?, g(1)?, g(2)?, g(3)?, g(4)?, g(5)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(|(camp, n, v, k, d, a)| {
            let wr = 100.0 * v as f64 / n.max(1) as f64;
            VCampeonUsado {
                icono: ic_campeon(cat, camp),
                partidas: n,
                victorias: v,
                derrotas: n - v,
                kda: dec(ratio(k, d, a), 2),
                pct: punto(wr, 0),
                pct_entero: punto(wr, 0),
            }
        })
        .collect();

    let mut st = conn.prepare(
        "SELECT p.id_partida FROM participaciones p JOIN partidas x USING (id_partida)
         WHERE p.id_jugador = ?1 ORDER BY x.fecha DESC LIMIT ?2",
    )?;
    let ids: Vec<i64> = st.query_map(params![id, limite as i64], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut historial = Vec::new();
    for id_partida in ids {
        let (Some(c), parts) = (cabecera(conn, id_partida)?, participantes(conn, id_partida)?) else { continue };
        let Some(p) = parts.iter().find(|p| p.id_jugador == id) else { continue };
        let rt = ratio(p.k, p.d, p.a);
        historial.push(VHistorial {
            id_partida,
            gana: p.victoria,
            campeon: ic_campeon(cat, p.campeon),
            mvp: mvp(&parts) == Some(id),
            k: p.k,
            d: p.d,
            a: p.a,
            kda: dec(rt, 2),
            kda_clase: clase_kda(rt),
            dano: miles(p.dano),
            objetos: p.objetos[..6].iter().map(|&o| ic_objeto(cat, o, false)).collect(),
            fecha: fecha_corta(&c.fecha).replace(" · ", " · "),
            iso: iso(&c.fecha),
            aria_fecha: aria_fecha(&c.fecha),
            duracion: duracion(c.duracion),
        });
    }

    let perfil = perfiles_riot(conn)?.remove(&id);

    // Puntos del grupo y puesto en el ranking.
    let puntos = puntos_elo(conn)?;
    let mis_puntos = puntos.get(&id).copied().unwrap_or(ELO_INICIAL);
    let puesto_puntos = 1 + todos.iter().filter(|x| x.id != id && puntos.get(&x.id).copied().unwrap_or(ELO_INICIAL) > mis_puntos).count();

    Ok(Some(VJugador {
        puntos: mis_puntos.round() as i64,
        puesto_puntos,
        nombre: r.nombre.clone(),
        etiqueta: r.etiqueta.clone(),
        iniciales: iniciales(&r.nombre),
        hue: hue_texto(&r.nombre),
        avatar: perfil.as_ref().and_then(|p| p.avatar(cat)),
        nivel: perfil.as_ref().map(|p| p.nivel),
        solo: perfil.as_ref().map(|p| v_rango("Solo/dúo", &p.solo)),
        flex: perfil.as_ref().map(|p| v_rango("Flex", &p.flex)),
        partidas: r.partidas,
        victorias: r.victorias,
        derrotas: r.partidas - r.victorias,
        visto: fecha_con_coma(&r.visto),
        visto_iso: iso(&r.visto),
        wr_texto: dec(r.winrate(), 1),
        wr_css: punto(r.winrate(), 1),
        kda: dec(r.kda(), 2),
        kda_detalle: format!("{} por partida", r.medias_texto()),
        dano_medio: miles(r.media(r.dano).round() as i64),
        puesto_dano: format!("{puesto}.º del grupo"),
        dobles: r.dobles,
        triples: r.triples,
        cuadras: r.cuadras,
        pentas: r.pentas,
        campeones,
        historial,
    }))
}

// ============================================================ PUNTOS DEL GRUPO (Elo)

/// Todos empiezan con 1000 puntos. Tras cada partida, cada jugador suma o resta
/// K * (resultado - esperado), donde "esperado" depende de la media de puntos de cada equipo:
/// vencer a un equipo mas fuerte da mas puntos, y perder contra uno mas debil quita mas.
const ELO_INICIAL: f64 = 1000.0;
const ELO_K: f64 = 32.0;

/// Puntos de cada jugador (por id), recalculados desde todo el historial en orden cronologico.
fn puntos_elo(conn: &Connection) -> Result<HashMap<i64, f64>> {
    let mut st = conn.prepare(
        "SELECT p.id_partida, p.id_jugador, p.equipo, p.victoria
         FROM participaciones p JOIN partidas x USING (id_partida)
         ORDER BY x.fecha, p.id_partida",
    )?;
    let filas: Vec<(i64, i64, i64, bool)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)? != 0)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut puntos: HashMap<i64, f64> = HashMap::new();
    for (_, jugadores) in filas.chunk_by(|a, b| a.0 == b.0).map(|g| (g[0].0, g)) {
        let azul: Vec<i64> = jugadores.iter().filter(|f| f.2 == 100).map(|f| f.1).collect();
        let rojo: Vec<i64> = jugadores.iter().filter(|f| f.2 != 100).map(|f| f.1).collect();
        let gana_azul = jugadores.iter().any(|f| f.2 == 100 && f.3);
        let gana_rojo = jugadores.iter().any(|f| f.2 != 100 && f.3);
        if azul.is_empty() || rojo.is_empty() || gana_azul == gana_rojo {
            continue; // sin dos equipos o sin un ganador claro: no cuenta
        }
        let media = |ids: &[i64], p: &HashMap<i64, f64>| ids.iter().map(|i| p.get(i).copied().unwrap_or(ELO_INICIAL)).sum::<f64>() / ids.len() as f64;
        let (ra, rr) = (media(&azul, &puntos), media(&rojo, &puntos));
        let esperado_azul = 1.0 / (1.0 + 10f64.powf((rr - ra) / 400.0));
        let cambio_azul = ELO_K * (if gana_azul { 1.0 } else { 0.0 } - esperado_azul);
        for id in &azul {
            *puntos.entry(*id).or_insert(ELO_INICIAL) += cambio_azul;
        }
        for id in &rojo {
            *puntos.entry(*id).or_insert(ELO_INICIAL) -= cambio_azul;
        }
    }
    Ok(puntos)
}

// ================================================================== ESTADISTICAS

pub struct VFilaRanking {
    pub pos: usize,
    pub id: i64,
    pub nombre: String,
    pub etiqueta: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
    pub rango_riot: Option<String>,
    pub puntos: i64,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub wr_val: String,
    pub wr_texto: String,
    pub kda_val: String,
    pub kda_texto: String,
    pub kda_clase: &'static str,
    pub dano_val: i64,
    pub dano_texto: String,
    pub mejores: Vec<VIcono>,
}

pub struct VEstadisticas {
    pub partidas: i64,
    pub jugadores: usize,
    pub podio: Vec<VFilaRanking>,
    pub filas: Vec<VFilaRanking>,
}

/// Los 4 campeones mas jugados de cada jugador (por id de jugador).
fn mejores_campeones(conn: &Connection) -> Result<HashMap<i64, Vec<i64>>> {
    let mut st = conn.prepare(
        "SELECT id_jugador, campeon_id FROM participaciones
         GROUP BY id_jugador, campeon_id
         ORDER BY id_jugador, COUNT(*) DESC, SUM(victoria) DESC",
    )?;
    let filas = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))))?;
    let mut por_jugador: HashMap<i64, Vec<i64>> = HashMap::new();
    for f in filas {
        let (j, c) = f?;
        let v = por_jugador.entry(j).or_default();
        if v.len() < 4 {
            v.push(c);
        }
    }
    Ok(por_jugador)
}

pub fn estadisticas(conn: &Connection, cat: &Catalogo) -> Result<VEstadisticas> {
    let partidas: i64 = conn.query_row("SELECT COUNT(*) FROM partidas", [], |r| r.get(0))?;
    let perfiles = perfiles_riot(conn)?;
    let puntos = puntos_elo(conn)?;
    let mejores = mejores_campeones(conn)?;
    let pts = |id: i64| puntos.get(&id).copied().unwrap_or(ELO_INICIAL);

    let mut res = resumenes(conn)?;
    res.sort_by(|a, b| pts(b.id).total_cmp(&pts(a.id)).then(b.winrate().total_cmp(&a.winrate())).then(b.partidas.cmp(&a.partidas)));

    let fila = |i: usize, r: &Resumen| VFilaRanking {
        pos: i + 1,
        id: r.id,
        nombre: r.nombre.clone(),
        etiqueta: r.etiqueta.clone(),
        iniciales: iniciales(&r.nombre),
        hue: hue_texto(&r.nombre),
        avatar: perfiles.get(&r.id).and_then(|p| p.avatar(cat)),
        rango_riot: perfiles.get(&r.id).and_then(|p| p.solo.as_ref()).map(texto_division),
        puntos: pts(r.id).round() as i64,
        partidas: r.partidas,
        victorias: r.victorias,
        derrotas: r.partidas - r.victorias,
        wr_val: punto(r.winrate(), 1),
        wr_texto: dec(r.winrate(), 0),
        kda_val: punto(r.kda(), 2),
        kda_texto: dec(r.kda(), 1),
        kda_clase: clase_kda(r.kda()),
        dano_val: r.media(r.dano).round() as i64,
        dano_texto: miles(r.media(r.dano).round() as i64),
        mejores: mejores.get(&r.id).map(|v| v.iter().map(|&c| ic_campeon(cat, c)).collect()).unwrap_or_default(),
    };

    let filas: Vec<VFilaRanking> = res.iter().enumerate().map(|(i, r)| fila(i, r)).collect();
    let podio: Vec<VFilaRanking> = res.iter().enumerate().take(3).map(|(i, r)| fila(i, r)).collect();

    Ok(VEstadisticas { partidas, jugadores: res.len(), podio, filas })
}

// ================================================================== EQUIPOS (antes de jugar)

const EQUIPOS_MAX: usize = 12;
const EQUIPOS_OPCIONES: usize = 3;

#[derive(Clone)]
pub struct VEqJug {
    pub id: i64,
    pub nombre: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
    pub puntos: i64,
    pub marcado: bool,
}

pub struct VEqEquipo {
    pub jugadores: Vec<VEqJug>,
    pub media: i64,
    /// Probabilidad de ganar, en %.
    pub prob: i64,
    /// Puntos que gana (o pierde) cada integrante si el equipo gana (o pierde), ya con signo.
    pub si_gana: String,
    pub si_pierde: String,
}

pub struct VOpcion {
    pub azul: VEqEquipo,
    pub rojo: VEqEquipo,
    pub diferencia: i64,
}

pub struct VEquipos {
    pub jugadores: Vec<VEqJug>,
    pub marcados: usize,
    pub opciones: Vec<VOpcion>,
    pub aviso: Option<String>,
}

fn con_signo(v: f64, positivo: bool) -> String {
    let n = v.round() as i64;
    if positivo { format!("+{n}") } else { format!("−{n}") }
}

/// Todas las formas de repartir a los elegidos en dos equipos (el de menos jugadores es `tam`).
/// Con numero par se fija al primero en el azul para no repetir el espejo.
fn repartos(n: usize) -> Vec<Vec<usize>> {
    let tam = n / 2;
    let mut res = Vec::new();
    for mascara in 0u32..(1u32 << n) {
        if mascara.count_ones() as usize != tam {
            continue;
        }
        if n % 2 == 0 && mascara & 1 == 0 {
            continue;
        }
        res.push((0..n).filter(|i| mascara >> i & 1 == 1).collect());
    }
    res
}

pub fn equipos(conn: &Connection, cat: &Catalogo, elegidos: &[i64]) -> Result<VEquipos> {
    let perfiles = perfiles_riot(conn)?;
    let puntos = puntos_elo(conn)?;
    let mut res = resumenes(conn)?;
    res.sort_by(|a, b| pts_de(&puntos, b.id).total_cmp(&pts_de(&puntos, a.id)));
    let jug = |r: &Resumen| VEqJug {
        id: r.id,
        nombre: r.nombre.clone(),
        iniciales: iniciales(&r.nombre),
        hue: hue_texto(&r.nombre),
        avatar: perfiles.get(&r.id).and_then(|p| p.avatar(cat)),
        puntos: pts_de(&puntos, r.id).round() as i64,
        marcado: elegidos.contains(&r.id),
    };
    let todos: Vec<VEqJug> = res.iter().map(jug).collect();
    let marcados: Vec<&VEqJug> = todos.iter().filter(|j| j.marcado).collect();

    let mut v = VEquipos { marcados: marcados.len(), jugadores: Vec::new(), opciones: Vec::new(), aviso: None };
    if marcados.len() > EQUIPOS_MAX {
        v.aviso = Some(format!("Máximo {EQUIPOS_MAX} jugadores."));
    } else if marcados.len() >= 2 {
        let reales: Vec<f64> = marcados.iter().map(|j| pts_de(&puntos, j.id)).collect();
        let media = |ix: &[usize]| ix.iter().map(|&i| reales[i]).sum::<f64>() / ix.len() as f64;
        let mut cand: Vec<(f64, Vec<usize>)> = repartos(marcados.len())
            .into_iter()
            .map(|a| {
                let b: Vec<usize> = (0..reales.len()).filter(|i| !a.contains(i)).collect();
                ((media(&a) - media(&b)).abs(), a)
            })
            .collect();
        cand.sort_by(|x, y| x.0.total_cmp(&y.0).then_with(|| x.1.cmp(&y.1)));
        for (dif, a) in cand.into_iter().take(EQUIPOS_OPCIONES) {
            let b: Vec<usize> = (0..reales.len()).filter(|i| !a.contains(i)).collect();
            let (ma, mb) = (media(&a), media(&b));
            let esp_a = 1.0 / (1.0 + 10f64.powf((mb - ma) / 400.0));
            let equipo = |ix: &[usize], m: f64, esp: f64| VEqEquipo {
                jugadores: ix.iter().map(|&i| marcados[i].clone()).collect(),
                media: m.round() as i64,
                prob: (esp * 100.0).round() as i64,
                si_gana: con_signo(ELO_K * (1.0 - esp), true),
                si_pierde: con_signo(ELO_K * esp, false),
            };
            v.opciones.push(VOpcion { azul: equipo(&a, ma, esp_a), rojo: equipo(&b, mb, 1.0 - esp_a), diferencia: dif.round() as i64 });
        }
    }
    v.jugadores = todos;
    Ok(v)
}

fn pts_de(p: &HashMap<i64, f64>, id: i64) -> f64 {
    p.get(&id).copied().unwrap_or(ELO_INICIAL)
}
