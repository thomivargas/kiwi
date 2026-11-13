//! Graficos SVG de la seccion Flex. Se generan en el servidor y usan las variables de color
//! del tema (clases `gf-*` en styles.css), asi funcionan en oscuro y claro sin JavaScript.

use std::fmt::Write;

const MT: f64 = 14.0;
const MR: f64 = 16.0;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub struct Punto {
    pub x: f64,
    pub y: f64,
    /// Texto al pasar el cursor.
    pub tip: String,
}

pub struct Serie {
    pub hue: u32,
    pub puntos: Vec<Punto>,
}

pub struct Marca {
    pub v: f64,
    pub texto: String,
}

/// Lineas con ejes. `x` e `y` son los rangos visibles; las marcas son las etiquetas de los ejes.
pub fn lineas(w: f64, series: &[Serie], x_marcas: &[Marca], y_marcas: &[Marca], x: (f64, f64), y: (f64, f64), aria: &str) -> String {
    const H: f64 = 260.0;
    const ML: f64 = 92.0;
    const MB: f64 = 30.0;
    let px = |v: f64| ML + (v - x.0) / (x.1 - x.0).max(1e-9) * (w - ML - MR);
    let py = |v: f64| MT + (1.0 - (v - y.0) / (y.1 - y.0).max(1e-9)) * (H - MT - MB);

    let mut s = format!(r#"<svg class="gf" viewBox="0 0 {w} {H}" role="img" aria-label="{}">"#, esc(aria));
    for m in y_marcas {
        let yy = py(m.v);
        let _ = write!(
            s,
            r#"<line class="gf-grid" x1="{ML}" x2="{:.1}" y1="{yy:.1}" y2="{yy:.1}"/><text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - MR,
            ML - 8.0,
            yy + 4.0,
            esc(&m.texto)
        );
    }
    for m in x_marcas {
        let _ = write!(s, r#"<text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#, px(m.v), H - 8.0, esc(&m.texto));
    }
    for serie in series {
        let color = format!("hsl({} 72% 58%)", serie.hue);
        if serie.puntos.len() > 1 {
            let pts: Vec<String> = serie.puntos.iter().map(|p| format!("{:.1},{:.1}", px(p.x), py(p.y))).collect();
            let _ = write!(s, r#"<polyline class="gf-linea" style="stroke:{color}" points="{}"/>"#, pts.join(" "));
        }
        if serie.puntos.len() <= 60 {
            for p in &serie.puntos {
                let _ = write!(
                    s,
                    r#"<circle class="gf-punto" style="fill:{color}" cx="{:.1}" cy="{:.1}" r="3.5"><title>{}</title></circle>"#,
                    px(p.x),
                    py(p.y),
                    esc(&p.tip)
                );
            }
        }
    }
    s.push_str("</svg>");
    s
}

/// Barras apiladas por periodo: (etiqueta, victorias, derrotas).
pub fn barras(w: f64, periodos: &[(String, i64, i64)], aria: &str) -> String {
    const H: f64 = 230.0;
    const ML: f64 = 34.0;
    const MB: f64 = 30.0;
    let maximo = periodos.iter().map(|p| p.1 + p.2).max().unwrap_or(0).max(4);
    let paso = (maximo as f64 / 4.0).ceil();
    let tope = paso * 4.0;
    let py = |v: f64| MT + (1.0 - v / tope) * (H - MT - MB);
    let ancho = (w - ML - MR) / periodos.len().max(1) as f64;

    let mut s = format!(r#"<svg class="gf" viewBox="0 0 {w} {H}" role="img" aria-label="{}">"#, esc(aria));
    for i in 0..=4 {
        let v = paso * f64::from(i);
        let yy = py(v);
        let _ = write!(
            s,
            r#"<line class="gf-grid" x1="{ML}" x2="{:.1}" y1="{yy:.1}" y2="{yy:.1}"/><text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="end">{}</text>"#,
            w - MR,
            ML - 8.0,
            yy + 4.0,
            v as i64
        );
    }
    for (i, (etiqueta, v, d)) in periodos.iter().enumerate() {
        let x0 = ML + ancho * i as f64 + ancho * 0.19;
        let w = ancho * 0.62;
        let (alto_v, alto_d) = (*v as f64 / tope * (H - MT - MB), *d as f64 / tope * (H - MT - MB));
        let base = H - MB;
        let tip = format!("{etiqueta}: {v} V · {d} D");
        if *v > 0 {
            let _ = write!(
                s,
                r#"<rect class="gf-v" x="{x0:.1}" y="{:.1}" width="{w:.1}" height="{alto_v:.1}" rx="2"><title>{}</title></rect>"#,
                base - alto_v,
                esc(&tip)
            );
        }
        if *d > 0 {
            let _ = write!(
                s,
                r#"<rect class="gf-d" x="{x0:.1}" y="{:.1}" width="{w:.1}" height="{alto_d:.1}" rx="2"><title>{}</title></rect>"#,
                base - alto_v - alto_d,
                esc(&tip)
            );
        }
        if i % if w < 500.0 { 4 } else { 2 } == 0 {
            let _ = write!(s, r#"<text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#, x0 + w / 2.0, H - 8.0, esc(etiqueta));
        }
    }
    s.push_str("</svg>");
    s
}

/// Mapa de calor semanal: filas lunes..domingo, columnas 0..23 h (hora local).
pub fn calor(w: f64, celdas: &[[i64; 24]; 7], aria: &str) -> String {
    const DIAS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];
    const ML: f64 = 40.0;
    const CELDA_H: f64 = 24.0;
    const MB: f64 = 24.0;
    let h = MT + CELDA_H * 7.0 + MB;
    let ancho = (w - ML - MR) / 24.0;
    let maximo = celdas.iter().flatten().copied().max().unwrap_or(0).max(1);

    let mut s = format!(r#"<svg class="gf" viewBox="0 0 {w} {h}" role="img" aria-label="{}">"#, esc(aria));
    for (d, fila) in celdas.iter().enumerate() {
        let y = MT + CELDA_H * d as f64;
        let _ = write!(s, r#"<text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="end">{}</text>"#, ML - 8.0, y + CELDA_H / 2.0 + 4.0, DIAS[d]);
        for (hora, &n) in fila.iter().enumerate() {
            let x = ML + ancho * hora as f64;
            let (clase, estilo) = if n > 0 {
                ("gf-calor", format!(r#" style="fill-opacity:{:.2}""#, 0.18 + 0.82 * n as f64 / maximo as f64))
            } else {
                ("gf-calor gf-calor--vacia", String::new())
            };
            let plural = if n == 1 { "partida" } else { "partidas" };
            let _ = write!(
                s,
                r#"<rect class="{clase}"{estilo} x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="3"><title>{} {:02}:00 · {n} {plural}</title></rect>"#,
                x + 1.0,
                y + 1.0,
                ancho - 2.0,
                CELDA_H - 2.0,
                DIAS[d],
                hora
            );
        }
    }
    for hora in (0..24).step_by(3) {
        let _ = write!(
            s,
            r#"<text class="gf-eje" x="{:.1}" y="{:.1}" text-anchor="middle">{hora:02}</text>"#,
            ML + ancho * (hora as f64 + 0.5),
            h - 6.0
        );
    }
    s.push_str("</svg>");
    s
}
