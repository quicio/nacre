# ADR-0002 — Backend de audio y runtime en tiempo real

**Estado:** Accepted (sujeto a validación por spike S1)
**Fecha:** 2026-11-22

## Contexto

Necesitamos salida de audio de baja latencia, multiplataforma, con control explícito de buffers, en Rust. El backend determina el comportamiento en tiempo real, la latencia y la integración con el sistema (PipeWire en Linux, CoreAudio en macOS, WASAPI en Windows).

Datos verificados (2026-11-22):

- `cpal` 0.18.2, licencia Apache-2.0, muy mantenido (último release 2026-08-16; actividad 2026-11-21), MSRV 1.85.
- Backends: ALSA (Linux, obligatoria incluso con otros), PipeWire nativo (feature `pipewire`, PipeWire ≥ 0.3.53), JACK, PulseAudio, CoreAudio, WASAPI, ASIO (Windows).
- En Linux, `BufferSize::Default` puede resolverse a valores muy grandes o dispares; los desajustes de sample rate con PipeWire provocan resampling y glitches audibles.
- La promoción a prioridad de tiempo real requiere `realtime` (CAP_SYS_NICE/rtprio) o `realtime-dbus` (rtkit).

## Decisión

1. `cpal` como backend inicial, aislado en `nacre-audio`.
2. Salida estéreo, `f32`, preferencia 48 kHz; si el dispositivo no lo soporta, se negocia el formato más cercano y se informa.
3. Buffer objetivo 256 frames (rango 64–1024, configurable). En Linux se probarán ALSA y PipeWire nativo; la elección final depende del spike S1.
4. El sample rate y el tamaño de bloque son **parámetros del runtime**, no constantes del DSP. Los coeficientes se recalculan al preparar.
5. El callback cumple el contrato de tiempo real de `docs/architecture/audio-runtime.md`: sin allocación, sin locks, sin I/O, sin pánicos.
6. El reloj del dispositivo es el maestro del transporte.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| `cpal` | Elegida: madurez, multiplataforma, control de stream, licencia permisiva |
| `rodio` | Capa de alto nivel sobre cpal; menos control del callback y del buffer. Rechazada para el motor, podría usarse en herramientas internas |
| JACK directo | Excelente en Linux profesional, pero dependencia de servidor y mala historia multiplataforma para un producto standalone |
| WASAPI/CoreAudio/ALSA directos | Multiplica el mantenimiento por plataforma sin beneficio claro en esta fase |
| `sdl2`/`miniaudio` | Enfoque de juego, menos control fino y dependencias nativas pesadas |

## Consecuencias

**Positivas**
- Un solo backend para las tres plataformas; el DSP no conoce el dispositivo.
- Ruta clara para JACK/PipeWire si el usuario profesional lo necesita.

**Negativas**
- `cpal` en Linux arrastra dependencia de ALSA aunque se use PipeWire.
- El comportamiento de `BufferSize::Default` es inconsistente; hay que fijar el buffer explícitamente.
- La promoción RT requiere configuración del sistema (grupo `audio`, `rtprio`, rtkit); no es "plug and play".

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Glitches por desajuste de sample rate con PipeWire | Coincidir con el clock del servidor; spike S1 mide xruns |
| Callback con allocaciones ocultas (p. ej. en `cpal` o en logging) | Test de asignación con allocator instrumentado; sin logging en audio |
| Bloqueos del dispositivo al cambiar configuración | Reconstrucción controlada del stream fuera del callback |
| Latencia inaceptable con buffer grande por defecto | Fijar `BufferSize::Fixed` y validar en el spike |

## Validación (spike S1)

Criterio de salida: 30 minutos de reproducción continua a 48 kHz/256 frames en el equipo de desarrollo con **0 xruns** y sin allocaciones en el callback; documentar latencia medida y comportamiento en ALSA vs PipeWire nativo vs JACK. Si no se cumple, se reevalúa el backend antes de implementar el resto del slice.
