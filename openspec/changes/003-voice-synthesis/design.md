# Design — voice synthesis

## Cadena de señal

```text
Gate/Pitch
    │
    ▼
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌────────────┐   ┌─────────┐
│ Saw      │──►│ SVF LP   │──►│ VCA      │──►│ Saturation │──►│ DC block│──► out
│ PolyBLEP │   │ 12 dB/oct│   │ (ADSR)   │   │ soft clip  │   │ 1-pole  │
└──────────┘   └────▲─────┘   └────▲─────┘   └────────────┘   └─────────┘
                    │              │
                 ┌──┴──┐        ┌──┴──┐
                 │ LFO │        │ ADSR│
                 └─────┘        └─────┘
```

Procesamiento por bloque, con parámetros suavizados dentro del bloque cuando cambian.

## Oscilador saw (PolyBLEP)

- Fase normalizada `[0,1)` en `f64`.
- Forma base: `2*phase - 1`.
- Corrección PolyBLEP en la discontinuidad (salto de −1 a +1) y en la mitad del ciclo, con ancho proporcional al incremento de fase.
- PolyBLEP con oversampling 4× solo en el oscilador, decidido por S4 (2026-11-23). 2× no cumple −60 dB a 1760 Hz. El resto de la voz corre a la tasa del dispositivo.

```text
poly_blep(t, dt) =
  t < dt        →  t/dt*2 - (t/dt)^2 - 1
  t > 1 - dt    →  ((t-1)/dt)^2 + 2*(t-1)/dt + 1
  otherwise     →  0
```

- La frecuencia se actualiza por bloque; el incremento se recalcula y la fase continúa (no se reinicia).
- El oscilador es la única fuente de alias del slice; el resto de etapas no genera componentes nuevas por encima de Nyquist (la saturación se limita a armónicos bajos con drive moderado; si el drive alto genera alias, se aplica oversampling local en la etapa de saturación en una iteración posterior).

## Filtro (TPT/SVF)

Topología de variable de estado con transformada bilineal pre-warped (Zavalishin):

```text
g = tan(pi * fc / fs)
k = 1 / Q
a1 = 1 / (1 + g*(g + k))
a2 = g * a1
a3 = g * a2

v3 = x - ic2eq
v1 = a1*ic1eq + a2*v3
v2 = ic2eq + a2*ic1eq + a3*v3
ic1eq = 2*v1 - ic1eq
ic2eq = 2*v2 - ic2eq
lowpass = v2
```

- Coeficientes recalculados cuando cambia cutoff o Q (por bloque o por muestra si hay modulación rápida; se elige por bloque con interpolación lineal para el LFO).
- Cutoff limitado a `[20, 0.45 * fs]` y Q a `[0.5, 20]`; los coeficientes se sanean (sin NaN, sin `tan` de π/2).
- Estado `ic1eq`/`ic2eq` en `f64` para estabilidad con Q alto.

## Envolvente ADSR

- Segmentos lineales en amplitud con curvas exponenciales suaves (one-pole) para evitar el "click" del ataque lineal:
  - Ataque: `1 - exp(-t/tau_a)`, normalizado para llegar a 1 en `attack_ms`.
  - Decay/Release: one-pole hacia el objetivo con tiempo `~3*tau`.
- Retrigger: el nuevo ataque parte del valor actual de la envolvente (no de 0) y se recalcula la pendiente; evita clicks.
- Sustain se alcanza tras `attack + decay`; si sustain = 1, no hay decay.
- La envolvente se calcula por muestra (coste bajo) y se aplica como VCA.

## LFO

- Fase propia, independiente del oscilador; frecuencia `[0.01, 20] Hz`.
- Formas: seno (tabla o `sin` aproximado) y triángulo.
- Salida bipolar `[-1, 1]`; profundidad en porcentaje.
- Destinos del slice: cutoff (modulación multiplicativa en octavas) y pitch (semitonos).
- Sin sync a tempo en este change (llega con el transporte musical en M2).

## Saturación

- `tanh` con normalización: `y = tanh(drive * x) / tanh(drive)` para compensar la ganancia.
- `drive` en dB → factor lineal `10^(dB/20)`.
- Salida acotada a `[-1, 1]` y DC blocker después.
- Si S4 detecta alias audible con drive alto, se evalúa oversampling local 2× en esta etapa (no en toda la voz).

## Protecciones

- `sanitize(x)`: `if x.is_finite() { x } else { 0.0 }` en cada frontera de unidad.
- Clamp de parámetros en la frontera de aplicación y de nuevo en el DSP (defensa en profundidad).
- DC blocker: `y[n] = x[n] - x[n-1] + R*y[n-1]` con `R = 1 - 2π*5/fs` (corte ~5 Hz).
- Headroom: la voz sale a −6 dBFS nominal; el bus maestro aplica ganancia.

## Parámetros del slice

| Parámetro | Rango | Unidad | Suavizado |
|---|---|---|---|
| `osc.frequency` | 20–2 000 | Hz | 5 ms |
| `osc.waveform` | sine, saw | — | cambio en frontera de bloque |
| `filter.cutoff` | 20–20 000 | Hz | 10 ms |
| `filter.resonance` | 0.5–20 | Q | 10 ms |
| `env.attack` | 1–10 000 | ms | no |
| `env.decay` | 1–10 000 | ms | no |
| `env.sustain` | 0–1 | — | 10 ms |
| `env.release` | 1–10 000 | ms | no |
| `lfo.rate` | 0.01–20 | Hz | 20 ms |
| `lfo.depth` | 0–100 | % | 20 ms |
| `lfo.destination` | cutoff, pitch | — | frontera de bloque |
| `sat.drive` | 0–24 | dB | 20 ms |
| `master.gain` | −60–0 | dB | 10 ms |

En el change 004 estos parámetros pasan al sistema común; aquí se implementan con la estructura mínima ya existente para no duplicar trabajo.

## UI del slice

- Teclado virtual de una octava o botón "Gate" (nota fija A2) para probar la envolvente.
- Controles: waveform, cutoff, resonance, A/D/S/R, LFO rate/depth/destination, drive, ganancia.
- Visualización simple de la forma de onda (opcional; no bloquea el slice).
- Renders de referencia reproducible desde la app o un ejemplo CLI (`cargo run --example render_ref`).

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| Oversampling global 4× | Coste alto y complejidad; se decide por unidad tras S4 |
| Filtro biquad directo (RBJ) | Menos estable con modulación rápida y Q alto; TPT es mejor para audio |
| ADSR lineal puro | Clicks y sensación mecánica; se usan curvas suaves |
| `fundsp` para la voz | Acopla el motor a su modelo; además impide controlar el coste por etapa |
| Wavetable saw | Pierde el carácter "duro" buscado y no resuelve el alias mejor que PolyBLEP con menos control |

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Aliasing audible | S4 mide; PolyBLEP + oversampling local si hace falta |
| Filtro inestable a Q alto | TPT + estado f64 + clamp de coeficientes |
| Click en retrigger | Ataque desde el valor actual + tests de discontinuidad |
| Saturación con alias | Drive acotado + medición; oversampling local si es necesario |
| La voz suena "a plugin genérico" | Banco de escucha en el gate M1; diseño de patches de referencia antes de añadir features |

## Validación

- Tests espectrales (FFT) para alias y respuesta del filtro.
- Tests de envolvente (tiempos, monotonía, retrigger, sustain).
- Tests de NaN/Inf/DC/clipping.
- Tres renders de referencia de ≥ 2 min con parámetros documentados.
- Escucha crítica: identidad sonora descriptible por un tercero.
