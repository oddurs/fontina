import type { APIContext } from 'astro';

// Generated rather than a static file in public/, so the sitemap URL follows SITE_URL
// and SITE_BASE, which the deploy workflow sets. A hardcoded absolute URL would be
// wrong for every deployment but the default one.
export function GET(context: APIContext) {
  const base = import.meta.env.BASE_URL.replace(/\/$/, '');
  const sitemap = new URL(`${base}/sitemap-index.xml`, context.site).href;
  return new Response(`User-agent: *\nAllow: /\n\nSitemap: ${sitemap}\n`, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
}
