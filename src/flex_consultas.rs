//! Rankeds flex: consultas y datos listos para las plantillas (`/flex` y `/flex/{id}`).
//! Lee solo de SQLite (tablas `flex_*` y `perfiles_riot`); los datos los trae `lol-collector flex`.

use crate::{
    catalogo::Catalogo,
    consultas::{self as c, PerfilRiot, RangoRiot, VIcono, VRango, dec, miles, punto},
    flex_graficos as g,
};
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

/// Ancho del lienzo de los graficos de la columna lateral de /flex (se muestran ~380 px).
const ANCHO_LATERAL: f64 = 440.0;
const SEMANAS: usize = 16;
const RECIENTES: usize = 8;
const HISTORIAL: usize = 20;
const DUO_MIN_PARTIDAS: i64 = 3;
const DUOS_MAX: usize = 8;

// ------------------------------------------------------------------ datos base

struct Fila {
    partida: String,
    jugador: i64,
    fecha: String,
    dt: Option<DateTime<Local>>,
    duracion: i64,
    equipo: i64,
    gana: bool,
    campeon: i64,
    rol: String,
    k: i64,
    d: i64,
    a: i64,
    dano: i64,
    cs: i64,
    vision: i64,
}

/// Participaciones del grupo (sin remakes), de la mas antigua a la mas reciente.
fn cargar(conn: &Connection) -> Result<Vec<Fila>> {
    let mut st = conn.prepare(
        "SELECT p.id_partida, p.id_jugador, x.fecha, x.duracion_seg, COALESCE(p.equipo,0), p.victoria,
                COALESCE(p.campeon_id,0), COALESCE(p.posicion,''), COALESCE(p.asesinatos,0), COALESCE(p.muertes,0),
                COALESCE(p.asistencias,0), COALESCE(p.dano_a_campeones,0), COALESCE(p.tropas,0), COALESCE(p.puntaje_vision,0)
         FROM flex_participaciones p JOIN flex_partidas x USING (id_partida)
         WHERE x.remake = 0 ORDER BY x.fecha, p.id_partida",
    )?;
    let filas = st.query_map([], |r| {
        let fecha: String = r.get(2)?;
        Ok(Fila {
            partida: r.get(0)?,
            jugador: r.get(1)?,
            dt: c::local(&fecha),
            fecha,
            duracion: r.get(3)?,
            equipo: r.get(4)?,
            gana: r.get::<_, i64>(5)? != 0,
            campeon: r.get(6)?,
            rol: r.get(7)?,
            k: r.get(8)?,
            d: r.get(9)?,
            a: r.get(10)?,
            dano: r.get(11)?,
            cs: r.get(12)?,
            vision: r.get(13)?,
        })
    })?;
    Ok(filas.collect::<rusqlite::Result<_>>()?)
}

fn nombres(conn: &Connection) -> Result<HashMap<i64, (String, String)>> {
    let mut st = conn.prepare("SELECT id_jugador, COALESCE(nombre,''), COALESCE(etiqueta,'') FROM jugadores")?;
    let filas = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, (r.get::<_, String>(1)?, r.get::<_, String>(2)?))))?;
    Ok(filas.collect::<rusqlite::Result<_>>()?)
}

/// Una fila por partida (la del primer jugador del grupo que aparece), para contar partidas del grupo.
fn unicas(filas: &[Fila]) -> Vec<&Fila> {
    let mut vistas = HashSet::new();
    filas.iter().filter(|f| vistas.insert(f.partida.as_str())).collect()
}

#[derive(Default)]
struct Res {
    partidas: i64,
    victorias: i64,
    k: i64,
    d: i64,
    a: i64,
    dano: i64,
    cs: i64,
    vision: i64,
    seg: i64,
}

impl Res {
    fn de(filas: &[&Fila]) -> Self {
        let mut r = Res::default();
        for f in filas {
            r.partidas += 1;
            r.victorias += i64::from(f.gana);
            r.k += f.k;
            r.d += f.d;
            r.a += f.a;
            r.dano += f.dano;
            r.cs += f.cs;
            r.vision += f.vision;
            r.seg += f.duracion;
        }
        r
    }
    fn winrate(&self) -> f64 {
        100.0 * self.victorias as f64 / self.partidas.max(1) as f64
    }
    fn kda(&self) -> f64 {
        c::ratio(self.k, self.d, self.a)
    }
    fn por_minuto(&self, v: i64) -> f64 {
        v as f64 / (self.seg as f64 / 60.0).max(1.0)
    }
}

