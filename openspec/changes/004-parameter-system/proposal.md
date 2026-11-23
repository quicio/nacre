# Proposal — parameter system

## Por qué

En los changes 002 y 003 los parámetros existen como casos especiales: un `f32` por aquí, un comando por allá, un `if` para el rango. Eso no escala al patch modular, a la automatización del host ni al agente. Antes de añadir efectos y persistencia necesitamos una única definición de qué es un parámetro, cómo se valida, cómo se normaliza, cómo se suaviza y cómo cruza la frontera de tiempo real sin romperla.

## Qué cambia

- Se introduce `ParameterId` estable y legible, `ParameterSpec` (unidad, rango, default, curva, suavizado) y `ParameterValue` en unidad.
- Se centraliza la validación: valores fuera de rango se rechazan con error estructurado, nunca se corrigen en silencio.
- Se añade normalización 0..1 con curvas linear/logarítmica/exponencial y su inversa, con ida y vuelta dentro de tolerancia.
- Se generaliza el suavizado por parámetro (one-pole, tiempo configurable) y el tratamiento de parámetros discretos.
- Se sustituye la ruta mínima de 002/003 por una única ruta de comandos con coalescing (último valor gana) y latencia acotada.
- Se añade un snapshot consistente para la UI y una tabla de parámetros consultable por la aplicación y, más adelante, por el agente.

## Capacidades

### Nuevas capacidades

- `parameter-system`: identidad, especificación, validación, normalización, suavizado, transferencia en tiempo real y snapshot de parámetros.

### Capacidades modificadas

Ninguna formalmente: `audio-runtime` y `voice-synthesis` conservan sus requisitos. La implementación de 002/003 se refactoriza para usar el sistema común sin cambiar el comportamiento observable que ya especifican.

## Impacto

- `crates/nacre-domain`: `ParameterId`, `ParameterSpec`, `ParameterValue`, `ParameterError`, curvas.
- `crates/nacre-application`: tabla de parámetros, casos de uso `set_parameter`, `list_parameters`, `get_snapshot`.
- `crates/nacre-dsp`: `Smoother` genérico, parámetros de voz migrados.
- `crates/nacre-audio-runtime`: ruta única de comandos, slots atómicos con coalescing, snapshot.
- `apps/nacre-standalone`: controles generados desde la tabla de parámetros en lugar de campos ad-hoc.
- Riesgo principal: que la abstracción añada coste o complejidad innecesaria en el callback. Mitigación: la ruta de audio sigue siendo un slot atómico + smoother; la tabla vive fuera del audio.

## No-alcance

- Automatización del host, MIDI CC, macros y modulación (M2/M4).
- Persistencia de valores y presets (VS6).
- Editor de curvas por el usuario.
- Parámetros de estructura del patch (nodos/conexiones), que llegan con el grafo modular.
