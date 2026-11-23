---
name: NACRE
description: Instrumento musical generativo en Rust — capas de nácar, precisión de carta científica.
colors:
  obsidian: "#050607"
  graphite: "#101214"
  graphite-2: "#171C1F"
  mineral: "#232A2D"
  pearl: "#E9E4D8"
  pearl-dim: "#BDB8AB"
  pearl-mute: "#9B978C"
  line: "rgba(233, 228, 216, 0.14)"
  line-strong: "rgba(233, 228, 216, 0.28)"
  iridescent-violet: "#B3A6CF"
  mineral-sea: "#93A89A"
  warm-amber: "#C8A878"
typography:
  display:
    fontFamily: "Bodoni Moda, Times New Roman, serif"
    fontSize: "clamp(2.4rem, 5.2vw, 5.2rem)"
    fontWeight: 500
    lineHeight: 1.02
    letterSpacing: "-0.03em"
  section-title:
    fontFamily: "Bodoni Moda, Times New Roman, serif"
    fontSize: "clamp(2.1rem, 4.6vw, 4rem)"
    fontWeight: 400
    lineHeight: 0.98
    letterSpacing: "-0.015em"
  body:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: "0em"
  label:
    fontFamily: "Azeret Mono, ui-monospace, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0.12em"
rounded:
  none: "0px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "32px"
  xl: "64px"
  section: "clamp(5rem, 11vw, 9.5rem)"
components:
  button-primary:
    backgroundColor: "transparent"
    textColor: "{colors.pearl}"
    borderColor: "{colors.pearl}"
    typography: "{typography.label}"
    rounded: "{rounded.none}"
    padding: "0 18px"
    height: "52px"
  button-primary-hover:
    backgroundColor: "{colors.graphite-2}"
    textColor: "{colors.pearl}"
  plate:
    backgroundColor: "{colors.graphite}"
    borderColor: "{colors.line}"
    rounded: "{rounded.none}"
  requirement-card:
    backgroundColor: "{colors.graphite}"
    borderColor: "{colors.line}"
    rounded: "{rounded.none}"
    padding: "32px"
---

# NACRE — sistema visual

## Overview

La identidad parte del nácar: capas sucesivas de crecimiento, iridiscencia mineral y estructura oculta. La web no ilustra el instrumento, lo **habita**: una placa de nácar ocupa la derecha del primer viewport y el texto vive en el negro quieto de la izquierda, con anotaciones técnicas propias de una carta científica.

Principios: material antes que decoración; precisión tipográfica; un solo gesto dominante por pantalla; el color iridiscente aparece en el material, nunca como neón de interfaz.

Modo oscuro por defecto. El claro no está implementado; si se añade, invierte los neutros y conserva el acento violeta solo para estados.

## Colors

Paleta de neutros minerales con un acento iridiscente restringido.

| Token | Valor | Uso |
|---|---|---|
| `obsidian` | `#050607` | Fondo de página, texto sobre superficies claras |
| `graphite` | `#101214` | Superficies de sección, tarjetas |
| `graphite-2` | `#171C1F` | Hover de botones, capas elevadas |
| `mineral` | `#232A2D` | Scrollbar, bordes gruesos |
| `pearl` | `#E9E4D8` | Texto principal, wordmark, bordes de acción |
| `pearl-dim` | `#BDB8AB` | Cuerpo secundario, descripciones |
| `pearl-mute` | `#9B978C` | Metadatos, etiquetas (nunca por debajo de 11 px) |
| `line` | `rgba(233,228,216,.14)` | Reglas de 1 px |
| `iridescent-violet` | `#B3A6CF` | Estado activo, palabras clave WHEN, foco de marca |
| `mineral-sea` | `#93A89A` | THEN, acentos de sistema |
| `warm-amber` | `#C8A878` | Reservado; aún sin uso en la web |

Reglas: el violeta no se usa como fondo; el texto sobre obsidiana siempre es `pearl` o `pearl-dim`; los estados hover de botones oscurecen o aclaran el borde, nunca invierten a fondo claro con texto claro.

## Typography

Tres voces, tres trabajos.

- **Display — Bodoni Moda 500.** Titulares y wordmark. Mayúsculas, tracking negativo (-0.02 a -0.03em), line-height ≥ 1.02 para que los acentos de las mayúsculas españolas no queden recortados (lección de esta implementación).
- **Cuerpo — Archivo 400/500.** Prosa y descripciones. Medida máxima 62–65ch; line-height 1.6.
- **Datos — Azeret Mono 400/500.** Navegación, etiquetas, coordenadas, cifras, pasos de milestone. Siempre en mayúsculas cortas con tracking 0.12–0.18em. Nunca se usa como "disfraz técnico" para prosa.

Escala: display `clamp(2.4rem, 5.2vw, 5.2rem)`; títulos de sección `clamp(2.1rem, 4.6vw, 4rem)`; cuerpo 1rem; etiquetas 0.75rem (suelo de 12 px). La jerarquía no depende del color, sino de tamaño y familia.

## Layout

