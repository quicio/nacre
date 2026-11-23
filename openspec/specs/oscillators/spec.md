# oscillators Specification

## Purpose

Define la generación de una señal seno controlable, con precisión de frecuencia, continuidad de fase y salida acotada, como primera unidad DSP del instrumento.

## Requirements

### Requirement: Precisión de frecuencia

El oscilador SHALL generar una seno cuya frecuencia medida esté dentro de ±1 cent de la frecuencia solicitada para el rango de 20 Hz a 5 000 Hz, a cualquier sample rate soportado entre 44.1 kHz y 192 kHz.

#### Scenario: Tono de referencia

- **WHEN** se configura el oscilador a 440 Hz y se renderizan 10 segundos a 48 kHz
- **THEN** el análisis espectral sitúa el pico dentro de ±1 cent de 440 Hz

#### Scenario: Frecuencia baja

- **WHEN** se configura el oscilador a 20 Hz
- **THEN** el pico espectral está dentro de ±1 cent de 20 Hz y no hay offset DC apreciable

### Requirement: Continuidad de fase

El oscilador SHALL mantener la continuidad de fase cuando cambia la frecuencia, sin reiniciar la fase ni producir discontinuidades mayores que el límite de click acordado.

#### Scenario: Barrido de frecuencia

- **WHEN** la frecuencia cambia de 220 Hz a 880 Hz durante la reproducción
- **THEN** no se produce una discontinuidad mayor que 0.01 en la señal de salida ni un click audible

#### Scenario: Reinicio explícito

- **WHEN** se reinicia el oscilador manualmente
- **THEN** la fase vuelve a cero en el siguiente bloque y el reinicio se aplica solo en la frontera del bloque

### Requirement: Nivel y silencio

La salida del oscilador SHALL permanecer en el rango −1.0 a 1.0 y SHALL producir silencio digital exacto cuando la ganancia es cero.

#### Scenario: Ganancia cero

- **WHEN** la ganancia se configura a 0
- **THEN** todas las muestras de salida son exactamente 0.0 y no queda residuo DC

#### Scenario: Ganancia máxima

- **WHEN** la ganancia se configura a 1.0 con frecuencia 1 kHz
- **THEN** el pico de salida no excede 1.0 y no se introduce distorsión por clipping

### Requirement: Validación de parámetros del oscilador

El oscilador SHALL rechazar frecuencias fuera de 0.1 Hz–20 000 Hz y ganancias fuera de 0.0–1.0, devolviendo un error de dominio antes de llegar al audio.

#### Scenario: Frecuencia inválida

- **WHEN** se solicita una frecuencia de 0 Hz o negativa
- **THEN** la operación devuelve `ParameterError::OutOfRange` y el oscilador mantiene su valor anterior

#### Scenario: Ganancia inválida

- **WHEN** se solicita una ganancia de 1.5
- **THEN** la operación se rechaza y el audio no cambia
