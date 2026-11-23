# Spec Delta

## Purpose

Define una voz monofónica expresiva y estable: oscilador saw con anti-aliasing, envolvente ADSR, filtro resonante, LFO, saturación y protecciones de señal, con parámetros acotados y comportamiento determinista.

## ADDED Requirements

### Requirement: Oscilador saw anti-aliased

La voz SHALL generar una onda saw cuya energía de alias en el rango audible quede al menos 60 dB por debajo de la fundamental, para frecuencias de 20 Hz a 2 000 Hz a 48 kHz.

#### Scenario: Saw a 1 kHz

- **WHEN** la voz suena a 1 kHz con el filtro abierto y sin saturación
- **THEN** el análisis espectral muestra componentes de alias por debajo de −60 dBFS respecto a la fundamental

#### Scenario: Cambio de frecuencia

- **WHEN** la frecuencia cambia de 110 Hz a 440 Hz durante la reproducción
- **THEN** la forma de onda se actualiza sin discontinuidades mayores que 0.05 ni clicks

### Requirement: Envolvente ADSR

La envolvente SHALL aplicar ataques, decaos, sostenimiento y liberación configurables, con tiempos entre 1 ms y 10 s, sin clicks en el ataque, la liberación ni el retrigger.

#### Scenario: Ataque y liberación

- **WHEN** se dispara una nota con ataque de 10 ms y liberación de 200 ms
- **THEN** la amplitud sube y baja de forma monótona en esos tiempos, sin picos ni discontinuidades

#### Scenario: Retrigger durante la liberación

- **WHEN** se dispara una nueva nota mientras la anterior está en liberación
- **THEN** la envolvente reinicia desde su valor actual (no desde cero) y no se produce click

#### Scenario: Sostenimiento

- **WHEN** la nota se mantiene con sustain 0.6
- **THEN** la amplitud se estabiliza en 0.6 ± 0.01 y no decae con el tiempo

### Requirement: Filtro paso-bajo resonante

El filtro SHALL atenuar 12 dB/octava por encima de la cutoff, con cutoff entre 20 Hz y 20 kHz y resonancia entre 0.5 y 20, permaneciendo estable y sin NaN ni infinitos en todo el rango.

#### Scenario: Barrido de cutoff

- **WHEN** la cutoff barre de 20 Hz a 20 kHz en 2 segundos con Q = 15
- **THEN** la salida permanece finita, no hay autooscilación no deseada y no se producen clicks

#### Scenario: Resonancia alta

- **WHEN** la resonancia se configura a 20 con una saw a 110 Hz
- **THEN** el pico de resonancia es audible y medible, y la salida no supera el límite de headroom tras la etapa de ganancia

### Requirement: LFO de modulación

La voz SHALL incluir un LFO con forma seno o triángulo, frecuencia entre 0.01 Hz y 20 Hz, y profundidad acotada, capaz de modular cutoff y pitch sin introducir discontinuidades.

#### Scenario: Modulación de cutoff

- **WHEN** el LFO modula la cutoff con profundidad del 50 % a 0.5 Hz
- **THEN** la cutoff oscila de forma continua y audible, sin saltos ni clicks

#### Scenario: LFO a frecuencia máxima

- **WHEN** el LFO se configura a 20 Hz
- **THEN** la modulación es estable, no produce alias audible y no desestabiliza el filtro

### Requirement: Saturación suave

La etapa de saturación SHALL añadir armónicos de forma controlada, con drive entre 0 y 24 dB, compensación de ganancia y salida acotada, sin introducir componentes por encima del límite de Nyquist ni DC apreciable.

#### Scenario: Saturación moderada

- **WHEN** el drive se configura a 12 dB sobre una señal de −12 dBFS
- **THEN** la salida aumenta su contenido armónico, permanece por debajo de 0 dBFS y no se produce aliasing audible

#### Scenario: Drive máximo

- **WHEN** el drive se configura a 24 dB
- **THEN** la salida sigue acotada, no hay NaN y el nivel percibido no aumenta de forma descontrolada

### Requirement: Protecciones de señal

La voz SHALL mantener su salida finita y libre de DC, sustituyendo NaN/Inf por cero y aplicando un DC blocker en la etapa final.

#### Scenario: Entrada corrupta

- **WHEN** un parámetro llega como NaN o infinito
- **THEN** la voz usa un valor seguro, la salida sigue siendo finita y el audio continúa

#### Scenario: Offset DC

- **WHEN** la voz suena 10 segundos con una saw asimétrica
- **THEN** el offset DC medido en la salida es menor que 0.001
