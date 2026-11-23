# Design — workspace foundation

## Contexto

Este change no produce sonido: produce el terreno. Su valor se mide por lo que impide (mezclar capas, dependencias circulares, CI inexistente) y por lo que habilita (el change 002 sin fricción).

Referencias: `docs/architecture/overview.md`, `docs/architecture/dependency-graph.md`, ADR-0001.

## Estructura propuesta

```text
nacre/
├── Cargo.toml                  workspace (resolver = "3", edition = "2024")
├── rust-toolchain.toml         channel = "1.100.0", components: rustfmt, clippy
├── deny.toml                   licencias, duplicados, vulnerabilidades
├── .github/workflows/ci.yml    fmt + clippy + test + build + openspec + deny
├── scripts/check-deps.sh       grafo permitido
├── crates/
│   ├── nacre-domain/           lib: ids, errores base, versión de formato
│   ├── nacre-application/      lib: puertos mínimos (AudioHost, Clock, Renderer)
│   ├── nacre-dsp/              lib: SampleRate, BlockSize, gain util + test
│   └── nacre-audio-runtime/            lib: AudioConfig (sin cpal todavía; se añade en 002)
└── apps/
    └── nacre-standalone/       bin: main vacío que imprime versión y estado "sin audio"
```

### Contenido mínimo por crate (justificación)

| Crate | Qué contiene en VS1 | Por qué no está vacío |
|---|---|---|
| `nacre-domain` | Newtypes `PatchId`, `NodeId`, `ParameterId`, `Revision`; `DomainError` | Son la base de todo lo que viene y ya se pueden testear |
| `nacre-application` | Traits `AudioHost`, `Clock`, `Renderer` con firmas mínimas | El change 002 los implementa; definirlos ahora fija la frontera |
| `nacre-dsp` | `SampleRate`, `BlockSize`, `apply_gain` con test | Es el primer código DSP real y valida el crate en CI |
| `nacre-audio-runtime` | `AudioConfig` (sample rate, canales, buffer) con validación | El change 002 añade `cpal` sobre esta base |
| `nacre-standalone` | `main` que imprime versión y "audio no inicializado" | Verifica que el binario enlaza y arranca sin dispositivo |

Nada de `TODO` módulos vacíos, ni traits sin uso, ni carpetas `mod foo {}` de relleno.

## Decisiones

### D1 — Cinco crates, no catorce
La estructura propuesta del blueprint (`patch`, `graph`, `generative`, `modulation`, `composition`, `performance`, `agent-api`, ...) se crea **cuando exista** el código que la justifica. En VS1, `patch`/`graph` viven conceptualmente en `nacre-domain` y `nacre-audio-runtime`; se extraerán cuando su tamaño lo pida (ver ADR-0001).

### D2 — Toolchain fijado en 1.100.0
`egui` 0.37 exige MSRV 1.95; fijar 1.100.0 evita sorpresas de compilación y da un entorno reproducible. Se actualiza de forma explícita, no automática.

### D3 — Lints de workspace
`unsafe_code = "warn"` (no `deny`, porque `cpal`/SIMD pueden necesitarlo en el futuro), `unused_must_use = "deny"`, `clippy::float_cmp` en `warn` salvo tests, `missing_docs` en `warn` para APIs públicas.

### D4 — CI en Linux solamente
El audio en macOS/Windows se valida cuando exista el runtime (VS2/VS5). Añadir matrices antes multiplica fallos sin información útil. Excepción: `cargo check` multiplataforma puede añadirse como job no bloqueante.

### D5 — Verificación de dependencias
`scripts/check-deps.sh` compara `cargo tree` con la lista permitida de `docs/architecture/dependency-graph.md`:

- `nacre-domain` no debe contener `cpal`, `egui`, `serde`.
- `cpal` solo debe aparecer bajo `nacre-audio-runtime`.
- Sin ciclos.

Se ejecuta en CI. Un fallo es un error, no una advertencia.

### D6 — `cargo deny`
Licencias permitidas: `MIT`, `Apache-2.0`, `ISC`, `BSD-2-Clause`, `BSD-3-Clause`, `Unicode-3.0`, `Zlib`. `GPL-3.0` explícitamente **no** permitida hasta que se decida la licencia del producto (ADR-0003). Esto bloquea `vst3-sys` si alguien lo introduce por error.

### D7 — Validación de OpenSpec en CI
`openspec validate --strict` en el job de documentación. Evita specs inválidas y changes a medio escribir.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| Un solo crate con módulos | Rápido hoy, pero no se puede verificar la dirección de dependencias ni aislar `cpal` |
| Catorce crates del blueprint | Sobrearquitectura: la mayoría estaría vacía y la regla "no crear vacíos" se rompería el día uno |
| `cargo nextest` y matrix completa desde el inicio | Optimización prematura; `cargo test` basta para un workspace sin audio |
| `cargo-deny` solo manual | Se olvida; mejor en CI desde el primer commit |

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Crates de relleno que nadie usa | Contenido mínimo real y justificado; revisión en el gate de VS2 |
| El script de dependencias es frágil | Basado en `cargo metadata`, no en parseo de texto de `cargo tree` |
| CI lenta por `cargo deny` | Cache de dependencias; job separado |
| `unsafe_code = deny` bloquea `cpal` | Se usa `warn` y revisión manual de cada `unsafe` |

## Validación

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
./scripts/check-deps.sh
cargo deny check
openspec validate --strict
```

Criterio de salida: todos los comandos en verde, el binario arranca e imprime su versión, y ningún crate contiene módulos vacíos ni `todo!()`.
