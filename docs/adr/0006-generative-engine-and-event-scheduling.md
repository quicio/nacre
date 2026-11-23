# ADR-0006 — Motor generativo y scheduling de eventos

**Estado:** Accepted
**Fecha:** 2026-11-22

## Contexto

La generación musical es el corazón del producto. Si el generador vive dentro del callback de audio, no se puede testear, serializar ni razonar; si vive en la UI, se acopla a la presentación. Además, el azar sin estructura destruye la identidad musical.

## Decisión

1. **Motor generativo separado del audio.** Produce `MusicalEvent` con `SampleTime`; no genera muestras.
2. **Crate propio** (`nacre-generative`) cuando se implemente en M2, con dependencia solo del dominio.
3. **Reloj único**: `SampleTime(u64)` del transporte; el scheduler trabaja con lookahead (2 bloques por defecto) y entrega eventos por un ring buffer SPSC.
4. **Eventos ordenados y deterministas** por `(time, sequence)`; sin allocación por evento (buffers preasignados).
5. **Semilla explícita por generador**, derivada de una semilla maestra con una función determinista (`splitmix64`). No hay `thread_rng` en el camino musical.
6. **Evolución con presupuesto** en tres escalas (micro/meso/macro) y una memoria de identidad que rechaza mutaciones destructivas.
7. **Relaciones declaradas** entre generadores, con profundidad máxima y sin ciclos.
8. **El scheduler no toca el callback**: escribe en la cola; el audio drena.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| Generar eventos en el callback | Imposible testear, riesgo de RT, no serializable; descartado |
| Generar en la UI | Acopla música a presentación; descartado |
| Usar `rand` con `thread_rng` | No reproducible; descartado para el camino musical (se permite en tests de estrés) |
| Un generador "todo en uno" con parámetros de azar | Suena a randomize; no da identidad; descartado |
| Scheduler sin lookahead | Jitter y eventos tardíos; descartado |

## Consecuencias

**Positivas**
- Testeable sin dispositivo (miles de compases por segundo).
- Reproducibilidad exacta y renders deterministas.
- El agente puede inspeccionar y mutar patrones sin tocar audio.

**Negativas**
- Hay que diseñar con cuidado la latencia del lookahead y la cuantización.
- La memoria de identidad requiere definir métricas musicales (no trivial).
- Más tipos y contratos que un generador improvisado.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| El jitter del hilo de control se traduce en timing irregular | Lookahead + cola con timestamps; medir `late_events` |
| La evolución suena aleatoria | Presupuesto de cambio + evaluación por escucha en el gate M2 |
| Deriva numérica en ejecuciones largas | Tiempo entero (frames), tests de 1 h |
| Relaciones entre generadores impredecibles | Profundidad máxima, sin ciclos, observabilidad |

## Validación

- Reproducibilidad exacta con la misma semilla en 10 000 compases.
- Jitter medido < 1 ms equivalente en el gate de M2.
- Pieza de 10 min con identidad verificable (histograma rítmico) en el gate M2.
