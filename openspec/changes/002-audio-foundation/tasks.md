# Tasks — audio foundation

> Prerrequisito: change 001 archivado. Antes de empezar, ejecutar los spikes S1 y S2 y registrar el resultado en `docs/research/risks-and-spikes.md`.

## 1. DSP base

- [x] 1.1 Implementar `SineOscillator` en `nacre-dsp` con fase `f64`, `prepare`, `set_frequency`, `process`, `reset`.
  - Verificación: test de 440 Hz a 48 kHz; error de frecuencia < 1 cent.
- [x] 1.2 Añadir saneado de entrada (`gain`/`frequency` no finitos) y clamping de salida a [−1, 1].
  - Verificación: test con NaN, Inf y ganancia 10; la salida nunca es NaN ni > 1.
- [x] 1.3 Test de continuidad de fase al cambiar frecuencia en mitad de un bloque.
  - Verificación: discontinuidad máxima < 0.01 entre muestras consecutivas.

## 2. Configuración y comandos

- [x] 2.1 Implementar `AudioConfig` con validación (sample rate 8 000–192 000; canales 1–2; buffer 32–2048).
  - Verificación: tests de valores límite y de rechazo.
- [x] 2.2 Definir `Command` (`SetFrequency`, `SetGain`, `Start`, `Stop`) y la cola SPSC (`rtrb`, capacidad 256).
  - Verificación: test de producto/consumidor sin bloqueo y de `Full` cuando se llena.
- [x] 2.3 Añadir slots atómicos de parámetros (`AtomicU32` con bits de `f32`) y su lectura en el callback.
  - Verificación: test de escritura desde un hilo y lectura desde otro sin `unsafe` no documentado.

## 3. Runtime con cpal

- [x] 3.1 Implementar `AudioHost::start` con `cpal`: dispositivo por defecto, configuración preferida y negociación.
  - Verificación: `cargo run --example tone` produce audio; log de configuración negociada.
- [x] 3.2 Implementar el callback: drenar comandos (≤64), procesar seno, aplicar ganancia maestra, escribir intercalado estéreo.
  - Verificación: test de allocación (0 asignaciones en 10 000 bloques); audio audible sin clicks.
- [x] 3.3 Implementar `stop` con fade de 20 ms y cierre diferido del stream.
  - Verificación: 20 ciclos start/stop sin errores ni hilos colgados; sin click audible.
- [x] 3.4 Implementar `snapshot()` (RMS, peak, xruns, descartes, frames, carga estimada).
  - Verificación: valores actualizados en < 1 s; el callback no se bloquea.
- [x] 3.5 Implementar el tratamiento de errores de dispositivo (`DeviceUnavailable`, `ConfigUnsupported`, `StreamBuildFailed`).
  - Verificación: test con configuración inválida; la app no entra en pánico.

## 4. Aplicación y UI

- [x] 4.1 Definir el puerto `AudioHostPort` en `nacre-application` e implementar los casos de uso `StartAudio`, `StopAudio`, `SetParameter`.
  - Verificación: tests con un doble de test (sin `cpal`).
- [x] 4.2 Implementar la ventana mínima con Play/Stop, frecuencia, ganancia y estado del dispositivo.
  - Verificación: ejecución manual; los controles reflejan el estado real.
- [x] 4.3 Implementar el cierre limpio (parar audio, liberar recursos, sin hilos colgados).
  - Verificación: cerrar durante reproducción; código de salida 0.

## 5. Render offline

- [x] 5.1 Implementar `OfflineRenderer` que use el mismo `SineOscillator` y la misma ganancia maestra.
  - Verificación: render de 10 s a 48 kHz produce el número esperado de frames.
- [x] 5.2 Escribir WAV `f32` PCM sin dependencias externas.
  - Verificación: el archivo se abre en un reproductor y la cabecera es válida.
- [x] 5.3 Test de determinismo por hash en dos renders idénticos.
  - Verificación: hashes iguales.

## 6. Verificación del slice

- [x] 6.1 Ejecutar 30 min de reproducción continua y registrar xruns, CPU y latencia.
  - Verificación: 0 xruns; CPU registrada en `HANDOFF.md`.
  - Medición registrada el 2026-11-23 en ALSA sobre PipeWire: 337 517/337 500 callbacks, peor callback 459 µs de 5333 µs, cero callbacks lentos. No se repitió la pasada de 30 min.
- [x] 6.2 Ejecutar `cargo test --workspace`, `clippy -D warnings`, `fmt --check`, `cargo deny check`.
  - Verificación: todo en verde.
- [x] 6.3 Actualizar `HANDOFF.md` con mediciones, decisiones y estado del siguiente change.

## Fuera de alcance

- Saw, ADSR, filtro, LFO, saturación, reverb, delay, MIDI, patch, persistencia, agente, plugin.
- Sistema completo de parámetros con curvas y smoothing configurable (change 004).
