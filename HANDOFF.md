# HANDOFF — estado de implementación

> Documento de continuidad. Se actualiza al final de cada sesión. Un agente nuevo debe leer esto primero, después `AGENTS.md`.

## Última actualización

**2026-11-23** — Change `002-audio-foundation` implementado y verificado. Primer audio real: dispositivo, callback, seno, comandos, render offline y ventana mínima. Pendiente de revisión humana y de `openspec archive`.

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

- `nacre-dsp`: `SineOscillator` con fase `f64`, precisión < 1 cent a 440 Hz y 20 Hz, saneado de NaN/Inf, silencio exacto con ganancia 0 y fase continua al cambiar frecuencia.
- `nacre-application`: `AudioConfig` validada (8 000–192 000 Hz, 1–2 canales, 32–2048 frames), `AudioHostPort`, comandos, errores estructurados y casos de uso `StartAudio`, `StopAudio`, `SetParameter`.
- `nacre-audio-runtime`: host `cpal` con preferencia por el host nativo PipeWire en Linux y respaldo ALSA; cola SPSC `rtrb` de 256 comandos; drenaje máximo de 64 por bloque; slots atómicos; medidores RMS/peak; contadores de frames, xruns y descartes; fade de parada de 20 ms con cierre diferido; render offline determinista y escritor WAV `f32` propio.
- `apps/nacre-standalone`: ventana `eframe` de 480×240 con Play/Stop, frecuencia 20–2 000 Hz, ganancia maestra −60–0 dB y estado del dispositivo/medidores. Cierre limpio mediante `on_exit` y `Drop`.
- Ejemplos: `tone` (2 s y termina solo) y `render_wav` (10 s a WAV, sin dispositivo).

## Mediciones de esta sesión

| Qué | Resultado |
|---|---|
| Host y configuración | Host nativo **PipeWire**, dispositivo `default_output`, f32 estéreo, **48 000 Hz**, `BufferSize::Fixed(256)`, `negotiated = false` |
| Callback sin asignaciones | **0 asignaciones** en 10 000 bloques con allocator instrumentado y comandos de frecuencia cambiantes por bloque |
| Fade de parada | 20 ms; el stream se cierra después de que el fade termina. Test de fade y test de 20 ciclos start/stop |
| Ciclos start/stop | 20 ciclos sin errores ni estado colgado; test completo en 0,45 s |
| Render offline | 10 s a 48 kHz = 480 000 frames / 960 000 muestras; hash FNV-1a `1dcf94810ad738a9` |
| Determinismo WAV | Dos ejecuciones del ejemplo producen el mismo SHA-256: `983d5c6ab5b19455d4db31eb600479289e2236c3de34a89c109304005227b856` |
| WAV validado externamente | `ffprobe`: `pcm_f32le`, 48 000 Hz, 2 canales, duración 10,000000 s |
| CPU con ventana visible | ~12 % de un núcleo en **build debug** (3,2 s de ventana+audio: 0,321 s user + 0,067 s sys). Incluye arranque; no es una medida de release |
| Gate de 30 min (medición previa, no repetida) | ALSA sobre PipeWire: **337 517/337 500 callbacks**, peor callback **459 µs de 5333 µs**, cero callbacks lentos |

No se reprodujo audio durante 30 minutos: el gate de 30 min se documenta con la medición del ADR-0002. Los xruns del kernel siguen sin medirse (cpal no expone `snd_pcm_status`); el error callback no se disparó.

## Qué NO existe todavía

- Saw, ADSR, filtro, LFO, saturación, reverb, delay, polifonía (change 003).
- Sistema completo de parámetros con curvas y smoothing configurable (change 004). Aquí solo frecuencia y ganancia con smoothing mínimo.
- Grafo modular, MIDI, persistencia de patch, agente, plugin (VST3/CLAP).
- Selección de dispositivo en la UI; se usa el dispositivo por defecto y se informa del elegido.
- Escucha humana formal de la ventana y del tono. La ventana compila y arranca, pero la validación auditiva queda para una persona.

## Change activo

**`002-audio-foundation`** — implementado (20/20 tareas), gates en verde, pendiente de revisión humana y de `openspec archive`. No archivar sin esa revisión.

El change `001-workspace-foundation` está archivado (`openspec/changes/archive/2026-11-23-001-workspace-foundation`).

## Estado de los changes

| Change | Artefactos | Implementado | Notas |
|---|---|---|---|
| `001-workspace-foundation` | proposal, design, tasks | Sí (2026-11-23) | Archivado |
| `002-audio-foundation` | proposal, specs, design, tasks | Sí (2026-11-23) | 20/20 tareas. Gates reejecutados en la revisión: fmt, clippy, 40 tests, `check-deps`, `cargo deny`. Pendiente de archivar |
| `003-voice-synthesis` | proposal, specs, design, tasks | No | Bloqueado por 002 |
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

## Decisiones abiertas (no inventar certeza)

1. **Licencia del proyecto.** Sin decidir. `nih-plug` es ISC, `cpal` Apache-2.0, `fundsp` MIT/Apache-2.0: compatibles entre sí, pero la licencia del producto (¿MIT? ¿Apache? ¿propietaria?) debe decidirse antes de publicar.
2. **Backend de audio en Linux.** PipeWire nativo es la preferencia y funciona en esta máquina; el gate de 30 min está medido en ALSA, no en el nativo (la pasada nativa fue de 45 s). JACK no medido.
3. **PolyBLEP vs oversampling** para saw. Spike S4.
4. **Formato de patch**: JSON legible vs binario compacto vs ambos con versión. Ver ADR-0008.
5. **Alcance del plugin**: VST3 primero, CLAP después (o simultáneos). Depende del spike S5.
6. **Nombre definitivo del producto** ("Nacre" es el nombre de trabajo). Ya está en los cinco crates y en el binario; cambiarlo ahora es un refactor del workspace completo.
7. **Medición de xruns del kernel y latencia impulso→salida.** Siguen sin medir; no sustituirlas por el contador de callbacks de cpal.

## Cómo continuar

```bash
cd /home/hugo/projects/nacre
openspec list --specs          # capacidades construidas
openspec show 002-audio-foundation
openspec validate --all --strict
cargo run -p nacre-audio-runtime --example tone
cargo run -p nacre-standalone
```

Si el change activo está aprobado y archivado, el siguiente es `003-voice-synthesis`. Si no, pregunta antes de escribir código.
