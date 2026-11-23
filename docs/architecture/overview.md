# Arquitectura — visión general

## Enfoque

Clean Architecture con una excepción explícita y documentada: **el audio en tiempo real no es un caso de uso**. Se separa el *plano de control* (aplicación, UI, agente) del *plano de datos* (callback de audio), y esa frontera se diseña como un contrato propio, no como una capa de aplicación disfrazada.

La regla que gobierna todo lo demás:

> Las dependencias de código apuntan hacia adentro. El dominio no depende de nada.

## Capas

```text
                    ┌──────────────────────────────┐
                    │        PRESENTATION          │
                    │  standalone UI · plugin UI   │
                    └──────────────┬───────────────┘
                                   │ commands / queries / view models
                    ┌──────────────▼───────────────┐
                    │         APPLICATION          │
                    │ use cases · ports · txns     │
                    └──────────────┬───────────────┘
                                   │ domain operations
                    ┌──────────────▼───────────────┐
                    │           DOMAIN             │
                    │ patch · composition · perf   │
                    │ parameters · musical intent  │
                    └──────────────────────────────┘
                                   ▲
                                   │ implements ports
                    ┌──────────────┴───────────────┐
                    │       INFRASTRUCTURE         │
                    │ cpal · files · midi · mcp    │
                    └──────────────────────────────┘
```

La infraestructura **no** está "debajo" del dominio en el grafo de dependencias: implementa interfaces definidas hacia adentro y se inyecta en la composición (app). El dominio nunca la nombra.

## Mapa de capas a crates

| Capa | Crate | Responsabilidad |
|---|---|---|
| Domain | `nacre-domain` | Entidades, value objects, invariantes, errores de dominio, operaciones puras |
| Application | `nacre-application` | Casos de uso, puertos, transacciones, revisiones, view models de lectura |
| DSP (interna) | `nacre-dsp` | Unidades DSP puras, sin I/O ni UI; deterministas y testeables offline |
| Runtime (infra) | `nacre-audio` | Dispositivo, callback, reloj, activación de grafo, transferencia RT |
| Presentation | `apps/nacre-standalone` (+ `nacre-ui` cuando exista el plugin) | Composición de dependencias, ventana, controles |

`nacre-dsp` no es una capa de Clean Architecture: es una **biblioteca de dominio técnico** que no conoce la aplicación ni la UI. Se mantiene separada para poder testearla offline, sin dispositivo, y para aislar `unsafe`/SIMD si llega.

## Dirección de dependencias

```text
apps/nacre-standalone ──► nacre-application ──► nacre-domain
        │                        ▲
        ├──► nacre-audio ────────┘
        │        │
        │        └──► nacre-dsp ──► nacre-domain (solo tipos compartidos)
        └──► nacre-persistence ──► nacre-application / nacre-domain
```

Reglas:

1. `nacre-domain` no depende de ningún otro crate del workspace ni de crates externos de I/O.
2. `nacre-application` depende solo de `nacre-domain`.
3. `nacre-dsp` puede depender de `nacre-domain` para newtypes de parámetros, nunca al revés.
4. `nacre-audio` depende de `nacre-dsp` y de los *puertos* de `nacre-application`.
5. La UI depende de `nacre-application`; jamás de `nacre-dsp` ni de `nacre-audio` internals.
6. Los adaptadores (persistencia, MCP, MIDI) dependen hacia adentro; nadie depende de ellos salvo la composición.

Ver [dependency-graph.md](dependency-graph.md) para el detalle y las reglas de CI.

## Los dos planos

### Plano de control

UI, agente, casos de uso, validación, compilación de grafo, persistencia. Puede bloquear, asignar memoria, hacer I/O, esperar red y usar LLMs. Corre en sus propios hilos.

### Plano de datos

Callback de audio, unidades DSP, mezcla, medición. Trabajo acotado y determinista por bloque. No bloquea, no asigna, no hace I/O.

### Frontera

