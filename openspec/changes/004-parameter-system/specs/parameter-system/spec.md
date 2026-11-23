# Spec Delta

## Purpose

Define un sistema único de parámetros: identidad estable, especificación con unidad y rango, validación estricta, normalización con curvas, suavizado, transferencia segura al audio y snapshot consistente para la interfaz.

## ADDED Requirements

### Requirement: Identidad y especificación del parámetro

Cada parámetro SHALL tener un identificador estable y legible, y una especificación con unidad, rango, valor por defecto, curva y tiempo de suavizado. La especificación MUST ser válida antes de que el parámetro pueda usarse.

#### Scenario: Parámetro válido

- **WHEN** se registra `filter.cutoff` con rango 20–20 000 Hz, default 1 000 Hz y curva logarítmica
- **THEN** el parámetro aparece en la tabla con esos valores y su valor inicial es 1 000 Hz

#### Scenario: Especificación inválida

- **WHEN** se registra un parámetro con rango mínimo mayor que el máximo o con default fuera de rango
- **THEN** el registro falla con `ParameterError::InvalidSpec` y el parámetro no se añade

#### Scenario: Identificador duplicado

- **WHEN** se registra dos veces el mismo identificador
- **THEN** la segunda operación falla con `ParameterError::DuplicateId` y el valor existente no cambia

### Requirement: Validación estricta de valores

El sistema SHALL aceptar solo valores dentro del rango de la especificación y SHALL rechazar el resto con un error estructurado, sin corregirlos ni aplicarlos parcialmente.

#### Scenario: Valor fuera de rango

- **WHEN** se intenta fijar `filter.cutoff` a 30 000 Hz
- **THEN** la operación devuelve `ParameterError::OutOfRange`, el valor anterior se conserva y el audio no cambia

#### Scenario: Valor no finito

- **WHEN** se intenta fijar un parámetro a NaN o infinito
- **THEN** la operación se rechaza con `ParameterError::NotFinite` y no llega ningún comando al audio

### Requirement: Normalización con curvas

El sistema SHALL convertir entre valor en unidad y valor normalizado 0..1 según la curva del parámetro, y la conversión inversa MUST recuperar el valor original dentro de una tolerancia de 1e-5 en el rango completo.

#### Scenario: Curva logarítmica

- **WHEN** se normaliza 1 000 Hz en un rango 20–20 000 Hz con curva logarítmica
- **THEN** el valor normalizado corresponde a la posición logarítmica y al desnormalizar se recupera 1 000 Hz ± 0.01 Hz

#### Scenario: Extremos

- **WHEN** se normaliza el mínimo y el máximo del rango
- **THEN** se obtiene 0.0 y 1.0 respectivamente, y la vuelta devuelve exactamente el mínimo y el máximo

### Requirement: Suavizado configurable

Cada parámetro SHALL declarar un tiempo de suavizado entre 0 y 100 ms. Los parámetros con suavizado mayor que cero MUST interpolar sus cambios de forma continua; los discretos MUST aplicarse en una frontera de bloque.

#### Scenario: Cambio continuo sin zipper noise

- **WHEN** la ganancia maestra cambia de −60 dB a 0 dB en 10 ms mientras suena una señal
- **THEN** la amplitud evoluciona de forma monótona y no se detectan escalones audibles ni discontinuidades

#### Scenario: Parámetro discreto

- **WHEN** cambia la forma de onda de sine a saw
- **THEN** el cambio se aplica al inicio del bloque siguiente y no a mitad de bloque

### Requirement: Transferencia al audio con coalescing

Los cambios SHALL llegar al callback mediante la cola acotada existente. Si varios cambios del mismo parámetro están pendientes, el callback SHALL aplicar el más reciente y descartar los intermedios, sin bloquear y con latencia máxima de dos bloques.

#### Scenario: Ráfaga sobre un parámetro

- **WHEN** llegan 500 cambios de cutoff antes de un bloque
- **THEN** el callback aplica el último valor, el estado final es ese valor y no se bloquea

#### Scenario: Latencia máxima

- **WHEN** se envía un único cambio con la cola vacía
- **THEN** el valor es efectivo en el bloque actual o en el siguiente

### Requirement: Snapshot consistente para la interfaz

El sistema SHALL exponer un snapshot de solo lectura con los valores actuales y normalizados de todos los parámetros, consistente y sin bloquear el callback.

#### Scenario: Lectura durante cambios

- **WHEN** la UI lee el snapshot mientras el audio recibe cambios
- **THEN** obtiene valores de una misma generación, sin mezclar valores nuevos y viejos del mismo parámetro

#### Scenario: Parámetro inexistente

- **WHEN** la UI pide un parámetro que no está en la tabla
- **THEN** recibe `None` o un error consultable y no se produce pánico

### Requirement: Reproducibilidad del estado final

Aplicar la misma secuencia de cambios SHALL producir el mismo estado final de parámetros y el mismo audio en un render offline, independientemente de cómo se agrupen los comandos por bloque.

#### Scenario: Dos secuencias equivalentes

- **WHEN** se aplican 1 000 cambios agrupados de forma distinta pero con el mismo estado final y los mismos tiempos
- **THEN** el render offline produce el mismo hash
