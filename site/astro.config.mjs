// @ts-check
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';

// SITE_URL and SITE_BASE are set by the deploy workflow. Locally the site is served
// from the root of localhost.
const site = process.env.SITE_URL ?? 'https://oddurs.github.io';
const base = process.env.SITE_BASE ?? '/';


export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  build: { format: 'directory', inlineStylesheets: 'always' },
  integrations: [sitemap()],
  markdown: {
    shikiConfig: { theme: 'github-light' },
  },
});
