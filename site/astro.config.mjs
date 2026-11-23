import { defineConfig } from 'astro/config';

// GitHub Pages: https://<user>.github.io/<repo>/
// Override with SITE_URL and BASE_PATH in CI or locally.
export default defineConfig({
  site: process.env.SITE_URL || 'https://hugomoraga.github.io',
  base: process.env.BASE_PATH || '/nacre',
  trailingSlash: 'ignore',
  build: {
    format: 'directory',
    inlineStylesheets: 'auto',
  },
  vite: {
    build: {
      assetsInlineLimit: 0,
    },
  },
});