/// (racha actual: (gana, largo), mejor racha de victorias, peor racha de derrotas)
fn rachas(forma: &[bool]) -> (Option<(bool, usize)>, usize, usize) {
    let (mut mejor_v, mut peor_d, mut largo, mut previo) = (0, 0, 0usize, None);
    for &g in forma {
        largo = if previo == Some(g) { largo + 1 } else { 1 };
        previo = Some(g);
        if g { mejor_v = mejor_v.max(largo) } else { peor_d = peor_d.max(largo) }
    }
    (previo.map(|g| (g, largo)), mejor_v, peor_d)
}

fn rol_texto(rol: &str) -> &'static str {
    match rol {
        "TOP" => "Superior",
        "JUNGLE" => "Jungla",
        "MIDDLE" => "Central",
        "BOTTOM" => "Tirador",
        "UTILITY" => "Soporte",
        _ => "Sin rol",
    }
}

// ------------------------------------------------------------- rangos y graficos

const TIERES: [&str; 7] = ["Hierro", "Bronce", "Plata", "Oro", "Platino", "Esmeralda", "Diamante"];
const ROMANOS: [&str; 4] = ["IV", "III", "II", "I"];
const BASE_APEX: f64 = 2800.0;

/// Posicion en una sola escala: 400 por liga, 100 por division. Maestro y superiores comparten base.
fn puntaje(tier: &str, division: &str, lp: i64) -> f64 {
    let t = match tier {
        "IRON" => 0,
        "BRONZE" => 1,
        "SILVER" => 2,
        "GOLD" => 3,
        "PLATINUM" => 4,
        "EMERALD" => 5,
        "DIAMOND" => 6,
        _ => return BASE_APEX + lp as f64,
    };
    let d = ROMANOS.iter().position(|r| *r == division).unwrap_or(0);
    (t * 400 + d * 100) as f64 + lp as f64
}

fn etiqueta_puntaje(v: f64) -> String {
    if v >= BASE_APEX {
        return format!("Maestro+ {}", (v - BASE_APEX) as i64);
    }
    let t = ((v / 400.0) as usize).min(TIERES.len() - 1);
    let d = (((v % 400.0) / 100.0) as usize).min(3);
    format!("{} {}", TIERES[t], ROMANOS[d])
}

fn dia_mes(ts: f64) -> String {
    DateTime::<Utc>::from_timestamp(ts as i64, 0)
        .map(|d| d.with_timezone(&Local))
        .map_or_else(String::new, |d| format!("{} {}", d.day(), c::MESES[d.month0() as usize]))
}

fn marcas_tiempo(t0: f64, t1: f64) -> Vec<g::Marca> {
    (0..5).map(|i| {
        let v = t0 + (t1 - t0) * f64::from(i) / 4.0;
        g::Marca { v, texto: dia_mes(v) }
    }).collect()
}

pub struct VLey {
    pub nombre: String,
    pub hue: u32,
}

pub struct VGraf {
    pub svg: String,
    pub leyenda: Vec<VLey>,
}

