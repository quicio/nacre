# Spec Delta

## Purpose

Define el ciclo de vida del dispositivo de audio, el contrato de tiempo real y la transferencia segura de comandos entre el plano de control y el callback, incluyendo el render offline determinista.

## ADDED Requirements

### Requirement: Apertura y negociación del dispositivo

El runtime SHALL abrir el dispositivo de salida por defecto y solicitar estéreo, `f32`, 48 kHz y un buffer fijo. Si el dispositivo no soporta esa configuración, SHALL seleccionar la configuración soportada más cercana e informar de la diferencia al plano de control.

#### Scenario: Dispositivo compatible

- **WHEN** el dispositivo por defecto soporta 48 kHz, estéreo y `f32`
- **THEN** el runtime abre el stream con esa configuración y reporta `sample_rate = 48000`, `channels = 2`, `buffer_frames = 256`

#### Scenario: Dispositivo sin 48 kHz

- **WHEN** el dispositivo solo soporta 44.1 kHz
- **THEN** el runtime abre el stream a 44.1 kHz, informa `negotiated = true` con la tasa real y el motor ajusta sus coeficientes a esa tasa

#### Scenario: Sin dispositivo de salida

- **WHEN** no hay ningún dispositivo de salida disponible
- **THEN** el runtime devuelve un error estructurado `DeviceUnavailable`, no entra en pánico y la aplicación sigue operativa en modo sin audio

### Requirement: Contrato de tiempo real del callback

El callback de audio MUST completar su trabajo sin asignar memoria, sin adquirir locks, sin realizar I/O, sin compilar o validar el grafo y sin entrar en pánico, con un coste acotado por bloque.

#### Scenario: Bloque procesado sin asignaciones

- **WHEN** el callback procesa 10 000 bloques consecutivos con parámetros cambiando
- **THEN** el contador de asignaciones del allocator instrumentado permanece en cero

#### Scenario: Valor inválido en el camino de audio

- **WHEN** un parámetro llega con un valor NaN o infinito
- **THEN** el callback lo sustituye por un valor seguro y continúa, sin pánico y sin propagar NaN a la salida

### Requirement: Transferencia acotada de comandos

Los cambios de parámetros y de transporte SHALL viajar del plano de control al callback mediante una cola acotada sin bloqueo. El callback SHALL aplicar un número máximo de comandos por bloque y reportar los descartes.

#### Scenario: Ráfaga de cambios

- **WHEN** el plano de control envía 10 000 cambios de frecuencia en un segundo
- **THEN** el callback aplica como máximo el límite configurado por bloque, el audio no se bloquea y el contador de comandos descartados es visible en el estado de salud

#### Scenario: Latencia de aplicación

- **WHEN** el plano de control envía un cambio de ganancia con la cola vacía
- **THEN** el nuevo valor es efectivo en el siguiente bloque procesado

### Requirement: Arranque, parada y reinicio

El runtime SHALL permitir iniciar y detener el audio de forma idempotente, aplicando un fade de salida de al menos 10 ms al detenerse, y SHALL poder reiniciarse sin fugas de hilos ni de recursos.

#### Scenario: Parada sin click

- **WHEN** el usuario detiene la reproducción mientras suena una señal continua
- **THEN** la salida se desvanece en 10–50 ms y no se produce una discontinuidad audible

#### Scenario: Arranque repetido

- **WHEN** se llama a iniciar y detener el audio 20 veces seguidas
- **THEN** no se acumulan hilos, no hay errores de dispositivo ocupado y el vigésimo arranque produce audio

### Requirement: Estado de salud del runtime

El runtime SHALL publicar al plano de control, al menos una vez por segundo, un snapshot con RMS, peak, xruns, comandos descartados y carga de CPU estimada.

#### Scenario: Lectura de estado durante la reproducción

- **WHEN** la aplicación consulta el estado de salud mientras el audio está activo
- **THEN** recibe valores actualizados en menos de 1 segundo, sin bloquear el callback

### Requirement: Render offline determinista

El runtime SHALL renderizar audio a un buffer o archivo WAV sin depender de un dispositivo, usando la misma ruta DSP que la reproducción. Dos renders con los mismos parámetros y la misma semilla MUST producir muestras idénticas.

#### Scenario: Dos renders idénticos

- **WHEN** se renderizan 10 segundos con frecuencia 220 Hz, ganancia −12 dB y la misma configuración
- **THEN** los dos buffers tienen exactamente las mismas muestras y el mismo hash

#### Scenario: Render sin dispositivo

- **WHEN** no hay dispositivo de audio disponible
- **THEN** el render offline se completa correctamente y produce un archivo válido
