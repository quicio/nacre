# Spec Delta

## Purpose

Define el arranque de la aplicación standalone, sus controles mínimos de transporte y ganancia, el comportamiento cuando no hay audio disponible y el cierre limpio.

## ADDED Requirements

### Requirement: Arranque de la aplicación

La aplicación SHALL arrancar y mostrar su interfaz aunque falle la inicialización de audio, indicando el estado del dispositivo y permitiendo operar en modo sin audio.

#### Scenario: Arranque con audio disponible

- **WHEN** el usuario abre la aplicación y hay un dispositivo de salida
- **THEN** la ventana muestra el estado "audio listo" con la tasa de muestreo y el tamaño de bloque efectivos

#### Scenario: Arranque sin dispositivo

- **WHEN** no hay dispositivo de salida disponible
- **THEN** la aplicación se abre, muestra el error de forma legible y no se cierra ni entra en pánico

### Requirement: Controles de transporte

La aplicación SHALL ofrecer controles de reproducción y parada que reflejen el estado real del runtime, sin permitir estados inconsistentes.

#### Scenario: Reproducir

- **WHEN** el usuario pulsa Play con audio disponible
- **THEN** el runtime arranca, el botón refleja el estado "reproduciendo" y se produce sonido

#### Scenario: Parar

- **WHEN** el usuario pulsa Stop mientras reproduce
- **THEN** la salida se desvanece, el runtime se detiene y el botón vuelve al estado "detenido"

#### Scenario: Play sin dispositivo

- **WHEN** el usuario pulsa Play sin dispositivo de audio
- **THEN** la acción se rechaza con un mensaje visible y el estado sigue siendo "detenido"

### Requirement: Control de ganancia maestra

La aplicación SHALL exponer una ganancia maestra entre −60 dB y 0 dB, aplicada sin clicks ni zipper noise.

#### Scenario: Cambio de ganancia en reproducción

- **WHEN** el usuario mueve el control de ganancia mientras suena el oscilador
- **THEN** el nivel cambia de forma suave, sin escalones audibles ni discontinuidades

#### Scenario: Silencio

- **WHEN** la ganancia se sitúa a −60 dB
- **THEN** la salida es prácticamente inaudible y el control sigue respondiendo

### Requirement: Cierre limpio

Al cerrarse la ventana, la aplicación SHALL detener el audio, liberar los recursos y terminar el proceso sin hilos colgados ni errores en el cierre.

#### Scenario: Cierre durante la reproducción

- **WHEN** el usuario cierra la ventana mientras el audio está activo
- **THEN** el stream se detiene, no se emite audio después del cierre y el proceso termina con código 0

#### Scenario: Cierre sin audio inicializado

- **WHEN** el usuario cierra la ventana en modo sin audio
- **THEN** el proceso termina con código 0 y sin mensajes de error de hilos
