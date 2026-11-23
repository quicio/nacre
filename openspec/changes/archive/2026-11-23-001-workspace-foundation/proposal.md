# Proposal — workspace foundation

## Por qué

Sin un workspace con límites verificables, el primer código de audio acabará mezclando DSP, dispositivo, UI y dominio en un solo crate. Ese error es barato de evitar ahora y caro de corregir después: obliga a reescribir la frontera de tiempo real y el modelo de patch. Necesitamos que la estructura compile, que las reglas de dependencia se comprueben solas y que cualquier agente sepa dónde va cada cosa antes de escribir la primera línea de DSP.

## Qué cambia

- Workspace Cargo con cinco piezas reales: `nacre-domain`, `nacre-application`, `nacre-dsp`, `nacre-audio-runtime`, `apps/nacre-standalone`.
- Toolchain fijado (Rust 1.100.0, edición 2024) y lints de workspace.
- CI mínima en Linux: `fmt`, `clippy -D warnings`, `test`, `build`, validación de OpenSpec y `cargo deny`.
- Script de verificación del grafo de dependencias permitido.
- Documentación base ya presente (`README.md`, `AGENTS.md`, `HANDOFF.md`, arquitectura, ADRs) enlazada desde el workspace.
- Convenciones de test y un test de humo por crate.

**No** se implementa audio, DSP, UI ni persistencia. Cada crate contiene solo el mínimo que justifica su existencia y que se usará en el change 002.

## Capacidades

### Nuevas capacidades

Ninguna. Es un change de tooling: no cambia comportamiento observable del instrumento. Se marca `skip_specs: true` porque no hay requisito de producto que especificar; inventar uno violaría la regla de no crear specs de relleno.

### Capacidades modificadas

Ninguna.

## Impacto

- Crea `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `.cargo/config.toml` (si es necesario), `.github/workflows/ci.yml`, `deny.toml`, `scripts/check-deps.sh`.
- Crea los cinco crates con `lib.rs`/`main.rs` mínimos y un test cada uno.
- No toca `docs/` ni `openspec/` salvo enlaces desde el README.
- Riesgo principal: crear abstracciones vacías. Mitigación: cada crate tiene una responsabilidad usada por el change 002 y nada más.

## No-alcance

- Ningún dispositivo de audio, oscilador, filtro o ventana funcional.
- Ningún crate de persistencia, generativo, agente, plugin o UI separada.
- Ninguna matriz de CI multiplataforma (se añade al estabilizar el audio).
- Ninguna decisión de licencia del producto (queda como ADR pendiente).