/// Evolucion del rango flex. Solo entran jugadores con al menos 2 registros.
fn grafico_rangos(conn: &Connection, jug: &HashMap<i64, (String, String)>, solo: Option<i64>, ancho: f64) -> Result<Option<VGraf>> {
    let mut st = conn.prepare(
        "SELECT id_jugador, fecha, tier, COALESCE(division,''), lp, COALESCE(victorias,0), COALESCE(derrotas,0)
         FROM flex_rangos ORDER BY fecha",
    )?;
    let filas = st.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?, r.get::<_, i64>(6)?))
    })?;
    let mut por_jugador: HashMap<i64, Vec<g::Punto>> = HashMap::new();
    for f in filas {
        let (id, fecha, tier, division, lp, victorias, derrotas) = f?;
        if solo.is_some_and(|s| s != id) || !jug.contains_key(&id) {
            continue;
        }
        let Some(ts) = c::local(&fecha).map(|d| d.timestamp() as f64) else { continue };
        let tip = format!("{} · {}", c::texto_rango(&RangoRiot { tier: tier.clone(), division: division.clone(), lp, victorias, derrotas }), dia_mes(ts));
        por_jugador.entry(id).or_default().push(g::Punto { x: ts, y: puntaje(&tier, &division, lp), tip });
    }
    let mut series: Vec<(i64, Vec<g::Punto>)> = por_jugador.into_iter().filter(|(_, p)| p.len() >= 2).collect();
    if series.is_empty() {
        return Ok(None);
    }
    series.sort_by_key(|(id, _)| *id);

    let todos = series.iter().flat_map(|(_, p)| p.iter());
    let (mut t0, mut t1, mut lo, mut hi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in todos {
        t0 = t0.min(p.x);
        t1 = t1.max(p.x);
        lo = lo.min(p.y);
        hi = hi.max(p.y);
    }
    if t1 - t0 < 1.0 {
        (t0, t1) = (t0 - 1.0, t1 + 1.0);
    }
    let (mut lo, mut hi) = (((lo - 50.0) / 100.0).floor() * 100.0, ((hi + 50.0) / 100.0).ceil() * 100.0);
    lo = lo.max(0.0);
    if hi - lo < 300.0 {
        hi = lo + 300.0;
    }
    let paso = if hi - lo <= 600.0 { 100.0 } else if hi - lo <= 1200.0 { 200.0 } else { 400.0 };
    let y_marcas: Vec<g::Marca> = std::iter::successors(Some(lo), |v| (*v + paso <= hi).then_some(*v + paso))
        .map(|v| g::Marca { v, texto: etiqueta_puntaje(v) })
        .collect();

    let leyenda = if solo.is_some() {
        Vec::new()
    } else {
        series.iter().map(|(id, _)| VLey { nombre: jug[id].0.clone(), hue: c::hue_texto(&jug[id].0) }).collect()
    };
    let series: Vec<g::Serie> = series.into_iter().map(|(id, puntos)| g::Serie { hue: c::hue_texto(&jug[&id].0), puntos }).collect();
    let svg = g::lineas(ancho, &series, &marcas_tiempo(t0, t1), &y_marcas, (t0, t1), (lo, hi), "Evolución del rango flex");
    Ok(Some(VGraf { svg, leyenda }))
}

/// Winrate acumulado tras cada partida.
fn grafico_acumulado(filas: &[&Fila], hue: u32) -> Option<VGraf> {
    if filas.len() < 2 {
        return None;
    }
    let mut v = 0;
    let puntos: Vec<g::Punto> = filas
        .iter()
        .enumerate()
        .map(|(i, f)| {
            v += i64::from(f.gana);
            let wr = 100.0 * v as f64 / (i + 1) as f64;
            g::Punto { x: (i + 1) as f64, y: wr, tip: format!("Partida {} · {} % de victorias", i + 1, dec(wr, 0)) }
        })
        .collect();
    let n = filas.len() as f64;
    let x_marcas: Vec<g::Marca> = (0..5)
        .map(|i| {
            let v = (1.0 + (n - 1.0) * f64::from(i) / 4.0).round();
            g::Marca { v, texto: format!("{}", v as i64) }
        })
        .collect();
    let y_marcas: Vec<g::Marca> = [0.0, 25.0, 50.0, 75.0, 100.0].iter().map(|&v| g::Marca { v, texto: format!("{v:.0} %") }).collect();
    let svg = g::lineas(720.0, &[g::Serie { hue, puntos }], &x_marcas, &y_marcas, (1.0, n), (0.0, 100.0), "Winrate acumulado por partida");
    Some(VGraf { svg, leyenda: Vec::new() })
}

