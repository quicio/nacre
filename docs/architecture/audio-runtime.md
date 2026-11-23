# Runtime de audio

Este documento describe el plano de datos: cómo el sonido se produce, se modifica y se reemplaza sin romper el tiempo real.

## Contrato de tiempo real

El callback de audio es un **contrato**, no un lugar para la lógica. Debe cumplir:

| Prohibido en callback | Alternativa |
|---|---|
| `Vec::push`, `Box::new`, `String` | Buffers preasignados, arrays de tamaño fijo |
| `Mutex`, `RwLock`, `mpsc` | Ring buffer SPSC, atómicos |
| `Mutex` del patch vivo | Snapshot preparado, inmutable durante el bloque |
| `std::fs`, red, `println!` | Logs fuera del audio |
| Compilar/validar grafo | Preparar fuera, activar con un puntero |
| `panic!`, `unwrap()`, índices sin comprobar | Errores verificados al preparar |
| Llamar a UI o LLM | Colas de eventos y snapshots |

Presupuesto por bloque: procesamiento acotado y predecible. El coste no debe depender del número de mensajes pendientes ni del tamaño del patch vivo (solo del grafo preparado).

## Estructura del runtime

```text
AudioHost (nacre-audio)
├── DeviceThread          hilo del backend (cpal)
├── CommandQueue          SPSC: Control → Audio
├── ParameterSlots        atómicos por parámetro (f32 en bits)
├── PreparedGraph         grafo inmutable, con buffers propios
├── GraphSlot             activo + entrante (crossfade)
├── TransportClock        frames, tempo, posición musical
├── MeterBuffer           audio → control (triple buffer)
└── EventQueue            eventos programados (lookahead)
```

### Hilos

| Hilo | Responsabilidad | Puede bloquear |
|---|---|---|
| UI | Render, interacción, envío de comandos | Sí |
| Aplicación / control | Casos de uso, validación, compilación, agente | Sí |
| Audio (backend) | Callback, DSP, mezcla | **No** |
| Scheduler (M2) | Generación de eventos con lookahead | Sí (nunca toca el callback directamente) |

El scheduler nunca comparte estado mutable con el callback: produce eventos en un ring buffer que el callback drena.

## Ciclo de vida del grafo

```text
Patch document
     │  validate()        (fuera de audio; dominio)
     ▼
ValidationReport
     │  compile()         (fuera de audio; prepara buffers y orden)
     ▼
PreparedGraph ──── activación ────► GraphSlot (activo)
     │                                   │
     │                                   ├── crossfade N ms
     ▼                                   ▼
  descartado                    grafo nuevo estable
```

### Reglas

1. Un `PreparedGraph` es inmutable y autosuficiente: no referencia el documento ni la UI.
2. La compilación reserva toda la memoria necesaria (buffers por nodo, buffers de mezcla, tablas).
3. La activación ocurre al inicio de un bloque, nunca en medio del procesamiento de una muestra.
4. El grafo saliente se mantiene vivo durante el crossfade y se libera **fuera** del callback (se envía por un canal al hilo de control).
5. Si la compilación falla, el grafo activo sigue sonando; el error se reporta al plano de control.
6. Nunca hay dos grafos mutables a la vez.

### Crossfade

- Duración por defecto: 20 ms, configurable (10–100 ms).
- Curva equal-power (`cos/sin`) para evitar caída de nivel en el centro.
- Ambos grafos procesan en paralelo durante la transición; el coste temporal se duplica (aceptable: es una transición, no el estado estable).
- Estado no compartido: el grafo nuevo arranca desde cero (fase, envelopes). Para evitar clicks, el crossfade cubre el transitorio.
- Si el patch nuevo tiene un tail (reverb/delay), el crossfade de entrada se complementa con un fade-out del tail antiguo.

Casos que requieren cuidado adicional:

- Cambio de sample rate: se reconstruye el dispositivo; no se hace en caliente en el primer slice.
- Cambio de número de canales: solo con recompilación completa del host.

## Transferencia de parámetros

### Camino

