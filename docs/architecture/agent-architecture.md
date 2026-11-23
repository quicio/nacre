# Arquitectura del agente

El agente es una capacidad del producto, no un chat pegado a un sintetizador. Pero el instrumento debe seguir funcionando completo sin él.

## Principio

> El agente usa la **misma** API que la UI. Si el agente puede hacer algo que la UI no puede, hay un agujero de diseño.

Consecuencias:

- No hay un "canal privilegiado" del agente hacia el audio o el dominio.
- Toda operación del agente es una operación de aplicación, tipada y validada.
- MCP es un **adaptador** sobre esa API, no la base de la arquitectura.
- El proveedor de LLM es intercambiable: Ollama, modelo remoto, agente externo, o ninguno.

## Planos

```text
┌─────────────────────────── Agent Control Plane ───────────────────────────┐
│  UI / chat / cliente MCP                                                  │
│        │                                                                  │
│        ▼                                                                  │
│  AgentGateway (application)                                               │
│   inspect · propose · validate · preview · apply · discard · undo         │
│        │                                                                  │
│        ├── ReadModel (patch, graph, generators, metrics)                  │
│        └── TransactionManager (revisiones, candidates, history)           │
└───────────────────────────────┬───────────────────────────────────────────┘
                                │ PreparedGraph + comandos
                                ▼
                       Audio Data Plane (RT)
```

El agente **nunca** entra al callback, nunca bloquea el audio y nunca muta estado vivo directamente.

## Modos

| Modo | Puede | No puede | Aprobación |
|---|---|---|---|
| **Collaborator** | Inspeccionar, proponer, explicar, pedir aprobación | Aplicar sin confirmación; destructivo | Humana por cambio |
| **Explorer** | Generar variantes, mutar, audicionar alternativas, descartar | Sobrescribir el estado base; borrar historia | Humana para "adoptar" una variante |
| **Director** | Traducir intención musical a operaciones; ajustar macros y parámetros de alto nivel | Operaciones estructurales destructivas; acceso a fs/red | Configurable; destructivo siempre requiere aprobación |

Los permisos no son un prompt: son un **filtro de la API**. Una operación no permitida no se ejecuta aunque el modelo la pida. Se devuelve un error estructurado y se registra el intento.

## Flujo transaccional

```text
1. inspect     → ReadModel consistente en revisión R
2. propose     → Vec<Operation> tipadas
3. validate    → errores de dominio + límites de permisos
4. build       → CandidateState (R + ops, sin tocar el vivo)
5. preview     → diff semántico + render/audición opcional
6. apply       → commit atómico → revisión R+1
   discard     → descarta el candidato, nada cambia
7. undo/redo   → historial por transacción
```

Reglas:

- Toda operación lleva `base_revision`. Si el estado cambió, la transacción falla con `StaleRevision`; el agente debe re-inspeccionar.
- `apply` es atómico: o entra todo, o nada.
- `preview` no altera el estado vivo; puede producir un render offline o un overlay temporal en audio.
- El historial tiene límite de memoria y agrupación por transacción.
- Cada transacción registra origen (`Human`, `Agent{mode}`), resumen y operaciones, para auditoría y explicación.

## API conceptual

Inspección (solo lectura):

```
inspect_patch, inspect_graph, inspect_generators, inspect_modulation,
inspect_parameters, inspect_observations, inspect_history
```

Mutación (tipada, transaccional):

```
create_node, remove_node, connect_ports, disconnect_ports,
create_generator, remove_generator, set_generator_parameter,
create_modulation_route, remove_modulation_route,
set_parameter, set_macro, set_musical_intent,
set_generative_behavior, evolve_pattern, mutate_patch,
set_environment, capture_variation
```

Cada operación tiene: precondiciones, validación, efecto, reversibilidad y nivel de permiso. Las operaciones no son strings ni JSON libre: son enums tipados.

## Observaciones

El agente no procesa audio crudo de forma continua. Recibe snapshots estructurados:

| Observación | Uso |
|---|---|
| `rms`, `peak` | Nivel, headroom, clipping |
| `spectral_centroid_hz` | Brillo, "distancia", evolución tímbrica |
| `frequency_distribution` | Balance espectral, resonancias |
| `event_density` | Densidad rítmica percibida |
| `active_voices` | Carga y textura |
| `parameter_evolution` | Qué cambió y cuándo |
| `xruns`, `dropped_commands` | Salud del runtime (no musical) |

Con esto el agente puede razonar ("más brillante sin subir el nivel") sin recibir muestras.

## Traducción de intención musical

El agente no mapea palabras a parámetros DSP directamente. Usa una capa semántica:

```text
"más distante y enorme"
      │
      ▼
MusicalIntent { space: Large, brightness: Dark, motion: Slow, density: Low }
      │
      ▼
plan de operaciones (reverb size/decay, filtro, detune, modulación lenta)
      │
      ▼
validación + preview + explicación
```

Cada traducción produce una **explicación** en una frase: qué se cambió, por qué y qué se espera escuchar. Sin explicación, no se aplica.

## MCP

- `nacre-agent-mcp` implementa un servidor MCP que envuelve `AgentGateway`.
- Recursos: patch, composition, observaciones, historial (solo lectura).
- Herramientas: operaciones de inspección y mutación con esquemas tipados.
- El servidor no contiene lógica de negocio: traduce JSON-RPC a llamadas de aplicación y errores de vuelta.
- Seguridad: permisos por modo, límites de tasa, auditoría, sin acceso a fs/red desde el servidor.

Otros clientes (UI interna, CLI, agente externo) usan la misma API sin pasar por MCP.

## Qué no hace el agente

- No compila el grafo ni toca el callback.
- No "edita el archivo del patch" por fuera de la aplicación.
- No inventa parámetros que no existen.
- No aplica cambios sin revisión cuando el modo lo exige.
- No es necesario para que el instrumento funcione.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| El agente se convierte en la única forma de usar el instrumento | La UI expone las mismas operaciones; tests de paridad |
| Transacciones que dejan estado inconsistente | CandidateState inmutable + commit atómico + tests de propiedades |
| Latencia del LLM en el camino de audio | El agente solo actúa en el plano de control; preview offline |
| Explicaciones inventadas | La explicación se genera desde el diff de operaciones, no desde el modelo |
| Permisos "de prompt" | Filtro en la API, no en el prompt del sistema |
