# HANDOFF — estado de implementación

> Documento de continuidad. Se actualiza al final de cada sesión. Un agente nuevo debe leer esto primero, después `AGENTS.md`.

## Última actualización

**2026-11-23** — Change `003-voice-synthesis` implementado: voz monofónica con saw 4× + PolyBLEP, ADSR, SVF, LFO, saturación y DC blocker; runtime y UI conectados; tres renders de referencia de 2 min. Queda sin marcar la escucha crítica con un tercero (tarea 6.3) y la revisión humana antes de archivar.

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

### Audio foundation (2026-11-23)

> Base del change 002, conservada como histórico. Desde el 003 el callback usa la voz, no el seno; el render WAV de 002 sigue disponible como preview.

- `nacre-dsp`: `SineOscillator` con fase `f64`, precisión < 1 cent a 440 Hz y 20 Hz, saneado de NaN/Inf, silencio exacto con ganancia 0 y fase continua al cambiar frecuencia.
- `nacre-application`: `AudioConfig` validada (8 000–192 000 Hz, 1–2 canales, 32–2048 frames), `AudioHostPort`, comandos, errores estructurados y casos de uso `StartAudio`, `StopAudio`, `SetParameter`.
- `nacre-audio-runtime`: host `cpal` con preferencia por el host nativo PipeWire en Linux y respaldo ALSA; cola SPSC `rtrb` de 256 comandos; drenaje máximo de 64 por bloque; slots atómicos; medidores RMS/peak; contadores de frames, xruns y descartes; fade de parada de 20 ms con cierre diferido; render offline determinista y escritor WAV `f32` propio.
- `apps/nacre-standalone`: ventana `eframe` de 480×240 con Play/Stop, frecuencia 20–2 000 Hz, ganancia maestra −60–0 dB y estado del dispositivo/medidores. Cierre limpio mediante `on_exit` y `Drop`.
- Ejemplos: `tone` (2 s y termina solo) y `render_wav` (10 s a WAV, sin dispositivo).

### Voz y síntesis (2026-11-23)

- `nacre-dsp`: `SawOscillator` (PolyBLEP solo en el cruce, sobremuestreo 4× con decimación FIR y buffers reservados en `prepare`), `Adsr`, `Svf` (TPT/SVF con estado `f64`), `Lfo` (seno/triángulo), `Saturation` (tanh normalizada), `DcBlocker` y `Voice` (oscilador → filtro → VCA → saturación → DC block, con LFO al cutoff o al pitch).
- `nacre-application`: comandos de nota (`NoteOn`/`NoteOff`), forma de onda, forma y destino del LFO y parámetros continuos de voz con rangos validados.
- `nacre-audio-runtime`: el callback usa la voz en lugar del seno; la cola SPSC transporta los nuevos comandos; los slots atómicos guardan los parámetros de voz también con el stream parado; render offline de referencia determinista en `reference.rs`.
- `apps/nacre-standalone`: ventana 560×720 con Play/Stop, gate, teclado de una octava (A2–A3), forma de onda, cutoff/Q, A/D/S/R, LFO (rate/depth/destino/forma), drive y ganancia maestra.
- `assets/reference/`: `drone.wav`, `bass.wav`, `metal.wav` (120 s, 48 kHz, estéreo, `f32`) y un README con parámetros, hashes y comandos de regeneración. Los WAV no se versionan (`*.wav` está en `.gitignore`).
- Ejemplo nuevo: `render_ref` (renderiza uno o los tres parches y termina solo, sin dispositivo).

## Mediciones de esta sesión

### Anti-aliasing del saw (S4)

Método: 1 s a 48 000 Hz, ventana rectangular, frecuencia en un bin (110/440/1760 Hz).
Alias = pico más alto que no es armónico, por debajo de 24 kHz, relativo a la
fundamental. El `SawOscillator` genera a 4× con PolyBLEP **solo en el cruce de
fase** y decima con un FIR Kaiser de 320 taps (passband 0,42·fs, stopband
0,5·fs); el resto de la voz corre a la tasa del dispositivo.

