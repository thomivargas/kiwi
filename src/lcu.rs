//! Acceso de solo lectura al cliente de League (LCU) en 127.0.0.1.

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::blocking::Client;
use serde_json::Value;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

pub struct Lcu {
    client: Client,
    base: String,
    auth: String,
}

impl Lcu {
    /// Busca el proceso del cliente y extrae puerto y token de su linea de comandos.
    pub fn detectar() -> Result<Self> {
        let sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_processes(ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always)),
        );
        let proc = sys
            .processes()
            .values()
            .find(|p| p.name().to_string_lossy().eq_ignore_ascii_case("LeagueClientUx.exe"))
            .context("No encuentro el cliente de League. Abrelo e inicia sesion.")?;

        let args: Vec<String> = proc.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let buscar = |clave: &str| {
            args.iter()
                .find_map(|a| a.strip_prefix(clave).map(|v| v.trim_matches('"').to_string()))
        };
        let puerto = buscar("--app-port=").context("No pude leer el puerto del cliente")?;
        let token = buscar("--remoting-auth-token=").context("No pude leer el token del cliente")?;

        let client = Client::builder()
            // El LCU usa un certificado propio; solo hablamos con 127.0.0.1.
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            client,
            base: format!("https://127.0.0.1:{puerto}"),
            auth: format!("Basic {}", STANDARD.encode(format!("riot:{token}"))),
        })
    }

    pub fn get(&self, ruta: &str) -> Result<Value> {
        let resp = self
            .client
            .get(format!("{}{ruta}", self.base))
            .header("Authorization", &self.auth)
            .send()
            .with_context(|| format!("Fallo la peticion a {ruta}"))?;
        if !resp.status().is_success() {
            bail!("{ruta} -> HTTP {}", resp.status());
        }
        Ok(resp.json()?)
    }
}
