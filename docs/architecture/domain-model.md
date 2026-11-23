# Modelo de dominio

El dominio describe **qué es** el instrumento, no cómo suena ni cómo se ejecuta. No conoce `cpal`, `egui`, `serde`, el sistema de archivos ni ningún LLM.

## Tres conceptos, tres ciclos de vida

```text
   Patch                      Composition                 Performance
   cómo suena                 qué pasa musicalmente       qué ocurre ahora
   ─────────                  ───────────────────         ────────────────
   nodos, puertos,            generadores, eventos,       transporte, escenas,
   conexiones,                ritmo, evolución,           macros, cambios en vivo,
   parámetros, modulación     relaciones, secciones       transiciones
   reutilizable               reutilizable                efímera
```

- Un **Patch** puede usarse con muchas **Compositions**.
- Una **Composition** puede sonar con distintos **Patches**.
- Una **Performance** referencia un Patch y una Composition y aplica cambios sin destruir ninguno de los dos.

No se colapsan en un solo documento. Un patch es una configuración tímbrica/estructural; una composition es una partitura generativa; una performance es una sesión.

## Patch

### Entidades

| Tipo | Descripción | Invariantes |
|---|---|---|
| `Patch` | Documento raíz | `id`, `name`, `revision`, `nodes`, `connections`, `parameters` |
| `Node` | Unidad de procesamiento | `NodeId` único; `kind` declarado; puertos declarados por el kind |
| `Port` | Punto de conexión tipado | `PortId` único dentro del nodo; `direction` (in/out); `SignalType` |
| `Connection` | Enlace entre puertos | Compatibilidad de tipo y dirección; sin duplicados; sin auto-conexión |
| `Parameter` | Valor controlable | `ParameterId` estable; rango; default; unidad; curva |
| `ModulationRoute` | Control → parámetro | Fuente de control válida; profundidad acotada; sin rutas circulares no declaradas |

### Tipos de señal

```rust
enum SignalType {
    Audio { channels: ChannelLayout },   // estéreo por defecto
    Control { rate: ControlRate },       // audio-rate o bloque
    Event { kind: EventKind },           // note, trigger, mod, transport
}
```

La compatibilidad es explícita: `Audio` solo conecta con `Audio` del mismo layout (o mediante un nodo de conversión declarado). `Control` puede modular parámetros, no audio. `Event` no se mezcla con `Control`.

### Invariantes del patch

1. Todo `NodeId` y `ParameterId` es único y estable durante la vida del patch.
2. Toda `Connection` referencia puertos existentes y compatibles.
3. El grafo de audio es acíclico. El feedback solo existe dentro de una unidad con estado explícito (delay), nunca como ciclo entre nodos.
4. Un parámetro siempre tiene un valor dentro de su rango, con default válido.
5. Una modulación no puede apuntar a un parámetro inexistente ni exceder su profundidad máxima.
6. Añadir o quitar nodos no invalida parámetros de otros nodos.
7. El patch es serializable a un formato versionado; cualquier cambio incompatible incrementa la versión de formato.

### Operaciones de dominio (puras)

- `Patch::add_node(kind, id) -> Result<Node, DomainError>`
- `Patch::connect(from, to) -> Result<Connection, DomainError>`
- `Patch::set_parameter(id, value) -> Result<(), DomainError>`
- `Patch::validate() -> Result<(), Vec<ValidationIssue>>`
- `Patch::remove_node(id) -> Result<RemovedNode, DomainError>` (devuelve lo necesario para undo)

Ninguna hace I/O, ninguna toca el audio.

## Composition

### Entidades

