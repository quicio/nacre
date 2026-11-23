# Riesgos y spikes

> Ordenados por impacto técnico × probabilidad. Un riesgo sin spike asociado es una opinión; un spike sin criterio de salida es una excusa.

## Cómo se usa

- Cada spike tiene **timebox** y **criterio de salida**. Si no se cumple, se documenta el resultado y se cambia la decisión, no se alarga indefinidamente.
- Los spikes viven en ramas `spike/*` o en `experiments/`, **no** en el workspace principal. No se mergean sin convertirlos en código de producción con tests.
- El resultado se escribe en el ADR correspondiente (estado `Accepted`/`Rejected`) y se resume en `HANDOFF.md`.

## Ranking

| # | Riesgo | Prob. | Impacto | Spike | Estado |
|---|---|---|---|---|---|
| R1 | El callback de audio no es realmente libre de alloc/locks/bloqueos | Alta | Crítico | S1, S2 | S2 medido; S1 duración en ALSA, contrato en PipeWire nativo |
| R2 | El timing generativo suena mal por jitter o cuantización | Media | Crítico | S7 | Pendiente (M2) |
| R3 | Reemplazo de grafo con clicks o picos de CPU | Alta | Alto | S3 | Pendiente |
| R4 | El DSP suena genérico o con aliasing audible | Media | Crítico | S4 | Pendiente |
| R5 | nih-plug no cubre VST3/CLAP o la licencia GPLv3 bloquea el negocio | Media | Alto | S5 | Pendiente (M4) |
| R6 | La UI de egui consume CPU o falla dentro del plugin | Media | Medio | S6, S5 | Pendiente |
| R7 | La evolución generativa destruye la identidad musical | Alta | Crítico | S7 (musical) | Pendiente (M2) |
| R8 | El modelo transaccional del agente deja estados inconsistentes | Media | Alto | — (tests de propiedades) | Pendiente (M3) |
| R9 | El formato de patch se vuelve incompatible y rompe presets | Media | Medio | — (ADR de versionado) | Pendiente (VS6) |
| R10 | El proyecto se sobrearquitectura y nunca suena | Media | Crítico | — (disciplina de scope) | Vigilado |
| R11 | Latencia/CPU insuficiente para polifonía y reverb de calidad | Media | Medio | Benchmarks VS5 | Pendiente |
| R12 | Multiplataforma (macOS/Windows) revela problemas tardíos | Media | Medio | CI extendida | Pendiente |

## Detalle de spikes

### S1 — Comportamiento de CPAL en tiempo real

**Pregunta:** ¿qué backend y configuración dan 30 min sin xruns en el equipo objetivo?

**Método:**
1. Stream de salida con `cpal` 0.18: ALSA, PipeWire nativo (feature `pipewire`) y JACK.
2. `BufferSize::Fixed(256)`, 48 kHz, estéreo, `f32`.
3. Contador de xruns/callbacks, latencia medida (entrada de impulso → salida), CPU por bloque.
4. Reproducción de 30 min con UI activa y sin ella.

**Criterio de salida:** 0 xruns en 30 min por backend; latencia documentada; diferencia ALSA vs PipeWire cuantificada. **Timebox:** 3 días.

**Desbloquea:** ADR-0002 (backend definitivo), VS2.

**Medido el 2026-11-23.** Con la feature `pipewire`, `cpal` 0.18.2 expone los hosts `PipeWire` y `Alsa`. El nativo anuncia f32 estéreo a 48 kHz fijo y buffers de 32 a 2048; el ALSA de PipeWire anuncia 1–384 000 Hz y buffers de 16 a 262 144. Ambos aceptan `Fixed(256)`. En 45 s el nativo dio 8434/8438 callbacks (máximo 50 µs) y ALSA 8439/8438 (máximo 42 µs), cero lentos en los dos. El gate de 30 min está medido solo en ALSA: 337 517/337 500, peor callback 459 µs de 5333 µs. No hay `jackd`. Xruns del kernel y latencia impulso→salida siguen sin medir. Detalle en ADR-0002.

### S2 — Transferencia de parámetros y comandos sin locks

**Pregunta:** ¿el camino UI → audio cumple el contrato de tiempo real?

**Método:**
1. Ring buffer SPSC (`rtrb`) para comandos; slots atómicos para valores actuales.
2. Allocator instrumentado en el callback; contador de asignaciones y de bloqueos.
3. Ráfaga de 10 000 cambios de parámetro/s desde un hilo de control.
4. Medir latencia de aplicación, descartes por cola llena y clicks.

**Criterio de salida:** 0 asignaciones y 0 bloqueos en el callback; cambio aplicado en ≤ 2 bloques; sin clicks audibles. **Timebox:** 2 días.

**Desbloquea:** VS2, VS4.

