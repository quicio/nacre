# Motor generativo

El motor generativo es independiente del motor de audio. Produce **eventos musicales**, no muestras. Esto permite testearlo sin dispositivo, renderizarlo offline, serializarlo y razonar sobre él.

## Principio

```text
Generador  →  MusicalEvent  →  scheduler  →  voz / parámetro / escena
   ↑                                                │
   └──────────  evolución (estado, semilla)  ←──────┘
```

El generador no sabe si el evento acabará en un oscilador, en un filtro o en una escena. Solo conoce el tiempo, el destino y el valor.

## Reloj

- Fuente única: `SampleTime` del transporte.
- El scheduler trabaja con **lookahead** (p. ej. 2 bloques) para absorber jitter del hilo de control.
- Los eventos se entregan en orden `(time, sequence)`; el orden es estable y determinista.
- Cuantización opcional a rejilla musical (1/4, 1/8, 1/16, 1/8T…) o libre.
- Latencia del scheduler acotada: el evento se genera antes de su tiempo de ejecución, nunca después.

## Generadores iniciales

| Generador | Idea musical | Estado interno | Parámetros clave |
|---|---|---|---|
| **Euclidean** | Distribución de pulsos con máxima uniformidad | posición, fase | `steps`, `pulses`, `rotation`, `accent` |
| **Probability** | Aparición con peso, no moneda justa | RNG, historia corta | `probability`, `humanize`, `accent_bias` |
| **Polyrhythm** | Varias rejillas simultáneas desfasadas | fase por capa | `layers`, `ratios`, `offset` |
| **Random Walk** | Deriva melódica con memoria | posición actual, dirección | `step_range`, `inertia`, `bounds` |
| **Drift** | Desviación lenta de un centro | valor, velocidad | `drift_rate`, `depth`, `return_force` |
| **Cycle / Orbit** | Patrones que regresan con variación | ciclo, fase, variación | `length`, `mutation`, `orbit` |
| **Markov** | Transiciones con pesos musicales | matriz, estado | `states`, `transition_bias` |
| **Fractal** | Autosimilitud rítmica | profundidad, ramas | `depth`, `branching`, `contraction` |
| **Tension** | Modulación de tensión armónica | acumulador, objetivo | `target`, `rate`, `release` |

Regla: **cada generador debe poder describirse en una frase musical**. Si no, no entra.

## Contrato de un generador

```rust
trait Generator {
    fn id(&self) -> GeneratorId;
    fn seed(&self) -> Seed;
    fn reset(&mut self, at: SampleTime);
    fn next_events(&mut self, window: TimeWindow, out: &mut EventBuffer);
    fn apply(&mut self, op: GeneratorOp) -> Result<(), GeneratorError>;
    fn observe(&self) -> GeneratorObservation;
}
```

Notas de diseño:

- `next_events` recibe una ventana y escribe en un buffer preasignado. Sin allocación por evento.
- El generador es determinista para `(seed, state, window)`.
- `observe()` devuelve telemetría (actividad, densidad, último evento) para UI y agente, sin exponer el RNG.
- No hay traits genéricos de "nodo generativo": un generador concreto es una estructura con estado, no un framework.

## Relaciones entre generadores

```text
GeneratorRelation
├── Influence { from, to, parameter, depth, curve }   // modula un parámetro de otro
├── Coupling  { a, b, strength, mode }                // phase lock / anti-phase
├── Constraint{ target, rule }                        // límites musicales
└── SeedLineage { parent, child, variation }          // herencia de material
```

Límites:

- Profundidad máxima de influencia (p. ej. 2 saltos) para evitar bucles impredecibles.
- Sin ciclos directos; el acoplamiento es simétrico y declarado, no un feedback oculto.
- Toda relación es visible en el documento y explicable por el agente.

## Evolución

Tres escalas, con presupuestos distintos y evaluación independiente:

| Escala | Periodo típico | Qué cambia | Qué no cambia |
|---|---|---|---|
| **Micro** | 1–4 beats | timing, velocity, micro-pitch, articulación | patrón, armonía |
| **Meso** | 4–32 bars | variantes de patrón, fills, densidad, registro | motivo principal, centro tonal |
| **Macro** | 1–8 min | secciones, instrumentación, densidad global, tensión | identidad (motivo, paleta, carácter) |

### Presupuesto de cambio

No se muta "un poco al azar". Cada escala tiene un **presupuesto** (p. ej. máximo 15 % de eventos alterados por compás) y una **memoria** de identidad (histograma de intervalos, perfil rítmico, centro tonal). Una mutación que destruye la identidad se rechaza o se atenúa.

```text
mutación propuesta
      │
      ▼
  evaluar identidad (distancia a la memoria)
      │
      ├── dentro de presupuesto → aplicar
      └── fuera → reducir o rechazar, registrar motivo
```

Esto es lo que separa a Nacre de un `randomize`.

### Semillas

- `Seed(u64)` explícita por generador.
- Semilla maestra de la composition; las semillas hijas se derivan de forma determinista (`splitmix64(master, generator_index)`).
- Misma semilla + mismo estado + misma ventana → mismos eventos.
- Cambiar la semilla es una operación explícita y visible, nunca implícita.

## Salida de eventos

- Buffer acotado por ventana; si se desborda, se reporta (no se descarta en silencio).
- Cada evento lleva su origen (`GeneratorId`) para trazabilidad y depuración.
- Los eventos de transporte (start/stop/tempo) no son eventos generativos: van por el canal de control.

## Integración con audio

```text
Scheduler (control)  ──EventQueue(SPSC)──►  Callback (audio)
                                               │
                                     SampleTime del bloque
                                               │
                                     consume eventos ≤ block_end
                                               │
                                     voz / parámetro / escena
```

- El callback consume eventos al inicio del bloque y los aplica con precisión de muestra cuando el destino lo permite.
- Un evento con tiempo dentro del bloque se aplica en el offset correcto, no al principio.
- Si el evento llega tarde (después de su tiempo), se aplica al inicio del bloque siguiente y se cuenta como `late_events`.

## Pruebas

| Test | Qué verifica |
|---|---|
| `seed_reproducibility` | Misma semilla → secuencia idéntica de eventos en 10 000 compases |
| `euclidean_reference` | Rejillas (n,k) comparadas con implementación de referencia |
| `event_ordering` | Orden estable con timestamps iguales |
| `no_drift_long_run` | 1 h de simulación sin deriva de fase ni acumulación de error |
| `identity_budget` | Una mutación fuera de presupuesto se rechaza |
| `relation_cycles` | Relaciones circulares se detectan y se rechazan |

Los tests del motor generativo no necesitan audio: corren en CI siempre.
