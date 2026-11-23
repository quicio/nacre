# ADR-0005 — Ejecución de grafo y actualizaciones en runtime

**Estado:** Accepted (la estrategia de crossfade requiere validación en S3)
**Fecha:** 2026-11-22

## Contexto

Un instrumento modular necesita cambiar su grafo sin cortar el audio: añadir un nodo, reconectar, cambiar un patch completo. Mutar el grafo vivo desde la UI o el agente es la forma más común de provocar clicks, carreras de datos y fallos de tiempo real.

## Decisión

1. **Compilar antes de tocar audio.** El documento de patch se valida en el dominio y se compila a un `PreparedGraph` inmutable fuera del callback.
2. **Nunca mutar el grafo activo.** La activación intercambia punteros a grafos preparados en una frontera de bloque.
3. **Crossfade equal-power** de 20 ms por defecto (10–100 ms configurable) entre grafo saliente y entrante.
4. **Liberación fuera del callback.** El grafo retirado se envía al hilo de control para su destrucción.
5. **Un grafo de audio es un DAG.** El feedback solo existe dentro de unidades con estado explícito (delay, reverb), nunca como ciclo entre nodos.
6. **Compilación fallida = patch anterior intacto.** El runtime reporta el error al plano de control; nunca queda en silencio.
7. **Cambios incrementales**: cuando el compilador pueda reutilizar nodos no afectados, se permitirá un swap parcial; en la primera versión, cualquier cambio estructural recompila el patch completo (más simple y correcto).

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| Mutar el grafo vivo con locks | Riesgo inaceptable en RT; descartado |
| Un solo grafo con "add/remove node" atómico en audio | Complejo, propenso a errores; aplazado hasta tener métricas |
| Recompilar y cortar el audio (stop/start) | Audible; inaceptable para un instrumento |
| Crossfade de patch completo | Elegido: simple, robusto, con coste temporal acotado |
| Doble motor permanente con mezcla | Coste de CPU duplicado permanente; solo durante la transición |

## Consecuencias

**Positivas**
- Sin clicks en cambios estructurales.
- Modelo mental simple: el audio solo ve grafos inmutables.
- El agente y la UI usan la misma ruta.

**Negativas**
- Pico de CPU durante el crossfade (dos grafos).
- El estado interno (fase, envolventes) no se transfiere: puede haber una transición tímbrica audible, aceptable si es corta.
- Recompilar todo el patch en cada cambio estructural puede ser caro en patches grandes (se optimiza después con métricas).

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Click por desalineación de fase entre grafos | Crossfade equal-power + prueba de discontinuidad |
| Pico de CPU provoca xrun | Limitar tamaño de patch en MVP; medir con S3; crossfade más largo si hace falta |
| Liberación accidental en callback | Propiedad clara: el runtime solo mueve el `Box`, nunca lo destruye |
| Estado colgado del grafo viejo | Contratos de `Drop` que no hacen I/O; test de fugas |

## Validación (spike S3)

- Cambiar entre dos patches (seno → saw filtrado) 100 veces durante reproducción; analizar la señal en busca de discontinuidades > umbral y de clicks audibles.
- Medir pico de CPU durante la transición.
- Confirmar 0 asignaciones dentro del callback.
- Criterio de salida: ningún click audible, ninguna discontinuidad > 0.01 en la salida, sin xruns.
