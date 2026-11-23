# ADR-0003 — Estrategia standalone y plugin

**Estado:** Proposed (hipótesis; validar con spike S5 antes de comprometerse)
**Fecha:** 2026-11-22

## Contexto

El producto debe existir como aplicación standalone (Fase 1) y como plugin (Fase 2). El mismo runtime debe servir a ambos, pero los frameworks de plugin imponen restricciones de licencia, de UI, de transporte y de empaquetado que no deben filtrarse al núcleo.

Datos verificados (2026-11-22):

- `nih-plug`: licencia **ISC**, activo (último push 2026-09-03), ~3k estrellas, no se publica en crates.io (se usa desde Git).
- Exporta **VST3 y CLAP**; soporta automatización sample-accurate en ambos, MIDI y transporte JACK.
- Editores: `nih_plug_egui`, `nih_plug_iced`, `nih_plug_vizia`.
- **Los bindings VST3 (`vst3-sys`) son GPLv3**: un plugin VST3 construido con nih-plug debe cumplir GPLv3. CLAP no arrastra esa obligación.
- Probado en Linux y Windows; soporte macOS limitado.

## Decisión

1. **Fase 1: standalone primero.** El plugin no entra en el MVP ni en el primer vertical slice.
2. El núcleo (`domain`, `application`, `dsp`) no conoce `nih-plug` ni ningún SDK de plugin. El adaptador de plugin vivirá en `apps/nacre-plugin` (o crate equivalente) cuando exista.
3. El runtime de audio se diseña con los requisitos del host desde el principio: transporte externo, automatización, cambios de estado, sample rate variable.
4. **CLAP como primer target de plugin**, y VST3 solo si la licencia del producto es compatible con GPLv3 o si se adopta una estrategia de licencia dual. Esta es una decisión de negocio, no solo técnica.
5. La UI se diseña para ser reutilizable entre standalone y plugin (ADR-0008), sin acoplarse a `eframe` en el núcleo.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| `nih-plug` | Elegida como candidata: VST3+CLAP, buen soporte de automatización, licencia ISC (con la excepción GPLv3 de los bindings VST3), activo |
| `clap-wrapper` / `vst3-sys` directo | Más control, mucho más trabajo; se puede considerar si nih-plug no cubre un requisito |
| Framework propio de plugins | Coste enorme y fuera de foco; rechazado |
| Solo CLAP | Menor cobertura de hosts hoy, pero sin la carga GPLv3; riesgo comercial a evaluar |
| Solo VST3 | Cobertura amplia, pero obliga a GPLv3 con los bindings actuales |

## Consecuencias

**Positivas**
- El standalone se entrega antes y valida el sonido sin la complejidad del host.
- La decisión de licencia se toma con información, no por inercia.

**Negativas**
- Dos targets de empaquetado y dos ciclos de test (standalone/host).
- La UI debe abstraerse lo suficiente para no depender de `eframe` en el plugin.
- `nih-plug` desde Git implica fijar un commit/tag y auditar actualizaciones.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| GPLv3 por VST3 incompatible con producto propietario | CLAP primero; decisión de licencia antes de VS16; asesoría legal si hace falta |
| macOS con soporte limitado en nih-plug | Spike S5 en macOS antes de prometer soporte |
| La UI de egui no funciona igual dentro del host | Prototipo de editor en S5; fallback a UI propia del host |
| Transporte del host distinto al interno | El reloj se abstrae desde VS2; tests con transporte externo simulado |
| Estado del plugin y persistencia distinta del patch standalone | Un solo modelo de estado serializable, versionado |

## Validación (spike S5, antes de VS16)

1. Compilar un plugin mínimo (ganancia + seno) como CLAP y VST3 con `nih-plug` en Linux.
2. Cargarlo en al menos dos hosts (p. ej. Reaper y Bitwig) y verificar audio, automatización y transporte.
3. Probar un editor `nih_plug_egui` y medir problemas de foco/repintado.
4. Documentar limitaciones reales y actualizar este ADR a `Accepted` o `Rejected`.

**Nota:** este ADR no autoriza implementar el plugin. Solo fija la estrategia y las condiciones para decidir.
