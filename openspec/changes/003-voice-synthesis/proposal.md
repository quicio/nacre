# Proposal — voice synthesis

## Por qué

Un seno demuestra que el audio funciona, no que el instrumento valga la pena. La identidad sonora de Nacre —drones, texturas metálicas, bajos orgánicos— aparece cuando hay armónicos, movimiento y carácter. Este change construye la primera voz completa y, con ella, el banco de escucha que decide si el proyecto merece seguir.

## Qué cambia

- Se añade un oscilador saw con anti-aliasing (PolyBLEP; oversampling solo si la medición lo exige).
- Se añade una envolvente ADSR con tiempos y curvas configurables y retrigger sin clicks.
- Se añade un filtro paso-bajo resonante de topología TPT/SVF, estable en todo el rango.
- Se añade un LFO (seno y triángulo) que modula cutoff y pitch con profundidad acotada.
- Se añade saturación suave (tanh o soft clip polinómico) con compensación de ganancia.
- Se añade DC blocker y gestión de headroom.
- La app añade un control de gate/nota y los parámetros de la voz.
- Se producen tres renders de referencia versionados (dron, bajo, metal).

## Capacidades

### Nuevas capacidades

- `voice-synthesis`: cadena de voz monofónica con oscilador anti-aliased, envolvente, filtro resonante, LFO, saturación y protecciones de señal.

### Capacidades modificadas

Ninguna. `oscillators` sigue existiendo; la voz la usa como base y añade la forma saw en una capacidad nueva para no reescribir requisitos del seno.

## Impacto

- `crates/nacre-dsp`: nuevos módulos `saw`, `adsr`, `svf`, `lfo`, `saturation`, `dc_block`, `voice`.
- `crates/nacre-audio`: el procesador del callback pasa de seno a voz; nuevos comandos de nota y parámetros.
- `apps/nacre-standalone`: controles de voz y gate.
- `assets/reference/`: tres WAV de referencia (no se versionan si pesan demasiado; ver design).
- Riesgo principal: aliasing audible y filtro inestable. Mitigación: spike S4 antes de cerrar el slice y tests espectrales.

## No-alcance

- Polifonía, unison, FM, AM, ring modulation, resonadores, delay, reverb (VS5 y M4).
- Sistema de parámetros completo (change 004): aquí los parámetros de la voz usan la transferencia mínima ya existente.
- Patch modular editable, wavetables, granular, modelado físico.
- Presets de usuario y persistencia.