| Frecuencia | Alias medido | Bin del pico | S4 ideal 4× (referencia) |
|---|---|---|---|
| 110 Hz | **−100,8 dB** | 20 950 Hz | −98,0 dB |
| 440 Hz | **−88,8 dB** | 20 840 Hz | −86,0 dB |
| 1760 Hz | **−77,0 dB** | 21 280 Hz | −74,7 dB |

Criterio: < −60 dB. Se cumple en las tres frecuencias, con 17 dB de margen en el
peor caso. La decimación es un lowpass real: respuesta del FIR 8,7e−6 en el
Nyquist del dispositivo (0,125 del rate 4×), 1,0e−5 a 26,6 kHz y 2,2e−15 en el
Nyquist de 4×. La salida del saw se escala a 0,8 para que el overshoot de Gibbs
del filtro (±1,16) no se recorte: recortarlo introducía alias de −59,2 dB.

### Resto de la voz

| Qué | Resultado |
|---|---|
| Precisión del saw | < 1 cent a 110/440/1760 Hz |
| Cambio 110 → 440 Hz | fase sin reinicio; paso de muestra < 0,05 lejos del cruce |
| Respuesta del filtro (fc 1 kHz, Q 0,707) | 100 Hz: 0,000 dB (modelo 0,000); 1 kHz: −3,010 (modelo −3,010); 10 kHz: −42,738 (modelo −42,738) |
| Estabilidad del filtro | barrido 20 Hz–20 kHz, Q 0,5/5/20: salida finita, sin NaN/Inf |
| ADSR | ataque 10 ms llega a 1; decay 100 ms a 0,60; sustain 0,6 ± 0,01 durante 10 s; release y retrigger desde el nivel actual |
| LFO | 0,5/5/20 Hz dentro de ±0,1 %; seno y triángulo; salida en [−1, 1] |
| Saturación | drive 0–24 dB; salida acotada; sin DC; 24 dB sobre seno de amplitud 0,5 a 440 Hz: alias **−107,9 dB** |
| DC blocker | saw asimétrica 10 s: offset medio < 0,001; NaN/Inf no se propagan |
| Voz completa | nota con ataque/sustain/release sin discontinuidades (test con seno); Q 20 no supera el límite de salida |
| Callback sin asignaciones | **0 asignaciones** en 10 000 bloques con nota, frecuencia y cutoff cambiando, allocator instrumentado |
| CPU con la ventana y la voz | ~30 % de un núcleo en **build debug** (medidor de salud del callback: `CPU 30 %`, RMS −32,4 dB, peak −27,0 dB con master −12 dB y gate activo) |

### Renders de referencia

Los tres parches renderizan 120 s (5 760 000 frames) con el ejemplo offline
`render_ref`, en 81 s de build debug. `ffprobe` valida `pcm_f32le`, 48 000 Hz,
2 canales y 120,000000 s en los tres. Dos ejecuciones producen el mismo hash
FNV-1a; no hay azar ni reloj.

| Parche | Pico | RMS | DC | Muestras a fondo de escala | Hash FNV-1a |
|---|---|---|---|---|---|
| `drone` | −5,59 dBFS | −13,84 dBFS | 2,30e−12 | 0 | `8f856bb8507da919` |
| `bass` | −6,03 dBFS | −24,29 dBFS | 5,80e−13 | 0 | `344e656bb6aae099` |
| `metal` | −7,90 dBFS | −11,56 dBFS | 1,41e−12 | 0 | `d324094d4c074635` |

### UI

La ventana 560×720 arranca en Wayland con `NACRE_AUTOSTART=1 NACRE_AUTOGATE=1`
y termina sola con código 0. La captura de esa ejecución muestra `reproduciendo`,
el gate activo, el teclado A2–A3, todos los controles de voz y el medidor de
salud con señal (`RMS −32,4 dB`, `Peak −27,0 dB`, `CPU 30 %`). La escucha
crítica sigue pendiente (6.3).

