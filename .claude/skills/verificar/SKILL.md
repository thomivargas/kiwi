---
name: verificar
description: Use when checking that the Sala Kiwi collector compiles, interpreting cargo errors, or closing a change with the right validation.
---

# Verificar — Sala Kiwi

Alcance, niveles y comandos: `.claude/rules/testing.md`. Proponer al usuario las pruebas pesadas antes de correrlas; `cargo check` se puede correr directo.

1. Si `cargo` no está en el PATH: `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`.
2. Si `serve` está en ejecución y hay que compilar `--release`: `Stop-Process -Name lol-collector` primero.
3. Correr `cargo check`; añadir `cargo test` / `cargo clippy` solo si el nivel lo pide.
4. No corregir errores si el usuario solo pidió comprobarlos. Reportar `archivo:línea — mensaje`; con más de diez, mostrar diez e indicar cuántos faltan.
5. En el cierre decir qué se corrió y qué no (por ejemplo `collect` sin cliente de League abierto, `perfiles` sin key).
