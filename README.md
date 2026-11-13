# Sala Kiwi

Historial y estadísticas de las partidas personalizadas de League of Legends de un grupo de amigos. Un colector en Rust lee el cliente de League, guarda todo en SQLite y sirve una web local con partidas, jugadores, estadísticas y equipos.

## Requisitos

- Windows con el cliente de League instalado.
- [Rust](https://rustup.rs) (edition 2024).
- Opcional, solo para rangos y perfiles: una key de [developer.riotgames.com](https://developer.riotgames.com).

## Empezar

```powershell
git clone <url-del-repo>
cd <carpeta>
Copy-Item .env.example .env        # y pon tu RIOT_API_KEY (opcional)

cargo run --release -- assets      # nombres e iconos (una vez)
cargo run --release -- collect     # con el cliente de League abierto y con sesión
cargo run --release -- serve       # http://localhost:8080
```

La base de datos se crea sola en `data/lol.db`; es tuya y no se sube al repo. Solo tendrá las partidas que tu cliente todavía expone (las últimas 20), así que conviene ejecutar `collect` con frecuencia.

## Comandos

| Comando | Qué hace | Necesita |
|---|---|---|
| `collect [--all]` | Guarda las partidas personalizadas (o todas) | Cliente de League abierto |
| `importar [carpeta]` | Importa JSON ya capturados | — |
| `assets` | Descarga nombres e iconos | — |
| `perfiles` | Icono, nivel y rango desde Riot | `RIOT_API_KEY` |
| `agregar nombre#tag` | Da de alta a un jugador | `RIOT_API_KEY` |
| `flex [--max N] [--jugadores 1,2]` | Rankeds flex y rango | `RIOT_API_KEY` y `perfiles` |
| `serve [--puerto N]` | Web en `http://localhost:8080` (solo 127.0.0.1) | — |

## Documentación

- `DOCUMENTACION.md`: decisiones, hallazgos sobre la API, reglas de cálculo y pendientes.
- `diagrama-proyecto.html`: flujo, tablas y pantallas.
- `CONTRIBUTING.md`: cómo contribuir.
- `CLAUDE.md` / `AGENTS.md`: contexto para agentes de IA.

## Aviso legal

Sala Kiwi no está respaldado por Riot Games ni refleja las opiniones de Riot Games ni de nadie involucrado oficialmente en la producción o gestión de las propiedades de Riot Games. Riot Games y todas las propiedades asociadas son marcas comerciales o marcas registradas de Riot Games, Inc.

Licencia: MIT (ver `LICENSE`).
