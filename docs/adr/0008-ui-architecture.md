# ADR-0008 — Arquitectura de UI

**Estado:** Proposed (se valida en VS2 y S6)
**Fecha:** 2026-11-22

## Contexto

La UI debe servir a dos presentaciones futuras (standalone y plugin) sin duplicar lógica ni acoplar el núcleo a un framework. También debe permitir una experiencia simple (macros, intención musical) y una experta (grafo, parámetros).

Datos verificados (2026-11-22): `egui`/`eframe` 0.37, licencia MIT OR Apache-2.0, MSRV 1.95, muy activo.

## Decisión

1. **egui/eframe** para el standalone del MVP.
2. La UI **no contiene DSP ni lógica de negocio**: emite comandos tipados a la aplicación y lee view models/snapshots.
3. **Dos niveles de interfaz**:
   - *Nivel 1 (musical)*: macros, intención, escenas, transporte.
   - *Nivel 2 (experto)*: parámetros crudos, grafo, modulación.
4. La UI se escribe contra un **view model** propio (`nacre-ui` o el módulo de presentación de la app), no contra tipos de `egui` ni del dominio.
5. Para el plugin, el editor se implementará con `nih_plug_egui` (o la alternativa que S5 valide) sobre el mismo view model. El núcleo no cambia.
6. Sin node editor complejo en el MVP. El grafo se muestra como lista/estructura editable, no como lienzo de cables.
7. El estado de UI (zoom, paneles, selección) no forma parte del patch ni de la composition.

## Alternativas consideradas

| Alternativa | Evaluación |
|---|---|
| egui/eframe | Elegida: rápido de iterar, licencia permisiva, integrable en nih-plug, buen soporte de texto/gráficos |
| Iced | Buena arquitectura, pero más ceremonia y menos integración directa con el ecosistema de audio |
| Vizia | Pensada para plugins, pero menor adopción y más riesgo de mantenimiento |
| UI web (Tauri/Electron) | Peso, latencia percibida y complejidad de empaquetado; no para un instrumento de audio |
| UI nativa por plataforma | Coste triplicado; descartado |

## Consecuencias

**Positivas**
- Iteración rápida y una sola base de código de UI.
- Integración razonable con el host de plugins mediante `nih_plug_egui`.

**Negativas**
- egui no es una UI nativa: accesibilidad y look&feel limitados.
- Repintado continuo puede consumir CPU; hay que controlar el modo de repintado.
- La UI del plugin tiene restricciones de ciclo de vida y foco que hay que validar.

## Riesgos

| Riesgo | Mitigación |
|---|---|
| CPU de UI compite con el audio | Repintado bajo demanda; medir en S6 |
| Acoplamiento de la lógica a `egui` | View models + comandos; revisión de dependencias en CI |
| UI del plugin inestable en algunos hosts | Spike S5; fallback a controles del host |
| Complejidad visual excesiva | Nivel 1 simple por defecto; nivel 2 oculto |

## Validación

- VS2: una ventana con Play/Stop y ganancia funciona sin que `nacre-application` importe `egui`.
- S6: consumo de CPU de la UI < 3 % de un núcleo con la ventana visible y sin interacción.
- S5: editor `nih_plug_egui` funcional en dos hosts.