Rejilla de 12 columnas con contenedor máximo de 1560 px y gutter fluido (`clamp(1.25rem, 4vw, 3.5rem)`).

- **Hero**: copy en la mitad izquierda, placa a sangre en la derecha; el texto nunca pisa la zona brillante. En móvil la placa pasa a fondo completo con un degradado vertical que garantiza contraste.
- **Secciones**: cabecera a dos columnas (título grande + lede a la derecha), cuerpo a ancho completo con reglas de 1 px.
- **Roadmap**: filas acordeón; M0 abierto por defecto, el resto colapsado. Nunca tarjetas repetidas.
- **Especificaciones**: cifras a la izquierda, requisito de ejemplo a la derecha.
- **Ritmo**: `--section-y` `clamp(5rem, 11vw, 9.5rem)`; más aire encima de un título que debajo.

## Elevation & Depth

Sin sombras. La profundidad la producen:

1. El material fotográfico del nácar (placa del hero y del cierre).
2. Planos de color (`obsidian` → `graphite` → `graphite-2`).
3. Reglas de 1 px que separan planos.
4. Degradados de oscurecimiento para legibilidad sobre imagen, nunca como adorno.

Prohibido: glassmorphism, halos, sombras difusas de tarjeta, `box-shadow` de offset duro.

## Shapes

Geometría recta. Radio 0 en todos los componentes. Los únicos círculos son:

- el símbolo (elipses concéntricas irregulares que representan capas de nácar),
- los puntos de la línea de tiempo,
- la diana de anotación sobre la placa.

Sin `clip-path` orgánicos, sin blobs, sin esquinas redondeadas.

## Components

- **Logo (lockup)**: símbolo topográfico + wordmark NACRE. Se usa siempre desde `site/src/assets/brand/nacre-logo-light.svg` (perla, fondos oscuros) o `nacre-logo-ink.svg` (obsidiana, fondos claros). Nunca se recompone con texto: el wordmark va vectorizado.
  - Altura de uso: 42 px en cabecera de escritorio, 32 px en móvil, 20 px en pie. Por debajo de 24 px usar el símbolo simplificado (`nacre-symbol-small.svg`), no el lockup.
  - Área de respeto: la altura de la «N» alrededor del lockup; nada entra en ese margen.
- **Símbolo**: `nacre-symbol-light.svg` / `nacre-symbol-ink.svg`, 560×325. Legible desde 48 px; por debajo, usar la versión simplificada de 4 anillos.
- **Favicon**: `site/public/favicon.svg`, símbolo simplificado sobre obsidiana, trazo grueso para 16 px.
- **Wordmark**: NACRE en Bodoni Moda 500 con tracking 0.26em; el logo vectorizado es la fuente de verdad, no el texto en vivo.
- **Botón primario**: rectángulo de 1 px `pearl`, altura 52 px, etiqueta mono en mayúsculas, flecha dibujada. Hover: fondo `graphite-2`, borde `pearl`.
- **Botón fantasma**: mismo cuerpo, borde `line-strong`, texto `pearl-dim`. Hover: borde `pearl`.
- **Placa**: figura con borde `line`, imagen 16:9, etiqueta "Concepto" superpuesta, pie de foto en `pearl-dim`. Clic para abrir a tamaño completo.
- **Acordeón de milestone**: `<details>` nativo; marcador `summary` oculto; estado "En curso" en violeta, "No iniciado" en `pearl-mute`.
- **Requisito**: tarjeta con borde, etiqueta mono, título display, cuerpo, escenarios WHEN/THEN en dos columnas con palabras clave en violeta y verde mar.
- **Etiquetas**: mono 12 px mínimo, tracking 0.12em, color `pearl-mute`; nunca por debajo del suelo de legibilidad.
- **Secuenciador de emergencia**: canvas de 9 capas × 16 pasos, con patrones euclidianos, pulso de barrido y eventos encendidos. Hover sobre una capa resalta su fila y la lectura pasa a `CAPA 05/09 · 2 de 16 pulsos`; sin hover muestra `PASO 12/16 · 3 eventos`. Es una visualización del mecanismo, no audio real.

## Do's and Don'ts

**Do**
- Usa los SVG del logo tal como están; el lockup es la marca, no una composición tipográfica.
- Deja que el nácar sea el único gesto dominante por pantalla.
- Usa reglas de 1 px y espacio en blanco para separar, no sombras.
- Escribe cifras reales (27 requisitos, 56 escenarios, 8 ADR) y marca el estado real (M0).
- Etiqueta toda imagen de interfaz como "Concepto" y enlázala a tamaño completo.
- Mantén el texto funcional ≥ 12 px y el contraste ≥ 4.5:1.

**Don't**
- No uses formas de onda, ecualizadores, notas musicales ni HUD futurista.
- No uses degradados morado-azul, neón, glow ni glassmorphism.
- No conviertas el nácar en fondo decorativo detrás de todo: pierde su función de material.
- No inventes capturas de la app ni métricas de rendimiento.
- No uses mayúsculas acentuadas con `line-height` menor que 1: el acento se recorta.
- No pongas kickers o eyebrows sobre los títulos; el título carga su propio peso.
