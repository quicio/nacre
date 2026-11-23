# Design — parameter system

## Modelo

```rust
pub struct ParameterId(&'static str);          // "filter.cutoff"

pub struct ParameterSpec {
    pub id: ParameterId,
    pub slot: u16,                              // índice estable en la tabla
    pub unit: Unit,
    pub range: RangeInclusive<f32>,
    pub default: f32,
    pub curve: Curve,
    pub smoothing_ms: f32,
    pub kind: ParameterKind,                    // Continuous | Discrete
}

pub enum Curve { Linear, Logarithmic, Exponential }
```

- `slot` se asigna al registrar y no cambia: permite acceso O(1) en el callback.
- `ParameterId` es un `&'static str` en la tabla compilada; para patches dinámicos se usará un interner en VS6.
- La tabla (`ParameterTable`) vive en la aplicación y es inmutable tras el arranque; el audio recibe una copia de solo lectura de los slots que necesita.

## Curvas

```text
Linear:        n = (v - min) / (max - min)
Logarithmic:   n = ln(v/min) / ln(max/min)                 (min > 0)
Exponential:   n = (v^γ - min^γ) / (max^γ - min^γ)          (γ = 2 por defecto)
```

- Inversas exactas dentro de 1e-5; en los extremos se devuelve exactamente `min`/`max` para evitar error de redondeo.
- Un rango con `min = 0` y curva logarítmica es inválido (`InvalidSpec`).

## Ruta de un cambio

```text
UI/agente
  → Application::set_parameter(id, value)
      ├─ lookup spec (slot, rango, curva)
      ├─ validar finito y en rango
      └─ Command::Set { slot, value } → cola SPSC (capacidad 256)
  → callback
      ├─ drenar hasta 64 comandos a un array local
      ├─ coalescing: por slot, quedarse con el último
      ├─ escribir slot atómico (bits de f32) + marcar generación
      └─ el DSP lee el slot y suaviza
```

- La cola transporta valores **en unidad**; la normalización es solo de UI/automatización.
- Si la cola está llena: `Err(QueueFull)` visible; el llamador decide reintentar o descartar (la UI descarta el frame, el agente debe reintentar).
- El callback nunca busca en un `HashMap` ni formatea strings.

## Coalescing

Con `n ≤ 64` comandos por bloque, el coalescing se hace con un array de `Option<f32>` indexado por slot (64KB máximo si hay 16k slots; en la práctica, la tabla de la voz tiene <64). Se recorre una vez y se aplica el último valor por slot.

- Coste acotado y predecible.
- Los comandos discretos (waveform, destino de LFO) también se coalescen; se aplican al inicio del bloque.

## Suavizado

```text
a = 1 - exp(-1 / (tau * fs))        // tau = smoothing_ms / 1000 / 3
y[n] = y[n-1] + (target - y[n-1]) * a
```

- `smoothing_ms = 0` → salto inmediato (parámetros discretos o de control).
- El estado del smoother vive en el procesador (no en la tabla), porque hay un smoother por instancia de voz.
- Se compara `|target - y| < 1e-6` para congelar el smoother y evitar denormales; se aplica flush-to-zero si el target es 0.
- Para cutoff se suaviza en el dominio logarítmico (percepción) y se convierte a Hz antes del filtro. El sistema expone ambos: valor en unidad y valor suavizado en la unidad.

## Snapshot

Triple buffer manual, sin locks y sin allocación en audio:

```text
[ Snapshot A ][ Snapshot B ][ Snapshot C ]
        ▲            ▲            ▲
     audio escribe  índice "back"
     control lee    índice "front"
     swap con AtomicUsize (Acquire/Release)
```

- El audio escribe en el slot trasero y publica el índice con `swap`.
- El control toma el índice frontal; si el swap devuelve uno distinto, el suyo pasa a ser el trasero.
- El snapshot contiene `values: [f32; N]` y `normalized: [f32; N]` con `N` fijo en compilación (p. ej. 128), más `generation: u64`.
- Si un parámetro no cabe en `N`, el registro falla en desarrollo (error, no truncamiento silencioso).

Alternativa descartada: `RwLock<Vec<f32>>` — bloquea el audio y asigna.

## Migración desde 002/003

1. Los parámetros ad-hoc (`SetFrequency`, `SetGain`) se sustituyen por `Set { slot, value }`.
2. La tabla se define en `nacre-application` y se inyecta en el runtime; el DSP recibe valores, no la tabla.
3. Los tests de 002/003 deben seguir pasando sin cambios de comportamiento: mismo audio para los mismos valores.
4. La UI genera los controles desde `ParameterSpec` (nombre, rango, unidad, curva), eliminando campos duplicados.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| Valores normalizados en todo el sistema | Oscurece el DSP y complica los tests; la unidad es el contrato interno |
| `HashMap<ParameterId, f32>` en el callback | Hash y posibles allocaciones; descartado |
| Un lock por parámetro | Bloquea el audio; descartado |
| Enviar el valor normalizado a la cola | La UI tendría que conocer la curva del DSP; se centraliza en aplicación |
| `ArcSwap` para el snapshot | Asigna en el hilo de audio al publicar; el triple buffer es más simple y determinista |

## Riesgos

| Riesgo | Mitigación |
|---|---|
| El coalescing pierde el orden de cambios discretos | Los discretos se aplican solo al inicio de bloque; test de secuencia |
| El triple buffer publica un snapshot a medias | Escritura completa antes del `swap` con `Release`; test de generación |
| La tabla crece y el array fijo se queda corto | Límite explícito (128) y error de registro; ampliar con ADR si hace falta |
| Suavizado en Hz suena no lineal | Suavizar cutoff en log y convertir; test de barrido |
| Duplicar parámetros entre UI y DSP | La UI se genera desde la tabla; test de cobertura de controles |

## Validación

- Tests de curvas (ida y vuelta, extremos, inválidos).
- Test de validación (fuera de rango, NaN, duplicado).
- Test de suavizado (monotonía, tiempo, sin escalones).
- Test de coalescing (500 cambios → último valor; estado final correcto).
- Test de snapshot (generación consistente bajo cambios concurrentes).
- Test de regresión: los renders de 002/003 no cambian tras la migración.
- Sin asignaciones en el callback tras la migración.
