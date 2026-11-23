# Design — audio foundation

## Contexto

Primer slice con sonido real. El objetivo es demostrar el camino completo y su seguridad en tiempo real, no construir el sintetizador. Al terminar, el change 003 sustituye el seno por una voz completa sin cambiar la frontera.

Referencias: `docs/architecture/audio-runtime.md`, ADR-0002, ADR-0005.

## Flujo del slice

```text
UI (eframe)                     Aplicación                  Audio
───────────                     ───────────                 ─────
Play ──────────────► start_audio() ──► AudioHost::start()
                                              │
                                              ▼
                                        cpal stream
Gain slider ───────► set_parameter(Gain)
                          │ validación
                          ▼
                    CommandQueue (rtrb) ─────────► callback: drain (≤64)
                                                          │
                                                    SineOscillator
                                                          │
                                                    master gain
                                                          ▼
                                                     output f32
```

## Componentes

### `nacre-dsp`

```rust
pub struct SineOscillator { phase: f64, phase_increment: f64 }
impl SineOscillator {
    pub fn prepare(&mut self, sample_rate: f32, frequency_hz: f32);
    pub fn set_frequency(&mut self, sample_rate: f32, frequency_hz: f32);
    pub fn process(&mut self, out: &mut [f32], gain: f32);
    pub fn reset(&mut self);
}
```

- Fase en `f64` normalizada a [0,1) para evitar deriva; incremento recalculado al cambiar frecuencia.
- `process` procesa un bloque mono; el runtime lo replica a estéreo (sin coste de DSP duplicado).
- Saneado: si `gain` no es finito, se usa 0.

### `nacre-audio`

```rust
pub struct AudioConfig { sample_rate: u32, channels: u16, buffer_frames: u32 }
pub struct AudioHost { /* cpal stream, colas, slots */ }

impl AudioHost {
    pub fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError>;
    pub fn stop(&mut self);
    pub fn submit(&self, command: Command) -> Result<(), SubmitError>;
    pub fn snapshot(&self) -> AudioSnapshot;
}
```

- `Command`: `SetFrequency(f32)`, `SetGain(f32)`, `Start`, `Stop`, `SetTransport`.
- Cola `rtrb::Producer/Consumer<Command>` con capacidad 256; `submit` devuelve `Full` en vez de bloquear.
- Parámetros actuales también en `AtomicU32` (bits de `f32`) como respaldo para la UI.
- El callback drena hasta 64 comandos por bloque; el resto queda para el siguiente.
- Estado: `AtomicU64` de frames, `AtomicU32` de xruns, `AtomicU32` de descartes.
- `snapshot()` lee los atómicos y calcula RMS/peak desde un buffer de medición escrito por el callback (triple buffer o array atómico de 8 slots).

### `nacre-application`

Casos de uso finos, sin conocer `cpal`:

```rust
pub trait AudioHostPort {
    fn start(&mut self, cfg: AudioConfig) -> Result<AudioStatus, AudioError>;
    fn stop(&mut self);
    fn submit(&self, cmd: ParameterCommand) -> Result<(), CommandError>;
    fn status(&self) -> AudioStatus;
}
pub struct StartAudio { host: Box<dyn AudioHostPort> }
impl StartAudio { pub fn execute(&mut self) -> Result<AudioStatus, AppError>; }
```

El trait es el puerto; `nacre-audio` lo implementa. La app compone.

### `apps/nacre-standalone`

- `eframe::run_native` con una ventana de ~480×240.
- Estado de UI: `playing`, `gain_db`, `status_text`.
- Repintado bajo demanda (`ctx.request_repaint_after(100ms)` para medidores; sin bucle a 60 fps).
- Cierre: `on_exit` → `stop()`; `Drop` del host como respaldo.

## Decisiones

### D1 — Frecuencia fija en la UI del slice
El slice expone un control de frecuencia (20–2000 Hz) y uno de ganancia. No hay teclado ni nota. El objetivo es verificar el camino, no tocar música.

### D2 — WAV propio, sin `hound`
Escribir un WAV `f32` PCM es ~80 líneas y evita una dependencia. Si en VS6 el export crece (metadatos, formatos), se reevalúa. Esto mantiene el árbol de dependencias pequeño, coherente con el presupuesto.

### D3 — Medición simple en el callback
RMS/peak por bloque en arrays atómicos; la FFT y el centroide espectral llegan en M3 (agente), fuera del callback.

### D4 — Sin `ArcSwap` todavía
El snapshot es de valores escalares; alcanza con atómicos. `ArcSwap` o triple buffer se añaden cuando exista estado estructurado (grafo, medidores múltiples).

### D5 — Fade de parada en el runtime
El fade de 20 ms se implementa como ramp de ganancia en el callback tras recibir `Stop`; el stream se cierra después de que el fade termina (bandera atómica), no inmediatamente.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| `rodio` en lugar de `cpal` | No da control del callback ni del buffer; imposible garantizar el contrato RT |
| `std::sync::mpsc` para comandos | Puede bloquear/asignar; descartado |
| Escribir el WAV con `hound` | Dependencia innecesaria para el alcance actual |
| Medir RMS en la UI leyendo el buffer de audio | Carrera de datos y lecturas inconsistentes |
| Un hilo de audio propio en vez del callback de cpal | Duplica el problema de sincronización y latencia |

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Allocaciones ocultas en `cpal` o en el logging | Spikes S1/S2; sin `log` en audio; allocator instrumentado en test |
| `BufferSize::Default` enorme | Fijar `BufferSize::Fixed(256)`; documentar fallback |
| Glitches por PipeWire con sample rate distinto | Coincidir con el clock del servidor; probar ALSA y PipeWire |
| Click al parar | Fade + cierre diferido del stream |
| La UI bloquea el hilo de audio | Nunca se llama a la UI desde el callback; solo snapshots atómicos |
| `eframe` consume CPU | Repintado bajo demanda; medir en S6 |

## Validación

```bash
cargo test -p nacre-dsp        # frecuencia, fase, ganancia
cargo test -p nacre-audio      # config, cola, snapshot, render offline
cargo run -p nacre-standalone  # comprobación manual: Play/Stop/ganancia
cargo clippy --workspace --all-targets -- -D warnings
```

Mediciones que deben quedar registradas en `HANDOFF.md`:

- xruns en 30 min (objetivo 0).
- Latencia del buffer y tasa efectiva.
- Hash de dos renders offline idénticos.
- Asignaciones en callback (objetivo 0).
- CPU media con la ventana visible.
