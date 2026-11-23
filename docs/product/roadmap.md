# Roadmap

> Estado: **M0 — planificación**. Este documento es el plan vivo; se revisa al cerrar cada milestone, no en cada commit.

## Cómo se lee este roadmap

- **Milestone (`M`)**: objetivo de producto con un gate de salida verificable.
- **Vertical slice (`VS`)**: incremento implementable de punta a punta, con su propio change de OpenSpec.
- **Gate**: criterio que debe cumplirse para avanzar. Si falla, se corrige lo anterior; no se compensa con features nuevas.
- **Esfuerzo**: estimación en semanas de trabajo efectivo a ~15 h/semana, una persona. Es una horquilla para planificar, no un compromiso.

Regla de oro: **un slice no empieza hasta que el anterior está archivado**. El roadmap no es una lista de deseos paralelos.

## Vista general

| Milestone | Objetivo | Slices | Esfuerzo | Gate |
|---|---|---|---|---|
| **M0** Planificación | Arquitectura, specs, decisiones | — | 1–2 sem | Revisión humana de arquitectura y specs |
| **M1** First Sound | Un sintetizador standalone que suene a Nacre | VS1–VS6 | 8–12 sem | 3 renders de referencia + 30 min sin glitches |
| **M2** Generative Runtime | Sistemas que evolucionan sin perder identidad | VS7–VS11 | 8–12 sem | Pieza de 10 min con identidad |
| **M3** Agent Integration | Intención musical → patch → audición → refinamiento | VS12–VS15 | 6–9 sem | Cambio de patch reversible desde lenguaje natural |
| **M4** Instrument | Plugin, MIDI, performance, presets | VS16+ | 10–16 sem | Plugin validado en hosts objetivo |

Ruta crítica: VS1 → VS2 → VS3 → VS4 → VS5 → VS6 → VS7 → VS8 → VS10 → VS12 → VS13 → VS15.

> **Nota sobre los changes:** solo los cuatro primeros (`001`–`004`) existen como artefactos en `openspec/changes/`. Los identificadores `005`–`015` del roadmap son una previsión de secuencia, no changes creados: cada uno se redactará cuando su slice esté a punto de empezar, con la información que den los anteriores.

---

## M0 — Planificación

**Objetivo:** establecer una base técnica coherente y revisable antes de escribir audio.

Entregables (este repositorio):

- Visión, principios, roadmap.
- Arquitectura: capas, dominio, runtime de audio, motor generativo, agente.
- 8 ADRs con estado explícito.
- Evaluación de dependencias con datos de mantenimiento y licencia.
- Riesgos ordenados y spikes con criterio de salida.
- Cuatro changes de OpenSpec del primer tramo.

**Gate M0:** revisión humana de la arquitectura y de las specs 001–004. Sin aprobación no se implementa. Cambios solicitados se reflejan en los artefactos, no en código.

**Riesgo dominante:** sobrearquitectura. Mitigación: el primer slice solo usa 5 crates y un oscilador.

---

## M1 — First Sound

**Objetivo:** demostrar que el motor suena y que el camino en tiempo real es sólido. No se construye un modular completo.

### VS1 — Workspace foundation · `001-workspace-foundation`

- Workspace Cargo, toolchain fijado, crates mínimos, CI, lints, estructura de docs.
- Sin delta specs: es tooling (`skip_specs: true`).
- **Verificación:** `cargo test --workspace` y `cargo clippy -D warnings` en CI sobre un workspace que compila; `openspec validate --strict` limpio.

### VS2 — Audio foundation · `002-audio-foundation`

- Dispositivo de salida, callback estéreo 48 kHz (con negociación), reloj de frames, arranque/parada.
- Oscilador seno con fase continua, parámetros de frecuencia/ganancia, transferencia sin locks.
- Render offline a WAV reproducible.
- App mínima: ventana con Play/Stop y ganancia maestra.
- **Spikes previos:** S1 (CPAL/backends), S2 (transferencia de parámetros).
- **Verificación:** 30 min continuos sin xruns ni clicks; dos renders offline con los mismos parámetros son idénticos (hash); el callback no asigna memoria (test de asignación).

### VS3 — Voice synthesis · `003-voice-synthesis`

- Saw con anti-aliasing (PolyBLEP u oversampling, decidido en S4), ADSR, filtro resonante TPT/SVF, LFO, saturación suave, DC blocker.
- **Verificación:** análisis espectral de aliasing bajo umbral acordado; estabilidad del filtro en barridos extremos; envolvente sin clics; patch de referencia "dron", "bajo", "metal".

### VS4 — Parameter system · `004-parameter-system`