## Qué NO existe todavía

- Polifonía, unison, FM, resonadores, delay, reverb, wavetables, granular, modelado físico.
- Sistema completo de parámetros con curvas y suavizado configurable (change 004). Los parámetros de voz usan la transferencia mínima de 002.
- Grafo modular, MIDI, persistencia de patch, agente, plugin (VST3/CLAP).
- Selección de dispositivo en la UI; se usa el dispositivo por defecto y se informa del elegido.
- Escucha crítica de la identidad sonora con un tercero (tarea 6.3). Los renders existen; falta la persona.
- Medición de xruns del kernel y de latencia impulso→salida. Siguen sin medirse.

## Change activo

**`003-voice-synthesis`** — implementado el 2026-11-23; queda pendiente la tarea 6.3 (escucha con un tercero) y la revisión humana antes de archivar.

El change `002-audio-foundation` está archivado (`openspec/changes/archive/2026-11-23-002-audio-foundation`). Las capacidades construidas viven en `openspec/specs/`: `audio-runtime` (6), `oscillators` (4), `standalone-app` (4).

El change `001-workspace-foundation` está archivado (`openspec/changes/archive/2026-11-23-001-workspace-foundation`).

## Estado de los changes

| Change | Artefactos | Implementado | Notas |
|---|---|---|---|
| `001-workspace-foundation` | proposal, design, tasks | Sí (2026-11-23) | Archivado |
| `002-audio-foundation` | proposal, specs, design, tasks | Sí (2026-11-23) | Archivado. Specs en `openspec/specs/` |
| `003-voice-synthesis` | proposal, specs, design, tasks | Sí (2026-11-23) | 18/19 tareas; **6.3 sin marcar** (escucha humana). Pendiente de archivar |
| `004-parameter-system` | proposal, specs, design, tasks | No | Bloqueado por 003 |

## Decisiones y desviaciones registradas