fn lunes(d: NaiveDate) -> NaiveDate {
    d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

/// Partidas por semana, victorias y derrotas apiladas. `filas` debe tener una fila por partida.
fn grafico_semanas(filas: &[&Fila], ancho: f64) -> Option<String> {
    if filas.is_empty() {
        return None;
    }
    let mut por_semana: HashMap<NaiveDate, (i64, i64)> = HashMap::new();
    for f in filas {
        let Some(dt) = f.dt else { continue };
        let e = por_semana.entry(lunes(dt.date_naive())).or_default();
        if f.gana { e.0 += 1 } else { e.1 += 1 }
    }
    let actual = lunes(Local::now().date_naive());
    let periodos: Vec<(String, i64, i64)> = (0..SEMANAS as i64)
        .rev()
        .map(|i| {
            let l = actual - Duration::days(7 * i);
            let (v, d) = por_semana.get(&l).copied().unwrap_or((0, 0));
            (format!("{} {}", l.day(), c::MESES[l.month0() as usize]), v, d)
        })
        .collect();
    Some(g::barras(ancho, &periodos, "Partidas flex por semana"))
}

fn mapa_calor(filas: &[&Fila], ancho: f64) -> Option<String> {
    if filas.is_empty() {
        return None;
    }
    let mut celdas = [[0i64; 24]; 7];
    for f in filas {
        if let Some(dt) = f.dt {
            celdas[dt.weekday().num_days_from_monday() as usize][dt.hour() as usize] += 1;
        }
    }
    Some(g::calor(ancho, &celdas, "Partidas flex por día y hora"))
}

// ===================================================================== GRUPO

pub struct VJug {
    pub id: i64,
    pub nombre: String,
    pub iniciales: String,
    pub hue: u32,
    pub avatar: Option<String>,
}

pub struct VFlexFila {
    pub pos: usize,
    pub jug: VJug,
    pub etiqueta: String,
    pub rango: Option<String>,
    pub rango_val: i64,
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
    pub racha_val: i64,
    pub racha_texto: String,
    pub racha_gana: bool,
    pub mejores: Vec<VIcono>,
}

pub struct VDuo {
    pub a: VJug,
    pub b: VJug,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub wr_val: String,
    pub wr_texto: String,
}

pub struct VRecJug {
    pub jug: VJug,
    pub campeon: VIcono,
    pub k: i64,
    pub d: i64,
    pub a: i64,
}

pub struct VReciente {
    pub gana: bool,
    pub fecha: String,
    pub iso: String,
    pub duracion: String,
    pub jugadores: Vec<VRecJug>,
}

pub struct VFlex {
    pub partidas: usize,
    pub jugadores: usize,
    pub wr_texto: String,
    pub wr_css: String,
    pub victorias: usize,
    pub derrotas: usize,
    pub ultima: String,
    pub filas: Vec<VFlexFila>,
    pub rangos: Option<VGraf>,
    pub semanas: Option<String>,
    pub calor: Option<String>,
    pub duos: Vec<VDuo>,
    pub recientes: Vec<VReciente>,
}

struct Ctx<'a> {
    cat: &'a Catalogo,
    nombres: HashMap<i64, (String, String)>,
    perfiles: HashMap<i64, PerfilRiot>,
}

impl Ctx<'_> {
    fn jug(&self, id: i64) -> VJug {
        let nombre = self.nombres.get(&id).map(|n| n.0.clone()).unwrap_or_default();
        VJug {
            id,
            iniciales: c::iniciales(&nombre),
            hue: c::hue_texto(&nombre),
            avatar: self.perfiles.get(&id).and_then(|p| p.avatar(self.cat)),
            nombre,
        }
    }
    fn etiqueta(&self, id: i64) -> String {
        self.nombres.get(&id).map(|n| n.1.clone()).unwrap_or_default()
    }
}

fn mas_jugados(filas: &[&Fila], cat: &Catalogo) -> Vec<VIcono> {
    let mut por: HashMap<i64, (i64, i64)> = HashMap::new();
    for f in filas {
        let e = por.entry(f.campeon).or_default();
        e.0 += 1;
        e.1 += i64::from(f.gana);
    }
    let mut v: Vec<_> = por.into_iter().collect();
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)).then(a.0.cmp(&b.0)));
    v.into_iter().take(4).map(|(id, _)| c::ic_campeon(cat, id)).collect()
}

