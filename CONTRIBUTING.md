# Contribuir

## Flujo

1. Haz un fork o crea una rama desde `main`: `feat/<tema>` o `fix/<tema>`.
2. Cambios pequeños y enfocados; una responsabilidad por módulo (ver `CLAUDE.md`).
3. Comprueba que compila: `cargo check` (y `cargo clippy` si puedes).
4. Si tocas la web, abre `serve` y revisa la pantalla afectada.
5. Abre un PR describiendo qué cambia y qué no pudiste verificar.

## Tu propia base y tu propia key

- Cada persona usa su `data/lol.db`. No existe base compartida.
- Cada persona saca su key en [developer.riotgames.com](https://developer.riotgames.com) y la pone en su `.env`. **No compartas keys por chat ni las pegues en issues o PR.**
- Si no tienes key, todo funciona salvo `perfiles`, `agregar` y `flex`.

## Qué nunca se sube

- `.env`, `data/riot.key` ni ninguna key de Riot.
- `data/` (bases, copias `.bak-*`, JSON de partidas, iconos descargados).
- Capturas, logs o ejemplos con nombres de invocador reales, PUUIDs o tokens del cliente.

`.gitignore` ya excluye lo anterior; no lo modifiques para exponerlo.

## Reglas del proyecto

- Esquema solo en `src/db.rs`, preferiblemente aditivo. Haz copia de tu base antes de migrar.
- La web nunca llama a Riot: lee de SQLite.
- Las reglas de cálculo (Elo, MVP, KDA) no cambian sin acuerdo del grupo.
- Textos visibles, tablas y documentación en español.
- Respeta el diseño de `diseno-sala-kiwi/`.
- Al terminar un hito, anótalo en `DOCUMENTACION.md` (secciones 5 y 9).

## Con agentes de IA

`CLAUDE.md` y `AGENTS.md` dan el contexto; las skills y reglas viven en `.claude/`. Tu configuración personal va en `.claude/settings.local.json` (ignorado por git).
