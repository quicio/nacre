# ADR-0007 — Plano de control del agente

**Estado:** Proposed (se valida en M3)
**Fecha:** 2026-11-22

## Contexto

El agente debe poder inspeccionar y transformar el instrumento sin comprometer el tiempo real, la integridad del estado ni la confianza del usuario. La tentación es exponer herramientas ad hoc y dejar que el LLM llame funciones internas; eso produce estados inconsistentes, acciones irreversibles y una arquitectura imposible de auditar.

## Decisión

1. **Agent Control Plane separado** del audio. El agente nunca entra al callback.
2. **API de aplicación tipada y estable** (`AgentGateway`), independiente del proveedor de LLM.
3. **MCP como adaptador**, no como fundamento. `nacre-agent-mcp` traduce JSON-RPC a la API; no contiene lógica de negocio.
4. **Operaciones transaccionales**: `base_revision` → validar → construir candidato → preview → apply/discard → undo/redo.
5. **Tres modos con permisos aplicados en la API**: Collaborator, Explorer, Director. El prompt no es un mecanismo de seguridad.
6. **Observaciones estructuradas** (RMS, peak, centroide, densidad, voces, evolución de parámetros) en lugar de audio crudo.
7. **Explicación obligatoria**: toda transacción produce un resumen legible generado desde el diff de operaciones.
8. **Auditoría**: origen, modo, operaciones y resultado de cada transacción.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| Herramientas MCP directamente sobre el dominio | Acopla el dominio a MCP y permite operaciones no validadas; descartado |
| Un LLM con acceso a archivos del patch | Salta validación, transacciones y auditoría; descartado |
| Solo chat sin acciones | No cumple la visión de Director; insuficiente |
| Agente dentro del callback | Inviable en RT; descartado |
| Permisos por prompt | Fácil de eludir; descartado |

## Consecuencias

**Positivas**
- El agente puede fallar sin corromper el estado.
- Cambiar de proveedor (Ollama, remoto, custom) no toca el núcleo.
- La UI puede reutilizar la misma API para acciones programadas/macros.

**Negativas**
- Más trabajo de diseño de operaciones y validación.
- El modelo de revisión añade complejidad a la aplicación.
- Preview/audición requiere un render o un overlay, con coste.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| La API se queda corta para intenciones musicales ricas | Empezar por inspección + operaciones básicas; ampliar con evidencia de M3 |
| Transacciones que dejan estado inconsistente | `CandidateState` inmutable + commit atómico + tests de propiedades |
| El agente aplica cambios destructivos en modo Director | Filtro de permisos + aprobación explícita para destructivo |
| Latencia del LLM percibida como lentitud del instrumento | El audio sigue sonando; el agente opera en el plano de control |
| Explicaciones que no coinciden con lo aplicado | Resumen generado del diff, no del modelo |

## Validación (gate M3)

- Un cliente sin LLM puede inspeccionar y producir un informe completo.
- Una operación con revisión obsoleta falla con `StaleRevision` y no altera nada.
- Un agente vía MCP construye un patch desde una intención y lo revierte; el estado vuelve a ser idéntico.
- El modo Director no puede ejecutar operaciones destructivas sin aprobación.
