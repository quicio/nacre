# Nacre

Instrumento musical generativo escrito en Rust. No es un DAW, ni un secuenciador, ni un clon de modular: es un sistema para construir **sistemas que producen música**.

> No programes canciones. Crea sistemas capaces de producir música.

Nacre explora composición generativa, diseño sonoro modular, ritmo procedural y colaboración entre humanos y agentes. Su identidad sonora apunta a drones hipnóticos, ritmos orgánicos e imperfectos, timbres metálicos y resonantes, movimiento armónico complejo y paisajes espaciales — referencia estética: electrónica experimental y ciencia ficción especulativa, con la atmósfera de *Dune*.

## Estado

**Fase M0: fundación del workspace implementada.** No hay dispositivo de audio, DSP musical ni UI todavía. El repositorio contiene la visión de producto, la arquitectura, las decisiones (ADRs), el roadmap y las especificaciones del primer vertical slice en OpenSpec.

| Documento | Contenido |
|---|---|
| [docs/product/vision.md](docs/product/vision.md) | Visión, identidad sonora, no-objetivos |
| [docs/product/principles.md](docs/product/principles.md) | Principios de producto y arquitectura |
| [docs/product/roadmap.md](docs/product/roadmap.md) | Roadmap por milestones, slices y gates |
| [PRODUCT.md](PRODUCT.md) | Verdad de producto para el trabajo de diseño |
| [DESIGN.md](DESIGN.md) | Sistema visual: color, tipografía, componentes |
| [docs/architecture/overview.md](docs/architecture/overview.md) | Clean Architecture y límites |
| [docs/architecture/domain-model.md](docs/architecture/domain-model.md) | Patch, Composition, Performance |
| [docs/architecture/audio-runtime.md](docs/architecture/audio-runtime.md) | Runtime de audio y grafo |
| [docs/architecture/generative-engine.md](docs/architecture/generative-engine.md) | Motor generativo y evolución |
| [docs/architecture/agent-architecture.md](docs/architecture/agent-architecture.md) | Plano de control del agente |
| [docs/architecture/dependency-graph.md](docs/architecture/dependency-graph.md) | Grafo de dependencias entre crates |
| [docs/research/risks-and-spikes.md](docs/research/risks-and-spikes.md) | Riesgos ordenados y spikes |
| [docs/research/dependency-evaluation.md](docs/research/dependency-evaluation.md) | Evaluación real de dependencias |
| [docs/adr/](docs/adr/) | Architecture Decision Records |

## Workspace

El workspace tiene cinco paquetes y fija Rust 1.100.0, edición 2024 y lints compartidos:

```text
crates/
├── nacre-domain          IDs, revisiones y errores base del dominio
├── nacre-application     Puertos AudioHost, Clock y Renderer
├── nacre-dsp             SampleRate, BlockSize y utilidades DSP puras
└── nacre-audio-runtime   AudioConfig validada, sin acceso a dispositivos
apps/
└── nacre-standalone      Binario mínimo; informa que el audio no está inicializado
```

## Desarrollo

Desde la raíz del repositorio:

```bash
cargo build --workspace
cargo run -p nacre-standalone
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
./scripts/check-deps.sh
cargo deny check
openspec validate --all --strict
```

`scripts/check-deps.sh` consulta `cargo metadata` para rechazar dependencias prohibidas, desplazamientos de `cpal` fuera del runtime y ciclos entre paquetes. La CI de Linux se define en `.github/workflows/ci.yml`.

## Metodología

**Spec-Driven Development + Vertical Slices**, con [OpenSpec](https://openspec.dev) 2.1.

Cada incremento coherente y testeable es un *change* en `openspec/changes/`, con `proposal.md`, delta specs, `design.md` y `tasks.md`. Nada se implementa sin una spec revisada. Al terminar, el change se archiva y sus requisitos pasan a `openspec/specs/`, que describe el sistema **tal como está construido**.

```bash
openspec list                 # changes en curso
openspec list --specs         # capacidades ya construidas
openspec show <change>        # detalle de un change
openspec validate --strict    # validar artefactos
```

Changes planificados para el primer slice:

| Change | Alcance | Capacidades |
|---|---|---|
| `001-workspace-foundation` | Workspace, límites, CI, tooling | — (tooling) |
| `002-audio-foundation` | Dispositivo, callback RT, render offline, app mínima | `audio-runtime`, `oscillators`, `standalone-app` |
| `003-voice-synthesis` | Saw, ADSR, filtro, LFO, saturación | `voice-synthesis` |
| `004-parameter-system` | Parámetros tipados, smoothing, transferencia RT | `parameter-system` |

Orden de implementación: 001 → 002 → 003 → 004. Ver [roadmap](docs/product/roadmap.md).

## Sitio web

El sitio público vive en [`site/`](site/): Astro estático, bilingüe (ES por defecto, EN conmutable), desplegado en GitHub Pages con `.github/workflows/pages.yml`.

Contenido real, sin inventar: estado M0, roadmap con gates, principios, arquitectura, especificaciones y dos maquetas de interfaz **etiquetadas como concepto** (no hay capturas de la app porque no existe).

```bash
cd site
npm install
npm run dev        # http://localhost:4321/nacre/
npm run build      # genera site/dist
npm run check      # tipos y diagnóstico de Astro
```

Variables de despliegue: `SITE_URL` (por defecto `https://quicio.github.io`) y `BASE_PATH` (por defecto `/nacre`). Para un repo con otro nombre, cambia `BASE_PATH`.

La identidad visual está documentada en [DESIGN.md](DESIGN.md); el brief de producto que la sostiene, en [PRODUCT.md](PRODUCT.md).

## Requisitos de entorno

- Rust 1.100.0 (fijado en `rust-toolchain.toml`, edición 2024).
- `cargo-deny` 0.22.0 y `actionlint` 1.7.12 para los controles locales.
- Linux: `pkg-config`, `libpipewire-0.3-dev` y `libasound2-dev`. El host de producción es PipeWire nativo; ALSA es el respaldo. macOS usa CoreAudio y Windows WASAPI; esos hosts no están medidos.
- OpenSpec CLI 2.1+ (solo para el flujo de specs, no para compilar).

## Licencia

Por definir. El repositorio es público en `quicio/nacre`, pero no hay archivo `LICENSE` todavía. Decidirla antes de distribuir binarios o un plugin. Ver `docs/adr/0003-standalone-and-plugin-strategy.md`.
