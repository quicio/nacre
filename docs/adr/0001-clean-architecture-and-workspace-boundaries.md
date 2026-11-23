# ADR-0001 — Clean Architecture y límites del workspace

**Estado:** Accepted
**Fecha:** 2026-11-22
**Decisores:** hugo, agente (propuesta)

## Contexto

Nacre debe sobrevivir a cambios de UI, de backend de audio, de host de plugins y de proveedor de IA. El riesgo principal en proyectos de audio en Rust no es la falta de abstracción, sino la abstracción prematura y la contaminación del dominio con detalles de DSP y de frameworks.

## Decisión

1. Clean Architecture con cuatro capas conceptuales: Domain, Application, Infrastructure, Presentation.
2. Las dependencias apuntan hacia adentro. `nacre-domain` no declara dependencias externas salvo `thiserror`.
3. El audio en tiempo real se trata como una **frontera de runtime**, no como un caso de uso (ver ADR-0002 y ADR-0005).
4. Se crean solo cinco crates en el primer tramo: `nacre-domain`, `nacre-application`, `nacre-dsp`, `nacre-audio-runtime`, `apps/nacre-standalone`. `nacre-persistence` se añade en VS6.
5. Las reglas de dependencia se verifican en CI, no solo por convención.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| Monolito con módulos | Barato al inicio, pero los límites se erosionan; difícil testear DSP sin dispositivo |
| Arquitectura hexagonal pura con un crate por puerto | Sobrearquitectura: decenas de crates sin contenido real |
| Framework ECS (p. ej. bevy_ecs) como base del grafo | Introduce un modelo de mundo ajeno al dominio musical y coste de adaptación; se puede reevaluar si el grafo lo justifica |
| "Solo hazlo sonar y luego refactoriza" | El coste de rehacer la frontera RT y el modelo de dominio es mayor que el de definirla ahora |

## Consecuencias

**Positivas**
- El dominio se puede testear sin audio, sin UI y sin red.
- Cambiar de UI o de backend no toca la lógica musical.
- El agente usa la misma API que la UI.

**Negativas**
- Más ceremonia inicial: mapear DTOs de persistencia, definir puertos.
- Riesgo de duplicar tipos entre dominio y persistencia.
- El equipo debe respetar la dirección de dependencias sin excepciones "temporales".

## Riesgos

| Riesgo | Mitigación |
|---|---|
| Sobrearquitectura antes del primer sonido | Presupuesto: 5 crates, un oscilador, ninguna abstracción sin segundo uso |
| Puertos que se inventan antes de conocer el runtime | Los puertos del primer slice son mínimos (`AudioHost`, `Clock`, `Renderer`); se amplían con evidencia |
| Reglas de dependencia que se relajan | Script de CI + ADR obligatorio para excepciones |

## Validación

- VS1: el workspace compila con las cinco piezas y un test por crate.
- VS2: `cpal` solo aparece en `nacre-audio-runtime` (`cargo tree -i cpal`).
- VS3: ningún tipo de `egui` en `nacre-dsp` ni en el dominio.
- VS6: la persistencia no obliga a añadir `serde` al dominio.
