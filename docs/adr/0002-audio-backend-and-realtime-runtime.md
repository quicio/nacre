# ADR-0002 — Backend de audio y runtime en tiempo real

**Estado:** Accepted para ALSA sobre PipeWire a 48 kHz / 256 frames (medido 2026-11-23). Pendiente el gate de 30 min y la comparación con PipeWire nativo y JACK.
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

Medido el 2026-11-23 en este equipo (CachyOS, PipeWire 1.x, `cpal` 0.18.2, usuario en `audio` con `rtprio` 99). Probe desechable, fuera del workspace.

| Qué | Resultado |
|---|---|
| Hosts compilados por defecto | Solo `Alsa`. `pipewire` y `jack` son features opcionales, no vienen en `default = []` |
| Dispositivo | `Default Audio Device`. El rango anunciado (1–384 000 Hz, buffer 16–262 144) es el virtual de PipeWire, no el de la tarjeta |
| `BufferSize::Fixed(256)`, 48 kHz, f32, estéreo | Aceptado |
| 45 s de seno a 440 Hz | 8436 callbacks frente a 8438 esperados (ratio 1.000). Los 2 que faltan caben en el arranque |
| 30 min, mismo stream | 337 517 callbacks frente a 337 500 esperados (ratio 1.000). 17 de más, no de menos: unos 91 ms de deriva de reloj en media hora, no bloques perdidos |
| Callback más lento | 59 µs en 45 s; 459 µs en 30 min. Presupuesto 5333 µs. Cero callbacks por encima del presupuesto en ambas pasadas |
| Xruns del kernel | No medidos. `cpal` no expone `snd_pcm_status`; el error callback no se disparó |
| Latencia impulso→salida | No medida |
| 30 min, PipeWire nativo, JACK | Gate de 30 min cumplido en ALSA sobre PipeWire (ver fila anterior). La feature `pipewire` compila; la pasada comparativa está por ejecutarse. JACK no está corriendo |

Conclusión provisional: en este equipo el camino de producción es ALSA a través de PipeWire, no ALSA directo. 256 frames a 48 kHz se sostiene 30 min sin callbacks lentos y sin bloques perdidos, con el peor callback a 459 µs de 5333 µs. Eso cierra el gate de duración para este backend. No cierra la comparación con PipeWire nativo ni con JACK, ni sustituye un contador de xruns del kernel.

Hecho de API para el change 002: en `cpal` 0.18 `SampleRate` es un alias de `u32`, el dispositivo se nombra por `Display`, y `build_output_stream` toma el `StreamConfig` por valor con el formato de muestra como parámetro de tipo. Los dos callbacks tienen que ser `'static`.