pub fn flex(conn: &Connection, cat: &Catalogo) -> Result<VFlex> {
    let filas = cargar(conn)?;
    let ctx = Ctx { cat, nombres: nombres(conn)?, perfiles: c::perfiles_riot(conn)? };
    let una_por_partida = unicas(&filas);

    let mut por_jugador: HashMap<i64, Vec<&Fila>> = HashMap::new();
    for f in &filas {
        por_jugador.entry(f.jugador).or_default().push(f);
    }
    // Solo jugadores con partidas flex guardadas: los que no se descargaron no se muestran.
    let ids: Vec<i64> = por_jugador.keys().copied().collect();
    let visibles: HashMap<i64, (String, String)> = ctx.nombres.iter().filter(|(id, _)| por_jugador.contains_key(id)).map(|(id, n)| (*id, n.clone())).collect();

    let mut tabla: Vec<VFlexFila> = ids
        .iter()
        .map(|&id| {
            let propias = por_jugador.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            let r = Res::de(propias);
            let forma: Vec<bool> = propias.iter().map(|f| f.gana).collect();
            let (actual, _, _) = rachas(&forma);
            let rango = ctx.perfiles.get(&id).and_then(|p| p.flex.as_ref());
            let (racha_gana, largo) = actual.unwrap_or((true, 0));
            VFlexFila {
                pos: 0,
                jug: ctx.jug(id),
                etiqueta: ctx.etiqueta(id),
                rango: rango.map(c::texto_rango),
                rango_val: rango.map_or(-1, |r| puntaje(&r.tier, &r.division, r.lp) as i64),
                partidas: r.partidas,
                victorias: r.victorias,
                derrotas: r.partidas - r.victorias,
                wr_val: punto(r.winrate(), 1),
                wr_texto: dec(r.winrate(), 0),
                kda_val: punto(r.kda(), 2),
                kda_texto: dec(r.kda(), 1),
                kda_clase: c::clase_kda(r.kda()),
                dano_val: (r.dano as f64 / r.partidas.max(1) as f64).round() as i64,
                dano_texto: miles((r.dano as f64 / r.partidas.max(1) as f64).round() as i64),
                racha_val: if racha_gana { largo as i64 } else { -(largo as i64) },
                racha_texto: if largo == 0 { String::new() } else { format!("{largo} {}", if racha_gana { "V" } else { "D" }) },
                racha_gana,
                mejores: mas_jugados(propias, cat),
            }
        })
        .collect();
    tabla.sort_by(|a, b| b.rango_val.cmp(&a.rango_val).then(b.partidas.cmp(&a.partidas)).then(a.jug.id.cmp(&b.jug.id)));
    for (i, f) in tabla.iter_mut().enumerate() {
        f.pos = i + 1;
    }

    let victorias = una_por_partida.iter().filter(|f| f.gana).count();
    let wr = 100.0 * victorias as f64 / una_por_partida.len().max(1) as f64;
    let ultima = una_por_partida.last().map_or_else(String::new, |f| c::fecha_corta(&f.fecha));

    // Duos: pares del grupo en el mismo equipo de una misma partida.
    let mut por_partida: HashMap<&str, Vec<&Fila>> = HashMap::new();
    for f in &filas {
        por_partida.entry(f.partida.as_str()).or_default().push(f);
    }
    let mut pares: HashMap<(i64, i64), (i64, i64)> = HashMap::new();
    for miembros in por_partida.values() {
        for (i, x) in miembros.iter().enumerate() {
            for y in &miembros[i + 1..] {
                if x.equipo == y.equipo {
                    let e = pares.entry((x.jugador.min(y.jugador), x.jugador.max(y.jugador))).or_default();
                    e.0 += 1;
                    e.1 += i64::from(x.gana);
                }
            }
        }
    }
    let mut duos: Vec<VDuo> = pares
        .into_iter()
        .filter(|(_, (n, _))| *n >= DUO_MIN_PARTIDAS)
        .map(|((a, b), (n, v))| {
            let wr = 100.0 * v as f64 / n as f64;
            VDuo { a: ctx.jug(a), b: ctx.jug(b), partidas: n, victorias: v, derrotas: n - v, wr_val: punto(wr, 1), wr_texto: dec(wr, 0) }
        })
        .collect();
    duos.sort_by(|x, y| y.wr_val.parse::<f64>().unwrap_or(0.0).total_cmp(&x.wr_val.parse::<f64>().unwrap_or(0.0)).then(y.partidas.cmp(&x.partidas)));
    duos.truncate(DUOS_MAX);

    // Ultimas partidas.
    let recientes: Vec<VReciente> = una_por_partida
        .iter()
        .rev()
        .take(RECIENTES)
        .map(|u| {
            let miembros = &por_partida[u.partida.as_str()];
            VReciente {
                gana: u.gana,
                fecha: c::fecha_corta(&u.fecha),
                iso: c::iso(&u.fecha),
                duracion: c::duracion(u.duracion),
                jugadores: miembros
                    .iter()
                    .map(|m| VRecJug { jug: ctx.jug(m.jugador), campeon: c::ic_campeon(cat, m.campeon), k: m.k, d: m.d, a: m.a })
                    .collect(),
            }
        })
        .collect();

    Ok(VFlex {
        partidas: una_por_partida.len(),
        jugadores: por_jugador.len(),
        wr_texto: dec(wr, 0),
        wr_css: punto(wr, 1),
        victorias,
        derrotas: una_por_partida.len() - victorias,
        ultima,
        filas: tabla,
        rangos: grafico_rangos(conn, &visibles, None, ANCHO_LATERAL)?,
        semanas: grafico_semanas(&una_por_partida, ANCHO_LATERAL),
        calor: mapa_calor(&una_por_partida, ANCHO_LATERAL),
        duos,
        recientes,
    })
}

