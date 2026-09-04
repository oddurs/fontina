import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';

// The manual. One file per chapter; `order` sets the reading order.
const docs = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/docs' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    order: z.number(),
  }),
});

// Announcements. The file name is the slug; `date` orders them and feeds the RSS.
const news = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/news' }),
  schema: z.object({
    title: z.string(),
    date: z.coerce.date(),
  }),
});

// Architecture decision records, read straight from the repository so there is one
// copy. The files carry their own heading and status line; nothing is required here.
// Records link each other by file name (0007-license-gpl-3.md); on the site each
// record is a directory, so those links are rewritten to ../<slug>/ before rendering.
const adr = defineCollection({
  loader: {
    name: 'adr',
    load: async ({ store, renderMarkdown, logger }) => {
      const dir = new URL('../../docs/adr/', import.meta.url);
      const { readdir, readFile } = await import('node:fs/promises');
      store.clear();
      for (const name of (await readdir(dir)).sort()) {
        if (!/^\d{4}-.*\.md$/.test(name)) continue;
        const id = name.replace(/\.md$/, '');
        const raw = await readFile(new URL(name, dir), 'utf8');
        const body = raw.replace(/\]\((?:\.\/)?(\d{4}-[a-z0-9-]+)\.md(#[^)]*)?\)/g, '](../$1/$2)');
        store.set({ id, data: {}, body, rendered: await renderMarkdown(body), filePath: `docs/adr/${name}` });
      }
      logger.info(`loaded ${store.keys().length} decision records`);
    },
  },
  schema: z.object({}),
});

export const collections = { docs, news, adr };
