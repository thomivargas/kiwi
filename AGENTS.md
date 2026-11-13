# AGENTS.md

**La documentación canónica vive en `CLAUDE.md`; no hay una copia paralela.** Leerlo siempre primero.

Leer contexto por alcance, no todo el repositorio:

| Archivo | Qué contiene |
|---|---|
| `CLAUDE.md` | **Siempre.** Mapa, comandos y reglas. |
| `DOCUMENTACION.md` | Decisiones, hallazgos, reglas de cálculo, pendientes y registro de cambios. Solo secciones relevantes. |
| `diagrama-proyecto.html` | Flujo, tablas y pantallas. |
| `diseno-sala-kiwi/` | Diseño visual; consultar solo al crear o cambiar pantallas. |
| `CONTRIBUTING.md` | Flujo para contribuir: ramas, PR, qué no subir. |

Buscar primero en código con `rg` en `src/` y `web/`.

Reglas cortas por dominio: `.claude/rules/` (`testing.md`, `base-de-datos.md`, `commits.md`; este último prohíbe atribuir commits y PR a una IA).
Skills: `.claude/skills/` (cada `SKILL.md` declara cuándo usarla): `verificar`, `base-de-datos`, `pulir-ui`, `nuevo-dato`.
Agentes de revisión: `.claude/agents/` (`revisor-calculos`, `revisor-publico`); úsalos solo ante un cambio real de su ámbito.

Siempre correr los comandos desde la raíz del repo.

Evita etiquetas y descripciones secundarias innecesarias. Deja que el diseño haga la explicación.

## Validación

Fuente única: `.claude/rules/testing.md`. No duplicar comandos o políticas aquí.