| Tipo | Descripción |
|---|---|
| `Composition` | Documento raíz: `id`, `name`, `revision`, `generators`, `sections`, `evolution` |
| `Generator` | Fuente de eventos: `GeneratorId`, `kind`, `seed`, `clock`, `state`, `output` |
| `GeneratorRelation` | Influencia entre generadores (acoplamiento, herencia, restricción) |
| `MusicalEvent` | Evento con timestamp, destino, tipo, valor, duración |
| `Section` | Tramo estructural con reglas propias |
| `EvolutionRule` | Regla de cambio a escala micro/meso/macro |
| `MusicalIntent` | Descripción semántica de alto nivel (densidad, tensión, movimiento, atmósfera, inestabilidad) |

### Evento musical

```rust
struct MusicalEvent {
    time: SampleTime,          // frames desde el inicio del transporte
    target: EventTarget,       // generador, voz, parámetro, escena
    kind: EventKind,           // NoteOn, NoteOff, Trigger, Mod, Transport
    value: EventValue,         // pitch, velocity, duración, índice…
    duration: Option<Frames>,
}
```

`SampleTime` es un newtype sobre `u64`. No hay `f64` de segundos en la frontera de scheduling: la conversión ocurre una vez, al compilar.

### Invariantes de la composition

1. Toda semilla es explícita; no existe generador sin `seed`.
2. Los eventos son ordenables por `(time, sequence)` y el orden es estable.
3. Un generador no puede depender de sí mismo, directa ni transitivamente.
4. Las relaciones entre generadores son declaradas y acotadas (profundidad máxima).
5. La evolución siempre tiene límites: un presupuesto de cambio por escala temporal.
6. El mismo estado + semilla + parámetros produce la misma secuencia de eventos.

## Performance

| Tipo | Descripción |
|---|---|
| `Performance` | Sesión viva: transporte, patch activo, composition activa, macros |
| `TransportState` | `Stopped`, `Playing`, `Paused`; posición en frames y en compases |
| `Macro` | Control de alto nivel mapeado a varios parámetros |
| `Scene` | Subconjunto de estado reproducible (patch + composition + macros) |
| `LiveChange` | Cambio aplicado durante la performance, con origen (humano/agente) |

Invariantes:

1. Una performance nunca muta el patch o la composition originales: aplica *overlays*.
2. Todo cambio en vivo es atribuible (humano o agente) y reversible.
3. El transporte es la única fuente de tiempo; ningún componente inventa su propio reloj.
4. Cambiar de escena no puede producir un corte de audio (se resuelve con crossfade en el runtime).

## Parámetros

```rust
struct ParameterSpec {
    id: ParameterId,
    name: &'static str,
    unit: Unit,          // Hz, dB, ms, ratio, semitones, %
    range: RangeInclusive<f32>,
    default: f32,
    curve: Curve,        // linear, exponential, logarithmic
    smoothing: Smoothing,// none, one-pole { time_ms }
}
```

- El dominio guarda **valores en unidad**, no normalizados.
- La normalización 0..1 es una preocupación de presentación/automatización y se calcula con la curva.
- Un valor fuera de rango se rechaza en la frontera de aplicación; el dominio no "corrige" silenciosamente.
- `ParameterId` es un identificador estable y legible (`filter.cutoff`), no un índice de array.

## Errores

El dominio usa errores estructurados y explícitos (`thiserror`), agrupados por área:

- `PatchError`: nodo inexistente, puerto incompatible, ciclo, duplicado.
- `CompositionError`: generador inválido, relación circular, semilla ausente.
- `ParameterError`: fuera de rango, id desconocido, curva inválida.
- `PerformanceError`: transporte inválido, escena inexistente, cambio no autorizado.

Nada de `String` como error, nada de `unwrap()` en rutas de usuario.

## Lo que el dominio NO modela

- Sample rate, buffers, dispositivos, formatos de audio.
- Layout de memoria de DSP, SIMD, coeficientes de filtro.
- Serialización (la persistencia mapea a DTOs propios y versionados).
- Sesiones de agente, prompts, herramientas MCP.
- Ventanas, widgets, temas.

Si un concepto solo existe para que una librería funcione, no pertenece al dominio.
