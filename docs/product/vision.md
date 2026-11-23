# Visión de producto

## Tesis

La mayoría de los instrumentos software piden al usuario que especifique **qué** suena y **cuándo**. Nacre pide que especifique **cómo evoluciona** un sistema sonoro, y luego lo deja funcionar.

> No programes canciones. Crea sistemas capaces de producir música.

Esto no es un generador aleatorio con estética: es un instrumento con identidad, en el que la variación es un material compositivo y la estabilidad es una decisión de diseño.

## Qué es

Un instrumento musical generativo, nativo, en Rust, con:

1. **Un motor de audio de calidad** — síntesis y procesamiento propios, sin muestras obligatorias.
2. **Un motor generativo de primera clase** — produce eventos musicales, no audio; tiene estado, semillas y relaciones.
3. **Un modelo de patch modular pero coherente** — conexiones tipadas de audio, control y eventos, no un lienzo infinito de cables.
4. **Semántica musical en la interfaz** — densidad, tensión, movimiento, atmósfera, inestabilidad; no solo cutoff y resonance.
5. **Un agente integrado** — capaz de inspeccionar, proponer, transformar y dirigir, siempre con transacciones y reversibilidad.
6. **Un runtime independiente** — el mismo motor sirve a la app standalone y, más adelante, al plugin.

## Qué no es

- Un DAW ni un reemplazo de Ableton/Bitwig/Reaper.
- Un secuenciador por pasos con un botón "randomize".
- Un clon de VCV Rack o de un modular Eurorack.
- Una colección de plugins de efectos.
- Un chatbot con un sintetizador detrás.
- Un producto que dependa de un proveedor de LLM para funcionar.

## Identidad sonora

Nacre debe sonar a **material**: piedra, metal, agua, mineral, membrana. La referencia estética es la electrónica experimental y la ciencia ficción especulativa — la escala y la atmósfera de *Dune* — no los efectos de sonido de videojuego.

Capacidades sonoras objetivo, en orden de prioridad:

1. **Drones profundos y texturas atmosféricas** con movimiento interno lento y no repetitivo.
2. **Ritmos orgánicos e imperfectos**: microdesplazamientos de timing y velocity que respiran, sin sonar desafinados ni caóticos.
3. **Timbres metálicos y resonantes**: FM, resonadores, saturación controlada, filtros con carácter.
4. **Movimiento armónico complejo**: relaciones interválicas que evolucionan sin progresiones pop.
5. **Espacialidad**: profundidad, distancia, tamaño; reverb y delay como arquitectura, no como adorno.
6. **Comportamiento autónomo**: el sistema puede sostener una pieza durante minutos sin intervención y seguir siendo reconocible.

### Antipatrones sonoros

- Aleatoriedad sin estructura (el "randomize" que destruye la identidad).
- Sci-fi genérico: risers, zaps, drones de película de serie B.
- Agresividad gratuita: distorsión como sustituto de diseño.
- Estática: un patch que suena igual durante diez minutos.
- Exceso de módulos: veinte osciladores que nadie entiende.

## Principios de experiencia

- **Resultado musical antes que configuración.** Un patch nuevo debe poder sonar bien en menos de un minuto.
- **Complejidad progresiva.** El usuario simple no ve el grafo; el usuario avanzado puede abrirlo.
- **El agente propone, el humano decide.** Toda acción del agente es inspeccionable, reversible y explicable.
- **La variación se escucha.** Evolucionar no es un botón: es una dimensión con escala temporal.

## Plataformas

| Fase | Entorno | Requisitos |
|---|---|---|
| 1 | App standalone nativa | Audio en tiempo real, transporte interno, MIDI in/out, edición de patches, render y export, agente |
| 2 | Plugin | Mismo runtime, sincronía con transporte del host, MIDI del host, automatización, persistencia de estado, presets |

El plugin **no** forma parte del primer vertical slice. Su viabilidad se investiga antes de comprometerse (spike S5, ADR-0003).

## Métricas de éxito del MVP

No son métricas de negocio: son criterios para decidir si el instrumento merece seguir.

1. Tres renders de referencia — dron evolutivo, bajo pulsante, textura metálica — que alguien quiera escuchar completos.
2. Una pieza generada de 10 minutos que mantenga identidad rítmica y armónica sin intervención.
3. Un agente que traduzca "hazlo más distante y enorme" en un cambio de patch audible, reversible y explicable.
4. Cero glitches de audio en 30 minutos de ejecución continua a 48 kHz con buffer de 256 frames.
