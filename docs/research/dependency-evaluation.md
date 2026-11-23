# Evaluación de dependencias

> Datos consultados el **2026-11-22** en crates.io y la API de GitHub. Las cifras cambian: reverificar antes de fijar versiones.

## Resumen

| Crate | Versión | Licencia | Último release | Estado | Uso previsto |
|---|---|---|---|---|---|
| `cpal` | 0.18.2 | Apache-2.0 | 2026-08-16 | Muy activo (push 2026-11-21) | Backend de audio (ADR-0002) |
| `nih-plug` | Git (sin release en crates.io) | ISC; bindings VST3 GPLv3 | — | Activo (push 2026-09-03) | Plugin Fase 2 (ADR-0003) |
| `fundsp` | 0.23.0 | MIT OR Apache-2.0 | 2026-01-07 | Moderado | Referencia de tests; no base del motor |
| `egui` / `eframe` | 0.37.0 | MIT OR Apache-2.0 | 2026-10-15 | Muy activo | UI standalone (ADR-0008) |
| `midir` | 0.11.1 | MIT | 2026-10-10 | Activo | MIDI (M4) |
| `rtrb` | 0.4.1 | MIT OR Apache-2.0 | 2026-10-25 | Activo | Ring buffer SPSC para audio |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | 2026-07-18 | Muy activo | Persistencia (DTOs) |
| `crossbeam-channel` | 0.5.17 | MIT OR Apache-2.0 | 2026-09-05 | Muy activo | Canales de control (nunca en audio) |
| `wmidi` | 4.0.11 | MIT | 2026-04-19 | Moderado | Mensajes MIDI (alternativa a midir para tipos) |

Descargas totales (referencia de adopción, no de calidad): `cpal` 30.2M, `egui` 32.8M, `fundsp` 283k, `rtrb` 16.2M.

## Notas por dependencia

### `cpal` 0.18.2

- **Licencia Apache-2.0** (cambió respecto a versiones históricas MIT; verificar si se necesita compatibilidad con proyectos GPL/ MIT-only).
- Linux: ALSA es obligatoria incluso usando JACK/PipeWire/PulseAudio; requiere `alsa-lib` de desarrollo.
- Features relevantes: `pipewire` (nativo, PipeWire ≥ 0.3.53), `jack`, `pulseaudio`, `realtime` (SCHED_FIFO, requiere CAP_SYS_NICE/rtprio), `realtime-dbus` (rtkit).
- Riesgo documentado: `BufferSize::Default` puede ser enorme o irregular; **fijar buffer explícito**.
- Riesgo documentado: desajuste de sample rate con PipeWire causa resampling y glitches; **coincidir con el clock del servidor**.
- MSRV 1.85.

### `nih-plug`

- No está en crates.io: se consume desde Git. Implica fijar tag/commit y auditar actualizaciones (sipply-chain).
- Exporta **VST3 y CLAP**; automatización sample-accurate en ambos; MIDI y transporte JACK.
- Editores: `nih_plug_egui`, `nih_plug_iced`, `nih_plug_vizia`.
- **Licencia:** ISC para el framework, pero `vst3-sys` (usado por `nih_export_vst3!`) es **GPLv3**. Un plugin VST3 distribuido debe cumplir GPLv3. CLAP no arrastra esa obligación.
- Probado en Linux/Windows; macOS con pruebas limitadas.
- Conclusión: candidato viable para CLAP; VST3 condicionado a la licencia del producto.

### `fundsp` 0.23.0

- Licencia permisiva (MIT OR Apache-2.0), API funcional de DSP.
- Última release 2026-01-07: mantenimiento moderado, no abandonado pero tampoco rápido.
- Se usa como **referencia en tests** y fuente de ideas; no como base del grafo propio (ADR-0004). Evita acoplar el motor a su modelo de tipos.

### `egui` / `eframe` 0.37

- MSRV **1.95** (superior a la de cpal); fija el toolchain mínimo del workspace en ≥ 1.95. Rust disponible: 1.100.
- Muy activo, licencia permisiva.
- Riesgo: repintado continuo consume CPU; controlar con repaint bajo demanda.
- Integración en plugin mediante `nih_plug_egui` (validar en S5).

### `rtrb` 0.4.1

- Ring buffer SPSC sin locks, pensado para tiempo real. Mantenido y ampliamente usado.
- Adecuado para comandos y eventos; **no** para estado grande. Para snapshots, triple buffer o `ArcSwap` (evaluar `arc-swap` cuando se implemente).

### `midir` 0.11.1

- MIDI I/O multiplataforma, licencia MIT, activo. Se usará en M4; en el plugin, el MIDI lo provee el host.
- Alternativa: `wmidi` para tipos de mensajes si `midir` no los cubre.

### `serde` 1.0.229

- Solo en `nacre-persistence`, nunca en `nacre-domain` (ADR-0001).
- Formatos: JSON para legibilidad y depuración; evaluar binario (postcard/CBOR) si el tamaño o la velocidad importan.

## Dependencias descartadas (por ahora)

| Crate | Motivo |
|---|---|
| `rodio` | Alto nivel, menos control del callback; no para el motor |
| `bevy` / `bevy_ecs` | Modelo de mundo ajeno al dominio; peso y curva de aprendizaje |
| `symphonia` | Decodificación de audio para samples; no necesaria hasta M4 |
| `cpal` + `jack` directo en el núcleo | Acoplaría el dominio al backend |
| `hound` | Escritura WAV: se puede implementar en ~100 líneas; evaluar si aparece una necesidad mayor |
| `rubato` | Resampling; no necesario hasta samples/wavetables |

## Reglas para añadir dependencias

1. Justificar en el PR: problema, alternativas, licencia, mantenimiento, impacto en RT.
2. Verificar licencia contra la del producto (pendiente de decisión).
3. Verificar MSRV y plataformas objetivo.
4. Aislar en un crate adaptador si es de infraestructura.
5. Añadir a `Cargo.toml` con versión fijada (no `*`), y actualizar este documento.
6. `cargo deny check` en CI para licencias, duplicados y vulnerabilidades.

## Licencia del producto (pendiente)

No decidida. Restricciones conocidas:

- `cpal` Apache-2.0, `egui` MIT/Apache-2.0, `fundsp` MIT/Apache-2.0, `midir` MIT, `rtrb` MIT/Apache-2.0: compatibles con MIT, Apache-2.0 o propietaria.
- `nih-plug` ISC: compatible.
- Bindings VST3 GPLv3: **incompatibles con un plugin VST3 propietario** salvo licencia dual o cumplimiento GPLv3.
- Decisión pendiente: licencia del producto y, en consecuencia, formato de plugin prioritario (CLAP primero si se quiere propietario).

**Acción:** resolver antes de publicar el repositorio o de distribuir binarios.
