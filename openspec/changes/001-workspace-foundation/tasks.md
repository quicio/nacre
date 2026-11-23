# Tasks — workspace foundation

> Orden estricto. Una tarea no se marca hasta ejecutar su verificación.

## 1. Base del repositorio

- [ ] 1.1 Crear `Cargo.toml` de workspace con `resolver = "3"`, `edition = "2024"`, miembros `crates/*` y `apps/*`.
  - Verificación: `cargo metadata --no-deps --format-version 1` devuelve los cinco paquetes.
- [ ] 1.2 Crear `rust-toolchain.toml` con `channel = "1.100.0"` y componentes `rustfmt`, `clippy`.
  - Verificación: `rustc --version` dentro del repo muestra 1.100.0.
- [ ] 1.3 Crear `.gitignore` (`/target`, `*.wav` de trabajo, `.DS_Store`, `openspec/.cache` si existe).
  - Verificación: `git status` no muestra `target/`.
- [ ] 1.4 Añadir sección `[workspace.lints]` y heredar `lints.workspace = true` en cada crate.
  - Verificación: `cargo clippy --workspace --all-targets` compila con los lints activos.

## 2. Crates mínimos

- [ ] 2.1 `crates/nacre-domain`: newtypes `PatchId`, `NodeId`, `ParameterId`, `Revision` y enum `DomainError` con `thiserror`. Sin `serde`.
  - Verificación: test de igualdad/hash y de `Display` del error; `cargo tree -p nacre-domain` sin dependencias de audio/UI.
- [ ] 2.2 `crates/nacre-application`: traits `AudioHost`, `Clock`, `Renderer` con firmas mínimas documentadas; sin implementaciones.
  - Verificación: `cargo build -p nacre-application`; `rg "cpal|egui" crates/nacre-application` sin resultados.
- [ ] 2.3 `crates/nacre-dsp`: `SampleRate`, `BlockSize` con validación y función `apply_gain`.
  - Verificación: test de ganancia unitaria y de rechazo de sample rate 0.
- [ ] 2.4 `crates/nacre-audio-runtime`: `AudioConfig` (sample rate, canales, buffer) con `validate()`.
  - Verificación: test de configuración válida (48 kHz, 2 canales, 256) y de buffer 0 rechazado.
- [ ] 2.5 `apps/nacre-standalone`: `main` que imprime nombre y versión del workspace y el estado "audio: no inicializado".
  - Verificación: `cargo run -p nacre-standalone` imprime la versión y termina con código 0.

## 3. CI y controles

- [ ] 3.1 Crear `.github/workflows/ci.yml` con jobs: `fmt`, `clippy`, `test`, `build`, `openspec`, `deny`.
  - Verificación: el workflow es YAML válido (`actionlint` si está disponible) y los comandos son los de `AGENTS.md`.
- [ ] 3.2 Añadir `deny.toml` con licencias permitidas y `GPL-3.0` denegada.
  - Verificación: `cargo deny check` en verde.
- [ ] 3.3 Crear `scripts/check-deps.sh` que use `cargo metadata` para comprobar el grafo permitido (domain sin cpal/egui/serde; cpal solo en nacre-audio-runtime; sin ciclos).
  - Verificación: el script pasa en verde; se prueba a introducir una dependencia prohibida y falla.
- [ ] 3.4 Añadir `openspec validate --strict` al job `openspec`.
  - Verificación: el comando pasa en local con los changes actuales.

## 4. Documentación y cierre

- [ ] 4.1 Actualizar `README.md` con la estructura real del workspace y los comandos de desarrollo.
  - Verificación: los comandos del README se ejecutan sin error.
- [ ] 4.2 Actualizar `HANDOFF.md`: change completado, siguiente change (`002-audio-foundation`), decisiones abiertas.
  - Verificación: revisión manual.
- [ ] 4.3 Ejecutar la validación completa del design (`build`, `test`, `clippy`, `fmt`, `check-deps`, `deny`, `openspec validate --strict`).
  - Verificación: todos en verde; pegar la salida en el reporte de cierre.

## Fuera de alcance de este change

- `cpal`, dispositivo de audio, osciladores, UI, persistencia, MIDI, plugin, agente.
- Cualquier crate adicional (`nacre-ui`, `nacre-generative`, `nacre-persistence`, `nacre-agent-*`).
