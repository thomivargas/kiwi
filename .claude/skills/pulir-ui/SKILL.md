---
name: pulir-ui
description: Use when creating or changing a Sala Kiwi screen, template or style, or when asked to polish or visually verify the web.
---

# Pulir UI — Sala Kiwi

Diseño de referencia: `diseno-sala-kiwi/` ("Escarcha y oro"). Respetarlo; no inventar paletas ni tipografías nuevas.

## Dónde va cada cosa

- Plantilla propia en `web/templates/` (Askama); piezas repetidas en `macros.html`.
- Datos ya formateados en `src/consultas.rs`; la plantilla no calcula.
- Ruta en `src/web.rs`.
- Estilos en una sección nueva de `web/static/styles.css`, con los colores del tema.
- Sin textos de pista ni pies innecesarios: que el diseño explique.
- Rangos de Riot sin LP junto a los puntos del grupo.

## Verificar

1. Compilar (parar `serve` antes en Windows) y arrancar `serve`.
2. Abrir `http://localhost:8080` y revisar la pantalla afectada, también en ancho de móvil.
3. Probar con datos vacíos (base nueva) y con datos reales.
4. Si no se pudo ver en el navegador, decirlo explícitamente en el cierre.