// =================================================================== JUGADOR

pub struct VFlexCampeon {
    pub icono: VIcono,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub kda: String,
    pub wr_val: String,
    pub wr_texto: String,
}

pub struct VFlexRol {
    pub nombre: &'static str,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub kda: String,
    pub wr_val: String,
    pub wr_texto: String,
}

pub struct VFlexHist {
    pub gana: bool,
    pub campeon: VIcono,
    pub rol: &'static str,
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub kda: String,
    pub kda_clase: &'static str,
    pub dano: String,
    pub cs: i64,
    pub fecha: String,
    pub iso: String,
    pub duracion: String,
}

pub struct VFlexJugador {
    pub jug: VJug,
    pub etiqueta: String,
    pub nivel: Option<i64>,
    pub rango: Option<VRango>,
    pub partidas: i64,
    pub victorias: i64,
    pub derrotas: i64,
    pub wr_texto: String,
    pub wr_css: String,
    pub kda: String,
    pub kda_detalle: String,
    pub dano_medio: String,
    pub cs_min: String,
    pub vision_min: String,
    pub racha_texto: String,
    pub racha_gana: bool,
    pub mejor_racha: usize,
    pub peor_racha: usize,
    pub forma: Vec<bool>,
    pub rangos: Option<VGraf>,
    pub acumulado: Option<VGraf>,
    pub semanas: Option<String>,
    pub calor: Option<String>,
    pub campeones: Vec<VFlexCampeon>,
    pub roles: Vec<VFlexRol>,
    pub historial: Vec<VFlexHist>,
}

