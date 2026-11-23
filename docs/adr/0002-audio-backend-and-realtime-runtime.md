# ADR-0002 — Backend de audio y runtime en tiempo real

**Estado:** Accepted. En Linux se prefiere el host nativo de PipeWire (`cpal` feature `pipewire`); ALSA es el respaldo. Medido el 2026-11-23.
**Fecha:** 2026-11-22

## Contexto

Necesitamos salida de audio de baja latencia, multiplataforma, con control explícito de buffers, en Rust. El backend determina el comportamiento en tiempo real, la latencia y la integración con el sistema (PipeWire en Linux, CoreAudio en macOS, WASAPI en Windows).

Datos verificados (2026-11-22):

- `cpal` 0.18.2, licencia Apache-2.0, muy mantenido (último release 2026-08-16; actividad 2026-11-21), MSRV 1.85.
- Backends: ALSA (Linux, obligatoria incluso con otros), PipeWire nativo (feature `pipewire`, PipeWire ≥ 0.3.53), JACK, PulseAudio, CoreAudio, WASAPI, ASIO (Windows).
- En Linux, `BufferSize::Default` puede resolverse a valores muy grandes o dispares; los desajustes de sample rate con PipeWire provocan resampling y glitches audibles.
- La promoción a prioridad de tiempo real requiere `realtime` (CAP_SYS_NICE/rtprio) o `realtime-dbus` (rtkit).

## Decisión

1. `cpal` como backend inicial, aislado en `nacre-audio-runtime`.
2. Salida estéreo, `f32`, preferencia 48 kHz; si el dispositivo no lo soporta, se negocia el formato más cercano y se informa.
3. Buffer objetivo 256 frames (rango 64–1024, configurable). En Linux, con la feature `pipewire` compilada y un servidor presente, se prefiere el host nativo de PipeWire; ALSA queda como respaldo. La feature no está en `default` de `cpal` y hay que activarla a propósito.
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
- En Linux el host nativo de PipeWire dice la verdad sobre el reloj: 48 kHz fijo y buffers entre 32 y 2048. El plugin ALSA de PipeWire anuncia 1–384 000 Hz y buffers enormes, y resamplea por detrás.

**Negativas**
- `cpal` en Linux arrastra ALSA aunque el host elegido sea PipeWire. La feature `pipewire` además exige `libpipewire-0.3` al compilar.
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

Medido el 2026-11-23 en este equipo (CachyOS, PipeWire 1.x, `cpal` 0.18.2, usuario en `audio` con `rtprio` 99). Probe desechable, fuera del workspace.

| Qué | Resultado |
|---|---|
| Hosts con feature `pipewire` | `PipeWire` y `Alsa`. Sin la feature, solo `Alsa` |
| PipeWire nativo, 45 s | Dispositivo `default_output`. f32 estéreo **48 000–48 000 Hz**, buffer 32–2048. `Fixed(256)` aceptado: 8434/8438 callbacks, máximo 50 µs, cero lentos |
| ALSA (plugin de PipeWire), 45 s | Sigue anunciando 1–384 000 Hz y buffer 16–262 144. 8439/8438 callbacks, máximo 42 µs, cero lentos |
| 30 min en ALSA sobre PipeWire | 337 517/337 500 callbacks. 17 de más (~91 ms de deriva), no bloques perdidos. Peor callback 459 µs de 5333 µs. Cero lentos |
| Xruns del kernel | No medidos. `cpal` no expone `snd_pcm_status`; el error callback no se disparó en ninguna pasada |
| Latencia impulso→salida | No medida |
| JACK | No hay `jackd`. No medido |

Conclusión: en este equipo se prefiere el host nativo de PipeWire. Ambos caminos sostienen 256 frames a 48 kHz (ratio 1.000, callback muy por debajo de los 5333 µs), pero solo el nativo anuncia el reloj real. El plugin ALSA acepta cualquier rate y resamplea. El gate de 30 min está medido en ALSA, no en el host nativo; la pasada nativa fue de 45 s. No sustituye un contador de xruns del kernel ni una medida de latencia impulso→salida.

Hecho de API para el change 002: en `cpal` 0.18 `SampleRate` es un alias de `u32`, el dispositivo se nombra por `Display`, y `build_output_stream` toma el `StreamConfig` por valor con el formato de muestra como parámetro de tipo. Los dos callbacks tienen que ser `'static`.