```text
 Application / Control Plane
            │
            │  comandos preparados, snapshots, ring buffers SPSC
            ▼
    ┌───────────────────┐
    │  Runtime Boundary │  ← único punto de contacto entre planos
    └─────────┬─────────┘
              ▼
        Audio Engine  (grafo preparado, inmutable durante el bloque)
              ▼
           DSP units
              ▼
         Audio Output
```

Contratos de la frontera:

| Dirección | Mecanismo | Garantía |
|---|---|---|
| Control → audio | Ring buffer SPSC acotado (`rtrb`) con comandos pequeños y `Copy` | Sin bloqueo; el productor nunca espera; si está lleno, el comando se rechaza con error visible |
| Control → audio | Parámetros atómicos por slot (`AtomicU32` con bits de `f32`) | Lectura sin lock; último valor gana; el smoothing ocurre en audio |
| Audio → control | Snapshot doble-buffer (`ArcSwap` o triple buffer) de medidores/estado | Lectura consistente sin bloquear el audio |
| Audio → control | Ring buffer de eventos de salida (p. ej. note-off vencidos) | Capacidad acotada, descarte contado |

Nada de `Mutex`, `RwLock`, `std::sync::mpsc`, `Arc<Mutex<Graph>>` ni `Vec` que crezca dentro del callback.

## Flujo: cambiar un parámetro desde la UI

```text
UI slider
  → Application::set_parameter(ParameterId, RawValue)
      → validación de rango y revisión (fuera de audio)
      → ParameterChange escrito en el ring buffer
  → callback: drena el ring buffer al inicio del bloque
      → actualiza el slot atómico del parámetro
      → el smoother interpola durante el bloque
  → DSP usa el valor suavizado
```

La UI nunca escribe en el grafo ni en el estado del procesador. Solo envía comandos.

## Flujo: reemplazar el grafo

```text
PatchDocument (inmutable)
   → validate()            (dominio: puertos, ciclos, tipos)
   → compile()             (prepara buffers, orden topológico, plan de ejecución)
   → PreparedGraph (sin allocaciones en audio)
   → Runtime::activate()   (crossfade con el grafo anterior)
   → retiro del grafo antiguo fuera del callback
```

Detalle en [audio-runtime.md](audio-runtime.md).

## Puertos (interfaces hacia adentro)

Definidos en `nacre-application`:

- `PatchRepository`: cargar/guardar/listar documentos.
- `AudioHost`: iniciar/parar dispositivo, consultar estado, enviar comandos preparados.
- `Clock`: tiempo actual del transporte (frames, tempo, posición musical).
- `EventSink`: entrega de eventos generados al runtime.
- `Renderer`: render offline de un patch/composition a buffer.
- `Observer`: publicación de mediciones para UI y agente.
- `AgentGateway`: operaciones tipadas expuestas al agente (implementado por aplicación, consumido por el adaptador MCP).

`nacre-audio` implementa `AudioHost`, `Clock`, `EventSink`, `Renderer` y `Observer`. `nacre-persistence` implementa `PatchRepository`. El adaptador MCP consume `AgentGateway`.

## Qué NO cruza la frontera

- Tipos de `cpal`, `egui`, `nih-plug`, `serde` o del SDK de MCP.
- `Arc<Mutex<...>>` compartido entre UI y audio.
- Referencias al patch "vivo" dentro del callback.
- Errores de dominio convertidos en pánicos.

## Riesgos arquitectónicos aceptados

| Riesgo | Estado | Mitigación |
|---|---|---|
| El modelo de dominio se contamina con detalles de DSP | Vigilado | ADR-0004: DSP recibe parámetros, no entidades |
| La frontera RT se rompe por comodidad | Alto | Test de asignación + revisión obligatoria de PR |
| `nacre-dsp` se convierte en una capa oculta con estado global | Vigilado | Unidades sin estado global; estado explícito por instancia |
| La UI empuja lógica de negocio | Medio | View models de solo lectura; comandos tipados |
| Sobrearquitectura antes de tener sonido | Alto | Solo 5 crates en VS1; nada de plugins internos ni buses genéricos |
