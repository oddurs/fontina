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
const adr = defineCollection({
  loader: glob({ pattern: '[0-9][0-9][0-9][0-9]-*.md', base: '../docs/adr' }),
  schema: z.object({}),
});

export const collections = { docs, news, adr };
