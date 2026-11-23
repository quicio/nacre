# HANDOFF — estado de implementación

> Documento de continuidad. Se actualiza al final de cada sesión. Un agente nuevo debe leer esto primero, después `AGENTS.md`.

## Última actualización

**2026-11-23** — Change `001-workspace-foundation` implementado y verificado. Sin código de audio todavía.

## Qué existe

- `README.md`, `AGENTS.md`, este archivo.
- Visión y principios de producto (`docs/product/`).
- Roadmap por milestones y vertical slices (`docs/product/roadmap.md`).
- Arquitectura: overview, modelo de dominio, runtime de audio, motor generativo, agente, grafo de dependencias (`docs/architecture/`).
- 8 ADRs (`docs/adr/`).
- Riesgos y spikes priorizados (`docs/research/risks-and-spikes.md`).
- Evaluación de dependencias con datos reales de crates.io/GitHub (`docs/research/dependency-evaluation.md`).
- OpenSpec 2.1 inicializado en `openspec/` con 4 changes planificados.

### Identidad y sitio (2026-11-22)

- `PRODUCT.md`: verdad de producto y restricciones de marca para el trabajo de diseño.
- `DESIGN.md`: sistema visual (obsidiana, nácar, Bodoni Moda + Archivo + Azeret Mono, reglas de 1 px, cero radios).
- `site/`: Astro estático bilingüe ES/EN con el sitio público (estado, roadmap, principios, arquitectura, specs, dos maquetas conceptuales).
- `.github/workflows/pages.yml`: despliegue a GitHub Pages.
- `.impeccable/`: configuración de Impeccable, comp aprobada (`mocks/approved/`) y sidecar de tokens (`design.json`).
- La placa de nácar del hero y del cierre es una imagen generada, con prompt embebido en el PNG (`embed-prompt`); las dos maquetas de UI están etiquetadas como **concepto** y también llevan su procedencia.
- Logo de marca vectorizado a partir del PNG entregado: `assets/brand/nacre-logo.svg` (currentColor), `-light` (perla), `-ink` (obsidiana), símbolo independiente y marca simplificada para favicon. El original conserva el halo del PNG fuente; los SVG son el material de uso.

## Qué NO existe todavía

- Audio real: el crate `nacre-audio-runtime` solo tiene `AudioConfig`; no hay `cpal`, callback ni dispositivo.
- Plugin (VST3/CLAP), MIDI, UI.
- Spikes S1 (backends CPAL) y S2 (transferencia RT): hipótesis, no mediciones.

## Change activo

**`002-audio-foundation`** — especificado, no implementado. Antes de empezar exige los spikes S1 y S2 (`docs/research/risks-and-spikes.md`); su promesa central (callback sin asignaciones ni locks con CPAL) es una hipótesis hasta que se midan.

El change `001-workspace-foundation` está implementado y verificado, pendiente de revisión humana y de `openspec archive`. No archivar sin esa revisión.

## Estado de los changes

| Change | Artefactos | Implementado | Notas |
|---|---|---|---|
| `001-workspace-foundation` | proposal, design, tasks | Sí (2026-11-23) | 16/16 tareas; 10 tests; gates en verde. Pendiente de archivar |
| `002-audio-foundation` | proposal, specs, design, tasks | No | Requiere spikes S1 y S2 antes de empezar |
| `003-voice-synthesis` | proposal, specs, design, tasks | No | Bloqueado por 002 |
| `004-parameter-system` | proposal, specs, design, tasks | No | Bloqueado por 003 |

## Decisiones abiertas (no inventar certeza)

1. **Licencia del proyecto.** Sin decidir. `nih-plug` es ISC, `cpal` Apache-2.0, `fundsp` MIT/Apache-2.0: compatibles entre sí, pero la licencia del producto (¿MIT? ¿Apache? ¿propietaria?) debe decidirse antes de publicar.
2. **Backend de audio en Linux.** ALSA directo vs JACK vs PipeWire-JACK. Spike S1.
3. **PolyBLEP vs oversampling** para saw. Spike S4.
4. **Formato de patch**: JSON legible vs binario compacto vs ambos con versión. Ver ADR-0008.
5. **Alcance del plugin**: VST3 primero, CLAP después (o simultáneos). Depende del spike S5.
6. **Nombre definitivo del producto** ("Nacre" es el nombre de trabajo). Ya está en los cinco crates y en el binario; cambiarlo ahora es un refactor del workspace completo.

## Cómo continuar

```bash
cd /home/hugo/projects/nacre
openspec list --specs          # capacidades construidas (vacío hoy)
openspec show 001-workspace-foundation
openspec validate --all --strict
```

Si el change activo está aprobado, implementa sus tareas. Si no, pregunta antes de escribir código.
