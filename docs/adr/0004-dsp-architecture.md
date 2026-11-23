# ADR-0004 — Arquitectura DSP

**Estado:** Accepted (con hipótesis abiertas sobre anti-aliasing y librerías)
**Fecha:** 2026-11-22

## Contexto

El sonido es el producto. El DSP debe ser correcto, estable, testeable sin dispositivo y de coste predecible en tiempo real. También debe permitir empezar pequeño sin cerrar la puerta a síntesis más compleja.

Datos verificados (2026-11-22):

- `fundsp` 0.23.0, licencia MIT OR Apache-2.0, mantenimiento moderado (último release 2026-01-07), ~283k descargas. Biblioteca de DSP funcional con grafos y primitivas.
- `egui`/`eframe` 0.37 con MSRV 1.95 (relevante para el toolchain del workspace).

## Decisión

1. **DSP propio en Rust** para las unidades del instrumento (osciladores, envolventes, filtros, LFO, saturación, delay, reverb). Sin dependencia de un framework de síntesis como base del motor.
2. `fundsp` se evalúa solo como **referencia y fuente selectiva de primitivas** (p. ej. para prototipos o comparación en tests), nunca como dependencia del camino de audio en producción sin un ADR específico.
3. Unidades DSP puras, sin estado global, sin I/O, sin UI:
   - `prepare(sample_rate, max_block)` una vez.
   - `reset()` explícito.
   - `process(block, params)` determinista.
4. Procesamiento en `f32` con acumulación en `f64` donde la estabilidad lo requiera (integradores, filtros).
5. Saneado obligatorio en la frontera de cada unidad: NaN/Inf → 0, clamping documentado, DC blocker donde aplique.
6. Filtros con topología TPT/SVF (zero-delay feedback) para estabilidad en barridos y modulación rápida.
7. Anti-aliasing de saw/square: decidir entre **PolyBLEP** y **oversampling 2×/4×** mediante spike S4. Por defecto, PolyBLEP por coste; oversampling si la medición auditiva lo exige.
8. Sin SIMD en la primera versión. Se mide primero; se optimiza con datos.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| `fundsp` como base | Rápido para prototipos, pero el modelo de grafos y sus tipos condicionan la arquitectura; difícil de integrar con un grafo tipado propio y con control fino de RT |
| Framework de audio completo (p. ej. `bevy` + `rodio`) | Demasiado peso y modelo ajeno; no resuelve el problema musical |
| DSP en C/C++ vía FFI | Riesgo de seguridad y de build multiplataforma sin beneficio claro |
| Oversampling universal | Coste alto; se decide por unidad, no globalmente |
| Wavetables para todo | No da la identidad orgánica/metal buscada; se reserva para M4 |

## Consecuencias

**Positivas**
- Control total del sonido y del coste por bloque.
- Tests unitarios de DSP sin dispositivo, rápidos y deterministas.
- Sin lock-in de framework.

**Negativas**
- Más trabajo inicial que usar `fundsp`.
- Riesgo de aliasing y de inestabilidad si no se testea con rigor.
- La calidad depende del criterio del autor; requiere banco de escucha.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Aliasing audible en saw | PolyBLEP + análisis espectral; spike S4 |
| Filtro inestable con cutoff/resonancia extremos | TPT/SVF + tests de barrido + clamp de coeficientes |
| Deriva de DC y clipping acumulado | DC blocker + soft clip + headroom definido |
| Sobreoptimización prematura | Sin SIMD hasta tener perfil de CPU real |
| "DSP propio" se convierte en una librería interminable | Solo las unidades que el roadmap necesita; nada de catálogo |

## Validación

- `oscillator_frequency_accuracy`: error < 1 cent a 440 Hz en 48 kHz.
- `filter_stability`: sin NaN/Inf en barridos de 20 Hz–20 kHz con Q hasta 20.
- `alias_suppression`: componentes > 20 kHz por debajo del umbral acordado (p. ej. −60 dBFS) en saw a 1 kHz.
- `dc_offset`: < 0.001 tras 10 s de saw.
- Comparación A/B con `fundsp` en tests (no en producción) para detectar errores graves.
