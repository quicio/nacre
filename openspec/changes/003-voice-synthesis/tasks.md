# Tasks — voice synthesis

> Prerrequisito: change 002 archivado. Ejecutar el spike S4 antes de fijar la estrategia de anti-aliasing.

## 1. Oscilador saw

- [x] 1.1 Implementar `SawOscillator` con fase `f64` y corrección PolyBLEP.
  - Verificación: test de frecuencia < 1 cent a 110/440/1760 Hz.
- [x] 1.2 Test espectral de alias: FFT de 1 s a 48 kHz, componentes > 20 kHz y alias in-band.
  - Verificación: alias < −60 dBFS respecto a la fundamental; registrar el gráfico/valores en `HANDOFF.md`.
- [x] 1.3 Integrar cambio de frecuencia sin reinicio de fase y sin discontinuidades.
  - Verificación: discontinuidad máxima < 0.05 en un salto de 110 → 440 Hz.

## 2. Envolvente ADSR

- [x] 2.1 Implementar `Adsr` con estados Idle/Attack/Decay/Sustain/Release y tiempos en ms.
  - Verificación: tests de duración y nivel para cada estado.
- [x] 2.2 Implementar retrigger desde el valor actual y liberación sin clicks.
  - Verificación: test de retrigger en release; discontinuidad < 0.01.
- [x] 2.3 Test de sustain estable a 0.6 durante 10 s.
  - Verificación: amplitud 0.6 ± 0.01 al final.

## 3. Filtro

- [x] 3.1 Implementar SVF TPT con estado `f64` y coeficientes recalculados.
  - Verificación: respuesta en frecuencia medida a 100 Hz/1 kHz/10 kHz dentro de ±1 dB del modelo teórico.
- [x] 3.2 Test de estabilidad: barrido 20 Hz–20 kHz con Q 0.5/5/20.
  - Verificación: salida finita, sin NaN/Inf, sin picos > 0 dBFS tras la etapa de ganancia.
- [x] 3.3 Modulación de cutoff por LFO con interpolación por bloque.
  - Verificación: sin clicks ni escalones audibles a 0.5 Hz y 20 Hz.

## 4. LFO y saturación

- [x] 4.1 Implementar `Lfo` (seno, triángulo) con fase independiente y rango 0.01–20 Hz.
  - Verificación: frecuencia medida dentro de ±0.1 %; salida en [−1, 1].
- [x] 4.2 Implementar saturación `tanh` normalizada con drive 0–24 dB.
  - Verificación: salida acotada, sin DC, armónicos medibles; test de alias con drive 24 dB.
- [x] 4.3 Implementar DC blocker y saneado de señal.
  - Verificación: offset < 0.001 tras 10 s; NaN/Inf no se propagan.

## 5. Voz y runtime

- [x] 5.1 Componer `Voice` (oscilador → filtro → VCA → saturación → DC block) con `note_on/note_off`.
  - Verificación: test de nota completa (ataque, sustain, release) sin discontinuidades.
- [x] 5.2 Sustituir el seno del callback por la voz y añadir los comandos de parámetros de voz.
  - Verificación: la app suena; los parámetros responden; sin allocaciones en el callback.
- [x] 5.3 Añadir gate/teclado mínimo en la UI y controles de voz.
  - Verificación: pruebas manuales de nota y de cada control.

## 6. Banco de escucha y cierre

- [x] 6.1 Crear tres patches de referencia: dron evolutivo, bajo pulsante, textura metálica.
  - Verificación: cada uno renderiza ≥ 2 min y suena sin artefactos.
- [x] 6.2 Guardar los renders en `assets/reference/` (o documentar cómo regenerarlos si el tamaño es excesivo).
  - Verificación: reproducibles con `cargo run --example render_ref -- <patch>`.
- [ ] 6.3 Escucha crítica con un tercero; registrar descripción de identidad y problemas.
  - Verificación: notas escritas en `HANDOFF.md`.
- [x] 6.4 Ejecutar tests, clippy, fmt, deny y actualizar `HANDOFF.md`.
  - Verificación: todo en verde.

## Fuera de alcance

- Polifonía, FM, resonadores, delay, reverb, patch editable, persistencia, MIDI, agente, plugin.
