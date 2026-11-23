# HANDOFF — estado de implementación

> Documento de continuidad. Se actualiza al final de cada sesión. Un agente nuevo debe leer esto primero, después `AGENTS.md`.

## Última actualización

**2026-11-22** — Fase M0 (planificación). Sin código de audio.

## Qué existe

- `README.md`, `AGENTS.md`, este archivo.
- Visión y principios de producto (`docs/product/`).
- Roadmap por milestones y vertical slices (`docs/product/roadmap.md`).
- Arquitectura: overview, modelo de dominio, runtime de audio, motor generativo, agente, grafo de dependencias (`docs/architecture/`).
- 8 ADRs (`docs/adr/`).
- Riesgos y spikes priorizados (`docs/research/risks-and-spikes.md`).
- Evaluación de dependencias con datos reales de crates.io/GitHub (`docs/research/dependency-evaluation.md`).
- OpenSpec 2.1 inicializado en `openspec/` con 4 changes planificados.

## Qué NO existe todavía

- `Cargo.toml` / workspace Rust.
- Cualquier crate, test o binario.
- CI.
- Plugin (VST3/CLAP), MIDI, UI.

## Change activo

**`001-workspace-foundation`** — pendiente de aprobación humana y de `openspec validate --strict`.

Al aprobarse: implementar `tasks.md` en orden, sin adelantar los changes 002–004.

## Estado de los changes

| Change | Artefactos | Implementado | Notas |
|---|---|---|---|
| `001-workspace-foundation` | proposal, design, tasks | No | Sin delta specs (`skip_specs: true`): es tooling |
| `002-audio-foundation` | proposal, specs, design, tasks | No | Bloqueado por 001; requiere spike S1 |
| `003-voice-synthesis` | proposal, specs, design, tasks | No | Bloqueado por 002 |
| `004-parameter-system` | proposal, specs, design, tasks | No | Bloqueado por 003 |

## Decisiones abiertas (no inventar certeza)

1. **Licencia del proyecto.** Sin decidir. `nih-plug` es ISC, `cpal` Apache-2.0, `fundsp` MIT/Apache-2.0: compatibles entre sí, pero la licencia del producto (¿MIT? ¿Apache? ¿propietaria?) debe decidirse antes de publicar.
2. **Backend de audio en Linux.** ALSA directo vs JACK vs PipeWire-JACK. Spike S1.
3. **PolyBLEP vs oversampling** para saw. Spike S4.
4. **Formato de patch**: JSON legible vs binario compacto vs ambos con versión. Ver ADR-0008.
5. **Alcance del plugin**: VST3 primero, CLAP después (o simultáneos). Depende del spike S5.
6. **Nombre definitivo del producto** ("Nacre" es el nombre de trabajo).

## Cómo continuar

```bash
cd /home/hugo/projects/nacre
openspec list --specs          # capacidades construidas (vacío hoy)
openspec show 001-workspace-foundation
openspec validate --strict
```

Si el change activo está aprobado, implementa sus tareas. Si no, pregunta antes de escribir código.
