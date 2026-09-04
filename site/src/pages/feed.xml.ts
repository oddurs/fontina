import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import type { APIContext } from 'astro';

export async function GET(context: APIContext) {
  const news = (await getCollection('news')).sort((a, b) => b.data.date.valueOf() - a.data.date.valueOf());
  const base = import.meta.env.BASE_URL.replace(/\/$/, '');
  return rss({
    title: 'unifont news',
    description: 'Announcements and release notes for unifont, a free software font manager.',
    site: context.site!,
    items: news.map((n) => ({
      title: n.data.title,
      pubDate: n.data.date,
      link: `${base}/news/${n.id}/`,
      description: n.body ?? '',
    })),
    customData: '<language>en</language>',
  });
}
