# Proposal — audio foundation

## Por qué

Es el primer vertical slice con sonido. Hasta que exista un callback que produzca audio estable, no se puede validar nada de lo demás: ni el DSP, ni los parámetros, ni el modelo de patch. El objetivo no es un sintetizador completo, sino demostrar que el camino dispositivo → callback → DSP → salida es correcto y que se puede testear sin oídos ni hardware.

## Qué cambia

- Se abre el dispositivo de salida con `cpal`, estéreo `f32`, preferencia 48 kHz y buffer fijo (256 frames), con negociación explícita si el dispositivo no lo soporta.
- Se implementa el contrato de tiempo real: sin allocación, sin locks, sin I/O y sin pánicos en el callback.
- Se añade un oscilador seno con fase continua y frecuencia/ganancia controlables mediante comandos SPSC.
- Se añade arranque/parada con fade de 20 ms y estado de salud (xruns, comandos descartados, carga de CPU).
- Se añade render offline determinista a WAV usando la misma ruta DSP.
- La app standalone mínima muestra Play/Stop, ganancia maestra y estado del dispositivo.

## Capacidades

### Nuevas capacidades

- `audio-runtime`: ciclo de vida del dispositivo, contrato de tiempo real, transferencia de comandos, reloj de frames, estado de salud y render offline determinista.
- `oscillators`: generación de seno con precisión de frecuencia, continuidad de fase y límites de salida.
- `standalone-app`: arranque de la aplicación, controles mínimos de transporte y ganancia, comportamiento sin dispositivo y cierre limpio.

### Capacidades modificadas

Ninguna. Es el primer change con comportamiento.

## Impacto

- `crates/nacre-audio`: `cpal`, `rtrb`, reloj, comando, `AudioHost` implementado.
- `crates/nacre-dsp`: oscilador seno y utilidades de bloque.
- `crates/nacre-application`: casos de uso `start_audio`, `stop_audio`, `set_parameter`, `render_offline`.
- `apps/nacre-standalone`: ventana `eframe` mínima y wiring.
- `Cargo.toml`: nuevas dependencias (`cpal`, `rtrb`, `eframe`, `hound` o escritor WAV propio).
- Riesgo principal: que `cpal` o el backend introduzcan allocaciones/bloqueos ocultos. Mitigación: spikes S1 y S2 antes de cerrar el slice.

## No-alcance

- Saw, ADSR, filtro, LFO, saturación (change 003).
- Sistema de parámetros completo con curvas y smoothing por parámetro (change 004); aquí solo frecuencia y ganancia con un smoothing mínimo.
- Grafo modular, múltiples voces, MIDI, persistencia de patch, agente, plugin.
- Selección de dispositivo en la UI (se usa el dispositivo por defecto y se informa del elegido).