```text
UI/agente → Application (valida) → CommandQueue (SPSC)
                                       │
                                 callback: drain al inicio del bloque
                                       │
                                 ParameterSlot (AtomicU32, bits f32)
                                       │
                                 Smoother (one-pole por parámetro)
                                       │
                                    DSP unit
```

- El comando contiene `ParameterId` + valor en unidad (no normalizado).
- El callback aplica como máximo `N` comandos por bloque (p. ej. 64) para acotar el coste; el resto espera al siguiente bloque. Los comandos descartados por cola llena se cuentan y se reportan.
- El smoothing vive en el procesador, no en la UI. Tiempo típico: 5–20 ms para cutoff/ganancia; 0 para parámetros discretos.
- Cambios discretos (waveform, modo de filtro) se aplican en frontera de bloque, con crossfade interno si producen discontinuidad.

### Parámetros y automatización futura

La misma ruta sirve para automatización del host (fase plugin): el host escribe en la cola desde el hilo de control; el audio no distingue origen. La diferencia es la densidad de eventos y la necesidad de sample-accuracy: se resuelve con un buffer de automatización por bloque, no con más locks.

## Reloj y transporte

- `SampleTime(u64)`: frames desde el inicio del transporte. Única fuente de verdad temporal.
- `Tempo`: BPM en `f64` (control), convertido a frames por beat en compilación.
- `MusicalPosition`: compás, beat, tick, derivado de `SampleTime` y tempo.
- El reloj del dispositivo es el maestro; el transporte se ancla al primer frame del bloque.
- No se usa `Instant::now()` dentro del callback para decisions musicales. El tiempo de pared solo se usa para diagnóstico fuera de audio.

## Render offline

El render offline comparte el mismo código DSP y el mismo compilador de grafo que el runtime, pero sin dispositivo:

```text
Patch + Composition + seed + duración
        │
        ▼
  OfflineRenderer (bloques de tamaño fijo, p. ej. 256)
        │
        ▼
  WAV (f32 o i16, según export)
```

Requisitos:

- Determinista: mismo input → mismo output byte a byte.
- Sin dependencia de reloj de pared ni de hilos.
- Mismo grafo preparado que el runtime (no una ruta "de test" distinta).
- Test de regresión por hash para patches de referencia.

## Medición y observación

Para la UI y el agente, el runtime publica un snapshot periódico (no cada bloque):

```rust
struct AudioSnapshot {
    rms: [f32; 2],
    peak: [f32; 2],
    spectral_centroid_hz: f32,
    event_density: f32,       // eventos por compás (medido en ventana)
    active_voices: u16,
    cpu_load: f32,
    xruns: u64,
    dropped_commands: u64,
}
```

- Cálculo de FFT fuera del callback (o en un hilo de análisis con buffer copiado).
- El agente consume este snapshot, nunca el audio crudo de forma continua.
- Los medidores son de solo lectura: no afectan al procesamiento.

## Inicialización y errores

- Si el dispositivo no soporta 48 kHz estéreo, se negocia el formato más cercano y se informa; el motor es agnóstico al sample rate (los coeficientes se recalculan al preparar).
- Si no hay dispositivo, la app arranca en modo "sin audio" y permite render offline; el error se muestra, no se silencia.
- Fallo al iniciar el stream: error estructurado (`AudioError::DeviceUnavailable`, `ConfigUnsupported`, `StreamBuildFailed`).
- Cierre: parar stream, drenar colas, liberar grafos en el hilo de control. Nunca `std::process::exit` desde audio.

## Pruebas del runtime

| Test | Qué verifica |
|---|---|
| `callback_no_alloc` | Un contador de asignaciones (allocator de test) permanece en 0 durante N bloques |
| `parameter_transfer` | Un cambio llega al DSP en ≤ 2 bloques, sin clicks |
| `graph_swap` | Discontinuidad máxima bajo umbral en un cambio de patch |
| `device_start_stop` | Arranque/parada repetidos sin fugas ni pánicos |
| `offline_determinism` | Mismo hash en dos renders |
| `xrun_stress` | 30 min con carga; contador de xruns estable |

Los tests de audio que necesitan dispositivo se marcan `#[ignore]` y corren en un job dedicado; los de DSP y offline corren siempre en CI.
