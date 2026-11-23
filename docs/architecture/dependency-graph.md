# Grafo de dependencias del workspace

## Estructura objetivo (primer tramo)

```text
apps/
└── nacre-standalone        binario: composición de dependencias + UI mínima

crates/
├── nacre-domain            sin deps internas ni I/O
├── nacre-application       → nacre-domain
├── nacre-dsp               → nacre-domain (solo newtypes compartidos)
├── nacre-audio             → nacre-dsp, nacre-application, nacre-domain
└── nacre-persistence       → nacre-application, nacre-domain   (VS6)
```

Crates que aparecerán **solo cuando exista su responsabilidad**:

| Crate | Cuándo | Motivo de la separación |
|---|---|---|
| `nacre-generative` | M2 | Motor de eventos independiente del audio; testable sin dispositivo |
| `nacre-ui` | M4 (plugin) | Segunda presentación real; evita duplicar la UI en dos apps |
| `nacre-agent-api` | M3 | Contrato estable para agentes, independiente de MCP |
| `nacre-agent-mcp` | M3 | Adaptador; aísla la dependencia del SDK de MCP |
| `nacre-render` | VS6 (si crece) | Render offline como servicio, reutilizado por CLI y tests |

No se crean crates vacíos "para que exista el sitio". Un crate nuevo requiere: responsabilidad propia, límite de dependencias verificable y tests propios.

## Grafo permitido

```text
        ┌──────────────────────┐
        │  nacre-standalone    │
        └───┬──────┬───────┬───┘
            │      │       │
            │      │       └────────────► nacre-persistence ──► nacre-application
            │      │                                                  │
            │      └────────────► nacre-audio ──► nacre-dsp ──────────┤
            │                        │                                │
            └────────────────────────┴────────────────────────────────┴──► nacre-domain
```

## Reglas verificables

1. `nacre-domain` no declara dependencias de workspace ni crates de infraestructura.
2. `nacre-application` solo puede depender de `nacre-domain`.
3. `nacre-dsp` no puede depender de `nacre-application`, `nacre-audio`, `nacre-persistence`, UI ni red.
4. `nacre-audio` no puede depender de UI ni de persistencia.
5. La UI no puede depender de `nacre-dsp` ni de `nacre-audio`.
6. Ningún crate puede depender de `apps/*`.
7. Sin ciclos, ni directos ni transitivos.
8. `serde` no entra en `nacre-domain`; la persistencia usa DTOs propios.

Estas reglas se comprueban en CI con `cargo tree` y un test de dependencias (p. ej. `cargo-deny` para licencias/duplicados y un script que compare el grafo permitido). Si una regla necesita excepción, se documenta en un ADR; no se relaja el script.

## Dependencias externas por crate (objetivo)

| Crate | Externas previstas | Nota |
|---|---|---|
| `nacre-domain` | `thiserror` | Sin `serde`, sin `rand` |
| `nacre-application` | `thiserror` | Puertos y casos de uso |
| `nacre-dsp` | `libm` (opcional) | Sin `cpal`, sin `egui` |
| `nacre-audio` | `cpal`, `rtrb`, `atomic_float` (o atómicos propios) | `cpal` solo aquí |
| `nacre-persistence` | `serde`, `serde_json`, `toml` | DTOs versionados |
| `nacre-standalone` | `eframe`, `egui`, `env_logger` | UI y wiring |
| `nacre-agent-mcp` | SDK MCP | Adaptador aislado |

## Presupuesto de dependencias

Antes de añadir una dependencia:

1. ¿Se puede resolver con la std en menos de ~100 líneas? Si sí, no se añade.
2. ¿Está mantenida? (último release < 12 meses, issues activos, bus factor razonable)
3. ¿Licencia compatible con la del producto? (ver `docs/research/dependency-evaluation.md`)
4. ¿Es segura en tiempo real? (sin alloc/locks en el camino de audio)
5. ¿Añade peso de compilación o plataformas que no necesitamos?
6. ¿Se puede aislar detrás de un crate adaptador?

Toda dependencia nueva se registra en el ADR correspondiente o en el PR con justificación.

## Estructura de directorios

```text
nacre/
├── Cargo.toml               workspace
├── rust-toolchain.toml
├── AGENTS.md
├── HANDOFF.md
├── README.md
├── apps/
│   └── nacre-standalone/
├── crates/
│   ├── nacre-domain/
│   ├── nacre-application/
│   ├── nacre-dsp/
│   ├── nacre-audio/
│   └── nacre-persistence/
├── assets/
│   ├── presets/
│   └── reference/           renders de referencia para escucha
├── docs/
│   ├── product/
│   ├── architecture/
│   ├── adr/
│   └── research/
├── openspec/
│   ├── specs/               capacidades construidas
│   └── changes/             cambios en curso y archivo
└── tests/                   integración de workspace
```

Los directorios `assets/samples` y `assets/wavetables` se crearán cuando exista el primer sample o wavetable real, no antes.