- Parámetros tipados (id estable, rango, curva, unidad), smoothing, snapshot para UI, validación y clamping.
- **Verificación:** cambios de parámetro sin zipper noise ni clicks; valores fuera de rango rechazados/saneados; test de concurrencia UI/audio.

### VS5 — Space and character · `005-space-and-character`

- Delay, reverb algorítmica básica, bus maestro, soft clip, gestión de headroom.
- **Verificación:** sin clipping digital en patches de referencia; cola de reverb sin artefactos; CPU < 15 % de un núcleo a 48 kHz/256.

### VS6 — Patch document and persistence · `006-patch-document`

- Modelo de patch serializable, versión de formato, guardar/cargar, migración futura, render determinista desde archivo.
- **Verificación:** round-trip patch → archivo → patch; render offline idéntico tras cargar; patch inválido produce error estructurado, no pánico.

### Gate M1

1. Tres renders de referencia (dron evolutivo, bajo pulsante, textura metálica), cada uno ≥ 2 min, sin muestras externas.
2. 30 minutos continuos sin glitches ni xruns en el equipo de desarrollo.
3. Cambio de cualquier parámetro sin clicks audibles.
4. Render offline determinista (mismo hash con mismos inputs).
5. CPU estable y sin crecimiento de memoria en ejecución prolongada.
6. **Escucha crítica:** alguien que no sea el autor describe la identidad sonora sin que se le diga qué esperar.

**Si el gate falla en sonido:** se invierte en DSP (VS3/VS5), no en features nuevas. **Kill criterion:** si tras dos iteraciones de DSP los renders siguen sin convencer, se replantea la arquitectura de voz y síntesis antes de seguir.

**No-alcance de M1:** grafo modular editable, MIDI, plugin, agente, generadores, presets de usuario, múltiples patches simultáneos.

---

## M2 — Generative Runtime

**Objetivo:** que el instrumento produzca música que evoluciona y sigue siendo reconocible.

### VS7 — Clock and scheduler · `007-clock-and-scheduler`

Transporte con tiempo en frames, scheduler con lookahead, cola de eventos acotada, cuantización musical, start/stop/continue.
**Verificación:** jitter de eventos < 1 ms equivalente; sin allocación en el hilo de scheduling; eventos ordenados y sin duplicados bajo carga.

### VS8 — Generators I · `008-generators-core`

Euclidean, probability, polyrhythm. Configuración por generador, semilla, salida de eventos, reloj compartido.
**Verificación:** rejillas euclidianas correctas para (n,k) conocidos; distribución de probabilidad dentro de tolerancia estadística; reproducibilidad exacta por semilla.

### VS9 — Generators II · `009-generators-motion`

Random walk, drift, cycle/orbit, tension. Relaciones generador→generador (influencia, acoplamiento, herencia de semilla).
**Verificación:** dos generadores acoplados no colapsan ni divergen; el walk respeta límites musicales; sin deriva numérica en 1 h.

### VS10 — Evolution · `010-evolution`

Evolución micro (timing/velocity/pitch), meso (variación de patrón por compases), macro (estructura por secciones). Presupuesto de cambio y "memoria" de identidad.
**Verificación:** una pieza de 10 min conserva el motivo rítmico principal (analizable por histograma de intervalos) y no repite literalmente.

### VS11 — Composition and scenes · `011-composition-scenes`

Composition como documento: secciones, escenas, transiciones, relación con patch(es). Performance mínima (macros).
**Verificación:** cambio de escena sin corte audible; la misma composition funciona con dos patches distintos.

### Gate M2

1. Pieza generada de ≥ 10 min con identidad rítmica y armónica reconocible.
2. Reproducible: misma semilla → mismo MIDI/eventos.
3. Interacción en vivo: cambiar densidad/tensión se escucha en < 1 compás sin romper el groove.
4. Estabilidad en ejecución de 1 h sin fugas ni deriva.

**Riesgo dominante:** que suene a "generador aleatorio". Mitigación: VS10 con presupuesto de cambio y evaluación por escucha antes de añadir generadores.

---

## M3 — Agent Integration

**Objetivo:** que una intención musical en lenguaje natural se convierta en un cambio de patch audible, explicable y reversible.

### VS12 — Agent API and inspection · `012-agent-api`

API independiente del proveedor: inspección de patch, grafo, generadores, modulación; lectura de observaciones (RMS, peak, centroide espectral, densidad de eventos, actividad de voces).
**Verificación:** un cliente de prueba (sin LLM) puede leer todo el estado y producir un informe estructurado.

### VS13 — Typed operations and transactions · `013-patch-transactions`