pub fn jugador(conn: &Connection, cat: &Catalogo, id: i64) -> Result<Option<VFlexJugador>> {
    let ctx = Ctx { cat, nombres: nombres(conn)?, perfiles: c::perfiles_riot(conn)? };
    if !ctx.nombres.contains_key(&id) {
        return Ok(None);
    }
    let todas = cargar(conn)?;
    let propias: Vec<&Fila> = todas.iter().filter(|f| f.jugador == id).collect();
    let r = Res::de(&propias);
    let forma: Vec<bool> = propias.iter().map(|f| f.gana).collect();
    let (actual, mejor_racha, peor_racha) = rachas(&forma);
    let (racha_gana, largo) = actual.unwrap_or((true, 0));
    let n = r.partidas.max(1) as f64;
    let jug = ctx.jug(id);
    let perfil = ctx.perfiles.get(&id);

    // Campeones y roles: (partidas, victorias, k, d, a).
    let agrupar = |clave: &dyn Fn(&Fila) -> String| -> Vec<(String, (i64, i64, i64, i64, i64))> {
        let mut por: HashMap<String, (i64, i64, i64, i64, i64)> = HashMap::new();
        for f in &propias {
            let e = por.entry(clave(f)).or_default();
            *e = (e.0 + 1, e.1 + i64::from(f.gana), e.2 + f.k, e.3 + f.d, e.4 + f.a);
        }
        let mut v: Vec<_> = por.into_iter().collect();
        v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)).then(a.0.cmp(&b.0)));
        v
    };
    let wr_de = |p: i64, v: i64| 100.0 * v as f64 / p.max(1) as f64;
    let campeones = agrupar(&|f| f.campeon.to_string())
        .into_iter()
        .take(8)
        .map(|(k, (p, v, a1, a2, a3))| VFlexCampeon {
            icono: c::ic_campeon(cat, k.parse().unwrap_or(0)),
            partidas: p,
            victorias: v,
            derrotas: p - v,
            kda: dec(c::ratio(a1, a2, a3), 2),
            wr_val: punto(wr_de(p, v), 0),
            wr_texto: dec(wr_de(p, v), 0),
        })
        .collect();
    let roles = agrupar(&|f| f.rol.clone())
        .into_iter()
        .map(|(k, (p, v, a1, a2, a3))| VFlexRol {
            nombre: rol_texto(&k),
            partidas: p,
            victorias: v,
            derrotas: p - v,
            kda: dec(c::ratio(a1, a2, a3), 2),
            wr_val: punto(wr_de(p, v), 0),
            wr_texto: dec(wr_de(p, v), 0),
        })
        .collect();

    let historial = propias
        .iter()
        .rev()
        .take(HISTORIAL)
        .map(|f| {
            let rt = c::ratio(f.k, f.d, f.a);
            VFlexHist {
                gana: f.gana,
                campeon: c::ic_campeon(cat, f.campeon),
                rol: rol_texto(&f.rol),
                k: f.k,
                d: f.d,
                a: f.a,
                kda: dec(rt, 2),
                kda_clase: c::clase_kda(rt),
                dano: miles(f.dano),
                cs: f.cs,
                fecha: c::fecha_corta(&f.fecha),
                iso: c::iso(&f.fecha),
                duracion: c::duracion(f.duracion),
            }
        })
        .collect();

    Ok(Some(VFlexJugador {
        etiqueta: ctx.etiqueta(id),
        nivel: perfil.map(|p| p.nivel),
        rango: perfil.map(|p| c::v_rango("Flex", &p.flex)),
        partidas: r.partidas,
        victorias: r.victorias,
        derrotas: r.partidas - r.victorias,
        wr_texto: dec(r.winrate(), 1),
        wr_css: punto(r.winrate(), 1),
        kda: dec(r.kda(), 2),
        kda_detalle: format!("{} / {} / {} por partida", dec(r.k as f64 / n, 1), dec(r.d as f64 / n, 1), dec(r.a as f64 / n, 1)),
        dano_medio: miles((r.dano as f64 / n).round() as i64),
        cs_min: dec(r.por_minuto(r.cs), 1),
        vision_min: dec(r.por_minuto(r.vision), 2),
        racha_texto: if largo == 0 { String::new() } else { format!("{largo} {}", if racha_gana { if largo == 1 { "victoria" } else { "victorias" } } else if largo == 1 { "derrota" } else { "derrotas" }) },
        racha_gana,
        mejor_racha,
        peor_racha,
        forma: forma.iter().rev().take(HISTORIAL).rev().copied().collect(),
        rangos: grafico_rangos(conn, &ctx.nombres, Some(id), 720.0)?,
        acumulado: grafico_acumulado(&propias, jug.hue),
        semanas: grafico_semanas(&propias, 720.0),
        calor: mapa_calor(&propias, 720.0),
        campeones,
        roles,
        historial,
        jug,
    }))
}