1. **`AudioConfig` vive en `nacre-application`.** El runtime lo reexporta; no hay dos tipos. `AudioHostPort`, `AudioStatus`, `AudioError`, `CommandError` y `ParameterCommand` también viven en application para no invertir la dependencia.
2. **Los comandos son de aplicación.** `nacre-application::ParameterCommand` es el tipo canónico; `nacre-audio-runtime::Command` es un alias. Evita duplicar la secuencia y mantiene la cola SPSC en el runtime.
3. **Los casos de uso prestan el host (`&mut dyn AudioHostPort`).** El design mostraba `Box<dyn AudioHostPort>` dentro de cada caso de uso; prestar el puerto permite que la app use un único host para start/stop/parámetros.
4. **`AudioHost::probe`.** Añadido para cumplir el escenario "audio listo" antes de pulsar Play: negocia dispositivo y configuración sin abrir stream.
5. **Criterio literal de discontinuidad < 0.01.** A 880 Hz y 48 kHz la pendiente máxima de una seno es ~0,115 por muestra, así que el criterio literal es físicamente incompatible con esa frecuencia. El test comprueba continuidad de fase analítica en el cambio 220→880 Hz y el criterio literal en un cambio de baja frecuencia (20→21 Hz). No hay reinicio de fase.
6. **`deny.toml`.** Se permiten `Apache-2.0 WITH LLVM-exception` (build de PipeWire) y las licencias de fuentes `OFL-1.1` y `Ubuntu-font-1.0` que arrastra `eframe`/`epaint_default_fonts`. No cambian la licencia del producto.
7. **El host nativo PipeWire se elige explícitamente** en Linux si está disponible; si no, `cpal::default_host()` cae a ALSA.
8. **El DSP usa la configuración efectiva negociada** (sample rate, canales, buffer), no la solicitada. Así un dispositivo a 44,1 kHz mantiene la afinación correcta.
9. **PolyBLEP solo en el cruce de fase del saw.** El design mencionaba una segunda corrección a mitad de ciclo; esa corrección pertenece a una cuadrada y en una saw añade error. Manda ADR-0004/S4.
10. **4× solo en el oscilador, con decimación real.** FIR Kaiser de 320 taps (β = 9), passband 0,42·fs, stopband 0,5·fs, en anillo y buffer 4× reservados en `prepare`. No se tiran muestras. La salida del saw se escala a 0,8 para que el overshoot de Gibbs (±1,16) no se recorte: el clamp a ±1 introducía alias de −59,2 dB. El filtro, VCA, saturación y DC blocker corren a la tasa del dispositivo.
11. **Saturación `tanh` normalizada.** `drive = 10^(dB/20) − 1` (0 dB transparente), `y = tanh(drive·x) / drive`. `tanh` es analítica y sus armónicos decaen exponencialmente: a 24 dB el alias medido es −107,9 dB, así que esta etapa no se sobremuestrea.
12. **Profundidad del LFO.** 100 % = ±3 octavas en cutoff (multiplicativo) y ±12 semitonos en pitch. Un valor de LFO por bloque, interpolado dentro del bloque: la oscilación hace glide y el filtro interpola coeficientes.
13. **Cambio de forma de onda en frontera de bloque.** Se copia la fase entre fuentes para no reiniciarla; el salto de timbre es intencional, como decía el design. No hay crossfade en este slice.
14. **Headroom de la voz a −6 dBFS** (ganancia 0,5) y clamp final. La ganancia maestra sigue en el runtime.
15. **Los patches de referencia viven en `nacre-audio-runtime`** (`reference.rs`): `nacre-application` no puede depender de `nacre-dsp` (grafo de dependencias) y el ejemplo necesita la voz. `OfflineRenderer` de 002 se queda como preview de seno para `render_wav`; el camino offline de la voz es `render_reference`.
16. **La voz hace snap de frecuencia cuando la envolvente está idle**, para que la primera nota no arrastre un glide de 5 ms desde una frecuencia obsoleta. Durante una nota, los cambios de pitch siguen siendo glides de un bloque.
17. **El FFT de los tests de alias es propio y solo de test.** Implementación mixed-radix (2/3/5) en `nacre-dsp/src/test_fft.rs`; no añade dependencias de producción.

## Decisiones abiertas (no inventar certeza)

1. **Licencia del proyecto.** Sin decidir. `nih-plug` es ISC, `cpal` Apache-2.0, `fundsp` MIT/Apache-2.0: compatibles entre sí, pero la licencia del producto (¿MIT? ¿Apache? ¿propietaria?) debe decidirse antes de publicar.
2. **Backend de audio en Linux.** PipeWire nativo es la preferencia y funciona en esta máquina; el gate de 30 min está medido en ALSA, no en el nativo (la pasada nativa fue de 45 s). JACK no medido.
3. **Escucha crítica de la identidad sonora (tarea 6.3).** Los tres renders y la ventana existen y producen señal; falta la escucha con un tercero y la descripción de identidad/problemas.
4. **Formato de patch**: JSON legible vs binario compacto vs ambos con versión. Ver ADR-0008.
5. **Alcance del plugin**: VST3 primero, CLAP después (o simultáneos). Depende del spike S5.
6. **Nombre definitivo del producto** ("Nacre" es el nombre de trabajo). Ya está en los cinco crates y en el binario; cambiarlo ahora es un refactor del workspace completo.
7. **Medición de xruns del kernel y latencia impulso→salida.** Siguen sin medir; no sustituirlas por el contador de callbacks de cpal.

## Cómo continuar

```bash
cd /home/hugo/projects/nacre
openspec list --specs          # capacidades construidas
openspec show 003-voice-synthesis
openspec validate --all --strict
cargo run -p nacre-audio-runtime --example tone
cargo run -p nacre-audio-runtime --example render_ref -- all
cargo run -p nacre-standalone
```

El change activo (`003-voice-synthesis`) está implementado pero **no archivado**: falta la tarea 6.3 y la revisión humana. No empieces `004-parameter-system` hasta archivarlo.