Operaciones tipadas, validación, revisión base, construcción de estado candidato, preview, apply/discard. Sin mutación directa.
**Verificación:** operación sobre revisión obsoleta falla con error estructurado; apply es atómico; discard no deja rastros; preview no altera el estado vivo.

### VS14 — Undo/redo and history · `014-undo-redo`

Historial por transacción, límites de memoria, agrupación, restauración exacta.
**Verificación:** 100 transacciones de ida y vuelta restauran el estado original bit a bit (o equivalente estructural); undo durante reproducción no produce clicks.

### VS15 — MCP adapter and permissions · `015-agent-mcp`

Adaptador MCP sobre la Agent API, tres modos (collaborator, explorer, director), permisos, rate limits, auditoría de operaciones.
**Verificación:** un agente externo vía MCP construye un patch desde una intención, lo audiciona y lo revierte; el modo director no puede ejecutar operaciones destructivas sin aprobación.

### Gate M3

1. "Crea un dron metálico lento que evolucione" → patch funcional en < 60 s de agente.
2. "Menos predecible sin subir la densidad" → cambio medible en eventos, no en volumen.
3. Toda acción es reversible y explicable en una frase.
4. El instrumento sigue funcionando completo con el agente desactivado.

---

## M4 — Instrument (post-MVP)

No comprometido hasta cerrar M3.

- **VS16 Plugin**: VST3 y CLAP sobre el mismo runtime; editor; estado; automatización; sincronía de transporte (depende de S5).
- **VS17 MIDI**: entrada/salida, clock, MPE opcional.
- **VS18 Performance**: escenas, macros, transiciones, controladores.
- **VS19 Presets and sharing**: formato de patch portable, presets de fábrica, validación de compatibilidad.
- **VS20 DSP expansion**: granular, wavetable, modelado físico, procesamiento espectral.
- **VS21 Agent local**: Ollama y modelos locales, sin dependencia de nube.

---

## Enablers transversales

No son slices; se hacen cuando un slice los necesita.

| Enabler | Cuándo | Nota |
|---|---|---|
| CI (fmt, clippy, test, build Linux) | VS1 | macOS/Windows se añaden al estabilizar audio |
| Harness de render offline | VS2 | Base para tests de sonido y comparación de hashes |
| Banco de escucha (renders de referencia) | VS3 | 3–5 archivos versionados con criterios |
| Benchmarks de callback | VS2/VS5 | Presupuesto de CPU por bloque |
| Test de asignación en audio | VS2 | Detectar alloc/lock en el callback |
| Fuzzing de patch/JSON | VS6 | Robustez del parser |
| Trazas/observabilidad del agente | VS13 | Auditoría de operaciones |

---

## Secuencia y paralelización

```text
M0 ──► VS1 ──► VS2 ──► VS3 ──► VS4 ──► VS5 ──► VS6 ══ Gate M1
                                            │
                    ┌───────────────────────┘
                    ▼
                   VS7 ──► VS8 ──► VS9 ──► VS10 ──► VS11 ══ Gate M2
                                                        │
                                                        ▼
                                                      VS12 ──► VS13 ──► VS14 ──► VS15 ══ Gate M3
                                                                                    │
                                                                                    ▼
                                                                                 M4 (VS16+)
```

Paralelizable con cuidado:

- VS9 (generadores) puede avanzar en paralelo con VS5/VS6 si el contrato de eventos de VS7 está congelado.
- VS12 (inspección) puede empezar cuando el modelo de dominio de VS6 está estable, sin esperar a VS11.
- La UI nunca va por delante del contrato de aplicación: primero el caso de uso, después el control.

No paralelizar: VS2 con VS3 (el contrato de voz depende del callback), VS13 con VS12 (las operaciones dependen del modelo de lectura).

---

## Definition of Done (cada slice)

1. Todas las tareas de `tasks.md` marcadas.
2. `openspec validate --strict` y `cargo test --workspace` verdes.
3. Tests nuevos cubren lo que la spec promete; ningún test deshabilitado.
4. Sin `unsafe` nuevo sin justificación; sin allocación en el callback.
5. Render o medición que demuestre el comportamiento (cuando aplique sonido).
6. `HANDOFF.md` actualizado.
7. ADR creado o actualizado si hubo una decisión relevante.
8. Change archivado y specs principales actualizadas.

## Gestión del alcance

- Una idea nueva durante la implementación **no se implementa**: se anota en el backlog de producto y se evalúa al cerrar el milestone.
- Si un slice crece más de lo previsto, se parte en dos changes, no se estira el gate.
- Los gates no se negocian hacia abajo; el alcance sí.
