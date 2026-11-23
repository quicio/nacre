# Tasks — parameter system

> Prerrequisito: change 003 archivado. La migración no debe cambiar el sonido de los renders de referencia: se comparan hashes antes y después.

## 1. Dominio

- [ ] 1.1 Implementar `ParameterId`, `Unit`, `Curve`, `ParameterKind` y `ParameterSpec` con validación de especificación.
  - Verificación: tests de spec válida, rango invertido, default fuera de rango y curva log con mínimo 0.
- [ ] 1.2 Implementar `ParameterValue` en unidad y `ParameterError` (`OutOfRange`, `NotFinite`, `UnknownId`, `DuplicateId`, `InvalidSpec`).
  - Verificación: tests de `Display` y de igualdad.
- [ ] 1.3 Implementar normalización y desnormalización para las tres curvas, con extremos exactos.
  - Verificación: round-trip < 1e-5 en 1 000 valores aleatorios por curva.

## 2. Aplicación

- [ ] 2.1 Implementar `ParameterTable` con asignación de slots, límite de 128 y consulta por id.
  - Verificación: test de duplicado, de desbordamiento de tabla y de lookup O(1).
- [ ] 2.2 Implementar `SetParameter` (validación + envío) y `ListParameters` (tabla para UI/agente).
  - Verificación: tests con doble de `AudioHostPort`; un valor inválido no envía comando.
- [ ] 2.3 Implementar `GetSnapshot` con lectura del triple buffer.
  - Verificación: test de snapshot con generación consistente.

## 3. DSP y runtime

- [ ] 3.1 Implementar `Smoother` one-pole con tiempo en ms, congelación y flush a cero.
  - Verificación: test de tiempo de establecimiento (±10 %) y de monotonía.
- [ ] 3.2 Sustituir los comandos ad-hoc por `Set { slot, value }` en la cola y en el callback.
  - Verificación: tests de 002/003 siguen pasando; 0 asignaciones en 10 000 bloques.
- [ ] 3.3 Implementar coalescing por slot con lote máximo de 64 comandos.
  - Verificación: 500 cambios → se aplica el último; sin bloqueo; contador de descartes correcto.
- [ ] 3.4 Implementar el triple buffer de snapshot (escritura en audio, lectura en control).
  - Verificación: test de concurrencia sin tearing durante 1 M de lecturas.
- [ ] 3.5 Migrar los parámetros de la voz y del master a la tabla común.
  - Verificación: hash de los renders de referencia idéntico al de 003 (o diferencia documentada y justificada).

## 4. UI y cierre

- [ ] 4.1 Generar los controles de la UI desde `ParameterSpec` (label, rango, unidad, curva, default).
  - Verificación: todos los parámetros de la tabla aparecen; no hay controles huérfanos.
- [ ] 4.2 Añadir el snapshot a la UI con repintado bajo demanda.
  - Verificación: los controles reflejan cambios externos (p. ej. desde un test) sin bloquear.
- [ ] 4.3 Ejecutar la suite completa, clippy, fmt, deny, y actualizar `HANDOFF.md`.
  - Verificación: todo en verde; mediciones y decisiones registradas.

## Fuera de alcance

- Automatización del host, MIDI CC, macros, modulación, persistencia de presets.
- Editor de curvas por el usuario.
- Parámetros de estructura del patch (nodos/conexiones).
