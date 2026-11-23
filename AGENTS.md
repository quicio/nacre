# AGENTS.md — Nacre

Instrucciones para agentes de código que trabajen en este repositorio. Léelo completo antes de escribir código.

## Qué es este proyecto

Nacre es un instrumento musical generativo en Rust: un motor de audio y composición procedural con capacidades de agente. No es un DAW ni un clon de modular. La prioridad es **calidad sonora y coherencia musical**, no cantidad de features.

## Estado actual

**M0 — planificación.** No existe código de audio. Hay arquitectura, ADRs, roadmap y cuatro changes de OpenSpec en `openspec/changes/`. El change activo es **`001-workspace-foundation`**; los demás están especificados y esperan a que el anterior se archive.

Antes de implementar, lee `HANDOFF.md` (estado real), `openspec/list` y la spec del change activo. Si el change no existe o no está validado, **no implementes**: propón primero.

## Flujo de trabajo obligatorio

1. Un cambio = un *change* de OpenSpec. Nada de código sin artefactos revisados.
2. Orden: `proposal.md` → `specs/<capability>/spec.md` → `design.md` → `tasks.md` → implementación.
3. Implementa **solo** las tareas de `tasks.md` del change activo, marcándolas al completarlas.
4. No implementes funcionalidad de changes futuros, aunque aparezca en el roadmap.
5. Al terminar todas las tareas: `openspec validate --strict`, tests verdes, y proponer archivar (no archivar sin revisión humana).

```bash
openspec list
openspec show <change> --json
openspec validate --strict
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Límites arquitectónicos (no negociables)

Dirección de dependencias: **Presentation → Application → Domain**. Infrastructure implementa puertos definidos hacia adentro. El dominio no conoce audio, UI, red, fs, IA ni frameworks.

- `nacre-domain`: sin dependencias externas de I/O, audio, UI o IA. Sin `serde` en el modelo de dominio (la persistencia mapea a DTOs propios).
- `nacre-application`: casos de uso, puertos, transacciones, revisión de estado. No conoce implementaciones concretas.
- `nacre-dsp`: DSP puro, sin UI ni fs. Puede depender de `nacre-domain` solo para tipos de parámetro si aporta claridad.
- `nacre-audio-runtime`: único lugar con acceso a dispositivo de audio y al callback. Depende de `dsp` y de puertos de `application`.
- `apps/standalone`: composición de dependencias (wiring). Ninguna lógica de negocio.
- `ui`: sin DSP ni mutación directa del grafo.

Prohibido:

- Dependencias circulares entre crates.
- Lógica de negocio en la capa de UI o en el backend de audio.
- Acceso a red, fs, locks bloqueantes o allocación en el callback de audio.
- `unwrap()`/`expect()` en rutas de audio en tiempo real (usar `Option`/valores seguros).
- Abstracciones genéricas "por si acaso" (traits sin segundo implementador real).

## Reglas de audio en tiempo real

El callback de audio **nunca** debe: asignar memoria, bloquear, hacer I/O, llamar a un LLM, compilar/validar el grafo, o ejecutar workflows de aplicación.

- Comunicación UI/agente → audio: canales SPSC acotados (`rtrb`) y parámetros atómicos.
- Comunicación audio → UI: snapshots atómicos y medidores con buffer circular.
- Todo cambio de grafo se compila y valida fuera del callback; se activa con crossfade.
- Sin `panic!`, sin índices sin comprobar, sin división por cero; NaN/Inf deben sanearse en la frontera de cada unidad DSP.

## Convenciones de código

- Rust edición 2024, `rust-toolchain.toml` fijado.
- Errores estructurados con `thiserror`; nada de `String` como error en APIs internas.
- Tipos explícitos: newtypes para `NodeId`, `ParameterId`, `SampleTime`, `Revision`. Sin `u64` desnudo en APIs públicas.
- Unidades en el nombre: `_hz`, `_ms`, `_db`, `_frames`. Ninguna ambigüedad de unidades.
- `unsafe` solo con comentario `// SAFETY:` y justificación; preferir alternativas seguras.
- Documentación de decisiones no obvias en el módulo, no en el commit.
- Los tests viven junto a la funcionalidad; los de integración en `tests/` del workspace.

## Testing mínimo por slice

- DSP: precisión de frecuencia, estabilidad del filtro, NaN/Inf, DC, clipping, continuidad de fase.
- Runtime: arranque/parada, transferencia de parámetros, ausencia de alloc en callback, apagado limpio.
- Offline: render determinista byte a byte con la misma semilla y parámetros.
- Integración: guardar/cargar patch, arranque de app, restauración de estado.

## Documentación

- ADR nueva en `docs/adr/` para toda decisión arquitectónica reversible con coste. Estado explícito: `Accepted`, `Proposed` o `Hypothesis`.
- No inventes certeza: si algo no está validado, márcalo como hipótesis y crea un spike en `docs/research/risks-and-spikes.md`.
- `HANDOFF.md` se actualiza al final de cada sesión de implementación.

## Idioma

Código, identificadores, nombres de tipos y APIs: **inglés**. Documentación de producto, arquitectura, ADRs y specs: **español**. Mensajes de commit: inglés, imperativo, convencional (`feat(audio): ...`).