**Medido el 2026-11-23: pasa el contrato de carga.** 100 000 comandos en 10 s (10 000/s) por un ring SPSC de `rtrb` 0.4.1 con 256 slots, publicados en 64 atómicos. Cero rechazos, profundidad máxima 60, latencia máxima 5,94 ms y p99 5,36 ms, contra un bloque de 5,33 ms. El consumidor solo hace `pop` y stores atómicos. Los «cero alloc / cero locks» son por construcción del probe, no por un allocator instrumentado; eso queda como test del change 002, donde el callback es real.

### S3 — Reemplazo de grafo y crossfade

**Pregunta:** ¿se puede cambiar de patch sin clicks ni xruns?

**Método:**
1. Dos `PreparedGraph` (seno → saw filtrado), crossfade equal-power 20 ms.
2. 100 cambios durante reproducción; análisis de discontinuidad muestra a muestra.
3. Medición de pico de CPU y de allocaciones.

**Criterio de salida:** ninguna discontinuidad > 0.01; sin clicks audibles; pico de CPU < 2× el estable; sin xruns. **Timebox:** 3 días.

**Desbloquea:** ADR-0005, VS5/VS6.

### S4 — Calidad de osciladores y filtros

**Pregunta:** ¿PolyBLEP basta o hace falta oversampling? ¿El filtro es estable y musical?

**Método:**
1. Saw naive vs PolyBLEP vs oversampling 2×/4× a 110/440/1760 Hz.
2. Análisis FFT de componentes aliaseadas; escucha A/B.
3. Barrido de cutoff 20 Hz–20 kHz con Q 0.5–20; comprobar NaN/Inf, ganancia y estabilidad.
4. Comparación con `fundsp` como referencia (solo test).

**Criterio de salida:** alias < −60 dBFS en el rango audible para el oscilador elegido; filtro estable en todo el barrido; decisión documentada. **Timebox:** 4 días.

**Desbloquea:** ADR-0004, VS3.

### S5 — Viabilidad del plugin (CLAP/VST3, egui, transporte)

**Pregunta:** ¿puede nih-plug entregar CLAP y VST3 con editor egui y transporte, sin sorpresas legales o de host?

**Método:**
1. Plugin mínimo (ganancia + seno) exportado como CLAP y VST3.
2. Carga en dos hosts (Reaper, Bitwig) en Linux: audio, automatización, estado, transporte.
3. Editor `nih_plug_egui`: foco, repintado, redimensionado.
4. Revisión de licencias: ISC + GPLv3 de `vst3-sys` vs licencia del producto.
5. Repetición en macOS si hay hardware disponible.

**Criterio de salida:** ambos formatos funcionan en al menos dos hosts; limitaciones documentadas; decisión de licencia tomada. **Timebox:** 1 semana.

**Desbloquea:** ADR-0003, M4.

### S6 — Coste y comportamiento de la UI

**Pregunta:** ¿egui es viable como UI de un instrumento en tiempo real?

**Método:**
1. Ventana con 50 controles y medidores a 60 fps; medir CPU y latencia de interacción.
2. Repintado bajo demanda vs continuo.
3. Interacción mientras el audio está activo (sin xruns).

**Criterio de salida:** UI < 3 % de un núcleo con ventana visible; sin xruns al interactuar; latencia de control percibida < 50 ms. **Timebox:** 2 días.

**Desbloquea:** ADR-0008, VS2.

### S7 — Identidad generativa

**Pregunta:** ¿la evolución produce música reconocible o ruido estadístico?

**Método:**
1. Implementar un prototipo mínimo de Euclidean + drift + evolución meso (fuera del workspace, script o crate de experimento).
2. Generar 5 piezas de 5 min con semillas distintas.
3. Evaluación por escucha ciega: ¿se reconoce el motivo? ¿cambia sin romperse?
4. Métricas: histograma de intervalos, densidad por compás, distancia al motivo original.

**Criterio de salida:** 4/5 piezas mantienen un motivo identificable a los 5 min; el cambio es perceptible sin intervención. **Timebox:** 1 semana.

**Desbloquea:** ADR-0006, M2.

## Riesgos sin spike (vigilancia)

- **R10 Sobrearquitectura:** se controla con el presupuesto de 5 crates en VS1 y con la regla "nada sin segundo uso".
- **R9 Formato de patch:** se controla con versión de formato explícita y tests de migración desde VS6.
- **R12 Multiplataforma:** se controla añadiendo macOS/Windows a CI en cuanto el runtime de audio esté estable, no al final.

## Riesgos de producto (no técnicos)

| Riesgo | Señal temprana | Respuesta |
|---|---|---|
| El instrumento suena a "demo de sintetizador" | Los renders no se escuchan dos veces | Invertir en DSP y en el diseño de voz antes de añadir features |
| El agente es un gadget | Nadie usa el modo Director | Rediseñar la semántica musical, no el prompt |
| La UI experta intimida | El usuario no pasa del nivel 1 | Simplificar el nivel 1; ocultar el grafo por defecto |
| El proyecto se estanca en planificación | Semanas sin un render | Gate M0 con fecha límite y primer slice corto |
