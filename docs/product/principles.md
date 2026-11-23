# Principios

Estos principios se usan para **decidir**, no para decorar. Cuando dos opciones son válidas, gana la que respeta más principios; cuando un principio entra en conflicto con otro, se documenta la tensión en un ADR.

## Producto

### 1. El sonido va primero
Un instrumento con cinco módulos que suena excelente vale más que uno con cincuenta que suena genérico. Ningún milestone se declara terminado si los renders de referencia no convencen. Si el sonido no está a la altura en el gate de M1, se rediseña el DSP antes de añadir features.

### 2. Generativo por diseño
La variación no es un efecto: es una dimensión del sistema. Todo parámetro generativo tiene escala temporal, rango y semilla. La evolución debe preservar identidad; el azar puro es un fallo de diseño.

### 3. Modular pero coherente
Los componentes se conectan por relaciones musicales significativas (audio, control, eventos), no por un lienzo infinito. Un patch debe poder explicarse en una frase.

### 4. Agent-native, no agent-dependent
El agente es una capacidad de primera clase, pero el instrumento funciona completo sin él. La API que usa el agente es la misma que usa la UI: si el agente puede hacer algo que la UI no, hay un agujero en el diseño.

### 5. Humano-dirigido
Toda autonomía es inspeccionable, acotada y reversible. El agente propone con transacciones; el humano aprueba. Nunca hay un botón que "hace algo" sin explicación.

### 6. Determinismo cuando importa
Semilla + estado + parámetros deben reproducir exactamente el mismo audio offline. La reproducibilidad es requisito de test, no una promesa de marketing.

### 7. Independencia del runtime
El núcleo no conoce la UI, el host de plugins, el dispositivo de audio ni el proveedor de IA. Cambiar de egui a otra UI, o de Ollama a un modelo remoto, no debe tocar el dominio ni el DSP.

### 8. Complejidad progresiva
El usuario principiante obtiene un resultado musical sin ver el grafo. El usuario avanzado puede abrirlo, pero el sistema no lo obliga a empezar por ahí.

### 9. Semántica musical antes que parámetros DSP
La interfaz y la API exponen densidad, tensión, movimiento, atmósfera, inestabilidad. Los parámetros crudos existen, pero son el nivel experto, no la puerta de entrada.

## Arquitectura

### 10. Dependencias hacia adentro
Presentation → Application → Domain. Infrastructure implementa puertos. El dominio no importa nada de fuera. Cualquier violación es un bug arquitectónico, no un atajo.

### 11. El callback de audio no es un caso de uso
Es un contrato de tiempo real: sin allocación, sin locks, sin I/O, sin pánicos, sin compilación de grafo. Todo lo demás ocurre fuera.

### 12. Tipos explícitos, APIs pequeñas
Newtypes para identidades y unidades. Sin `u64` desnudo ni `f32` sin unidad en fronteras públicas. Un trait solo existe cuando hay un segundo implementador real o una frontera de test clara.

### 13. Nada de abstracciones para futuros hipotéticos
No se crea un sistema de plugins interno, un bus de eventos genérico ni un framework de nodos "por si acaso". Se extrae cuando hay dos casos de uso reales.

### 14. La incertidumbre se documenta
Si una decisión no está validada, se marca `Hypothesis` y se propone un spike con criterio de salida. La arquitectura no debe ocultar riesgos detrás de capas.

### 15. Un cambio, una spec
Cada incremento tiene propósito, alcance, no-alcance y criterios de aceptación verificables. Los cambios grandes se parten; los cambios sin spec no se implementan.

## Evaluación de propuestas

Antes de aceptar una feature o un cambio arquitectónico, responder:

1. ¿Mejora el sonido o la musicalidad? ¿Cómo se escucha?
2. ¿Añade una dependencia? ¿Está mantenida, licenciada de forma compatible y es compatible con tiempo real?
3. ¿Rompe la dirección de dependencias? ¿Puede hacerse sin romperla?
4. ¿Es reversible? ¿Qué cuesta deshacerlo?
5. ¿Se puede verificar con un test, un render o una escucha?
6. ¿Estamos resolviendo un problema real de hoy o uno imaginario de dentro de dos años?
