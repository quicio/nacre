# Sitio de NACRE

Sitio público del proyecto: Astro estático, bilingüe (ES por defecto, EN en `/en/`), desplegado en GitHub Pages.

## Desarrollo

```bash
npm install
npm run dev      # http://localhost:4321/nacre/
npm run build    # site/dist
npm run check    # tipos + diagnóstico
```

Con `BASE_PATH=/` el sitio se sirve desde la raíz (útil para previsualizar sin el prefijo del repo):

```bash
BASE_PATH=/ npm run dev
```

## Estructura

```text
site/
├── astro.config.mjs        site + base configurables por entorno
├── public/                 favicon.svg, og.png
├── src/
│   ├── assets/             placas de nácar y maquetas conceptuales
│   ├── components/         Page.astro (composición), Symbol.astro (marca)
│   ├── content/site.ts     copy ES/EN, milestones y cifras reales
│   ├── layouts/Base.astro  head, fuentes, metadatos, motion
│   ├── pages/              index.astro (es), en/index.astro (en)
│   └── styles/global.css   tokens y sistema visual
└── dist/                   salida estática (no versionada)
```

## Despliegue

El workflow `.github/workflows/pages.yml` construye y publica en cada push a `main` que toque `site/`. Requiere:

1. Repositorio en GitHub con Pages configurado en **Settings → Pages → Source: GitHub Actions**.
2. `SITE_URL` y `BASE_PATH` correctos en el workflow (por defecto `https://hugomoraga.github.io` y `/nacre`).

URL esperada: `https://hugomoraga.github.io/nacre/`.

## Contenido

- Las cifras (27 requisitos, 56 escenarios, 8 ADR) salen de `openspec/changes/` y deben actualizarse a mano al archivar cambios.
- Las maquetas de interfaz son **concepto**: no hay binario. Están etiquetadas y enlazadas a tamaño completo.
- El estado del roadmap (M0 en curso) debe reflejar el estado real del repositorio.
