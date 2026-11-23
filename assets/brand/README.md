# Kit de marca — NACRE

Fuente canónica del logo. El sitio importa copias desde `site/src/assets/brand/`; tras cambiar un SVG aquí, ejecuta `npm run brand:sync` en `site/`.

## Archivos

| Archivo | Uso |
|---|---|
| `nacre-logo.svg` | Lockup completo con `currentColor`. Para SVG inline o herramientas que heredan color. |
| `nacre-logo-light.svg` | Lockup en perla `#E9E4D8`, para fondos oscuros (`<img>`). |
| `nacre-logo-ink.svg` | Lockup en obsidiana `#050607`, para fondos claros. |
| `nacre-symbol.svg` | Símbolo con `currentColor`. |
| `nacre-symbol-light.svg` / `-ink.svg` | Símbolo en perla / obsidiana. |
| `nacre-symbol-small.svg` | Marca simplificada (2 anillos, trazo grueso) para 16–24 px y favicon. |
| `nacre-logo.png` | Lockup rasterizado, 1960×325, blanco con alfa. Para documentos y avatares. |
| `nacre-logo-source.png` | PNG original entregado, sin procesar. Conserva halo y ruido de fondo: no usar en producción. |

## Reglas

- Altura mínima del lockup: 24 px. Por debajo, usar `nacre-symbol-small.svg`.
- Área de respeto: la altura de la «N» alrededor del lockup.
- No recomponer el wordmark con texto: los SVG son la fuente de verdad.
- No aplicar degradados, sombras ni contornos al logo.

## Procedencia

PNG entregado por el autor del proyecto (2026-11-22). Limpiado con umbral de alfa, mediana y apertura; vectorizado con potrace 1.16 (`--flat --turdsize 15 --alphamax 1.0 --opttolerance 0.2`). Los path se normalizaron a `currentColor` y se generaron las variantes por sustitución de relleno.
