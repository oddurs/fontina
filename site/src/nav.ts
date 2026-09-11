// SPDX-License-Identifier: GPL-3.0-or-later
//
// fontina — a font manager.
// Copyright (C) 2026 Oddur Sigurdsson
//
// This program is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this
// program. If not, see <https://www.gnu.org/licenses/>.

// The site's navigation, defined once.
//
// The masthead and the footer sitemap used to be two literals in the layout, listing
// overlapping sets of pages with no relationship between them: the footer's heading said
// "Documentation" where the masthead said "Manual", and either could gain, lose or
// rename a link without the other noticing.
//
// Here the masthead is derived from the sitemap, so it cannot list a page the site index
// does not have — `masthead()` throws on one — and the two can no longer drift apart.
// Where the masthead deliberately words something differently, that is `RENAMED`, one
// visible map of four entries, rather than a coincidence between two literals.

const REPO = 'https://github.com/oddurs/fontina';

/** One destination. `sub` marks a link into a section of another page. */
export type Link = {
  label: string;
  href: string;
  sub?: boolean;
};

/** A heading and the links under it, as the footer sets them. */
export type Group = {
  head: string;
  links: Link[];
};

/**
 * The whole site index, which the footer prints in full.
 *
 * `at` is a function of the base path rather than a constant because the site is served
 * from `/` locally and `/fontina/` on Pages, and every internal href has to carry it.
 */
export function sitemap(at: (path: string) => string): Group[] {
  return [
    {
      head: 'About fontina',
      links: [
        { label: 'Mission', href: at('/mission/') },
        { label: 'Releases', href: at('/releases/') },
        { label: 'News', href: at('/news/') },
        { label: 'Decisions', href: at('/adr/') },
        { label: 'Contributors', href: `${REPO}/graphs/contributors` },
      ],
    },
    {
      head: 'Manual',
      links: [
        { label: 'Contents', href: at('/docs/') },
        { label: 'Command reference', href: at('/docs/cli/') },
        { label: 'FAQ', href: at('/faq/') },
      ],
    },
    {
      head: 'Download',
      links: [
        { label: 'Binaries', href: at('/download/') },
        { label: 'Verifying', href: at('/download/#verifying-a-download'), sub: true },
        { label: 'From source', href: at('/download/#building-from-source'), sub: true },
      ],
    },
    {
      head: 'Sources',
      links: [
        { label: 'Git', href: REPO },
        { label: 'Changelog', href: `${REPO}/blob/main/CHANGELOG.md` },
        { label: 'License', href: at('/license/') },
      ],
    },
    {
      head: 'Development',
      links: [
        { label: 'Roadmap', href: at('/roadmap/') },
        { label: 'Contributing', href: at('/contributing/') },
        { label: 'Web style', href: at('/style/') },
      ],
    },
    {
      head: 'Bugs',
      links: [
        { label: 'How to report', href: at('/bugs/') },
        { label: 'Bug tracker', href: `${REPO}/issues` },
        { label: 'Security', href: at('/security/') },
      ],
    },
  ];
}

/**
 * The five the masthead carries, named by the group and label they have below.
 *
 * Five, and the design note in the stylesheet argues for keeping it that way: at this
 * size a wrapped row beats a disclosure, and a disclosure needs either script or a hack
 * that fights the user agent stylesheet. The footer is the site index; this is the
 * shortlist.
 *
 * A page the shortlist does not contain lights nothing here, and that is right rather
 * than a gap to paper over. `/faq/` is grouped under Manual below and was tempting to
 * cover from the Manual entry, but it does not live under `/docs/`, so the underline
 * would be pointing at a section the reader is not in. Every page is marked in the
 * footer sitemap instead, including the ones below a section index.
 */
const MASTHEAD: [group: string, label: string][] = [
  ['Manual', 'Contents'],
  ['Download', 'Binaries'],
  ['About fontina', 'News'],
  ['Sources', 'License'],
  ['Sources', 'Git'],
];

/** What the masthead should read, where it differs from the sitemap's wording. */
const RENAMED: Record<string, string> = {
  Contents: 'Manual',
  Binaries: 'Download',
  License: 'Free software',
  Git: 'Source',
};

/**
 * The masthead's destinations, taken from the sitemap.
 *
 * Throws if an entry names something the sitemap does not have, which is the point: a
 * page renamed or moved in the sitemap fails the build here rather than leaving a link
 * in the masthead that goes nowhere.
 */
export function masthead(at: (path: string) => string): Link[] {
  const index = sitemap(at);
  return MASTHEAD.map(([head, label]) => {
    const group = index.find((g) => g.head === head);
    const link = group?.links.find((l) => l.label === label);
    if (!link) {
      throw new Error(
        `the masthead asks for "${label}" under "${head}", and the sitemap has no such link. ` +
          `Fix MASTHEAD in site/src/nav.ts, or put the link back.`,
      );
    }
    return { ...link, label: RENAMED[link.label] ?? link.label };
  });
}

/**
 * Is `href` the page being looked at, or the section it belongs to?
 *
 * The old test was `href === Astro.url.pathname`, which is true on a section's own index
 * page and nowhere else. So a reader on `/docs/cli/` — eleven screens into the command
 * reference — saw nothing lit in the masthead at all, and the same went for every
 * decision record, every news post, and every chapter of the manual. The one thing
 * navigation is for is saying where you are, and it did it on six pages out of
 * thirty-eight.
 *
 * A section counts as current when the path is at or below it, matched on a slash so
 * that `/docs/` does not light for a hypothetical `/docs-old/`.
 *
 * Two things never match. The site root would otherwise be an ancestor of every page.
 * And a link to a fragment of another page is a shortcut rather than a destination:
 * lighting `Verifying` and `From source` alongside `Binaries` on `/download/` would
 * mark three current pages, which is one more than there can be.
 *
 * This answers "does this link cover the page", which several links in one list can.
 * `currentHref` is what picks the one to mark.
 */
export function isCurrent(href: string, pathname: string, root: string): boolean {
  if (/^[a-z]+:/i.test(href) || href.includes('#')) return false;

  const here = withSlash(pathname);
  const there = withSlash(href);

  if (there === withSlash(root)) return here === there;
  return here === there || here.startsWith(there);
}

/** A path ending in exactly one slash, so prefix tests land on segment boundaries. */
function withSlash(path: string): string {
  return path.endsWith('/') ? path : `${path}/`;
}

/**
 * Which one link in a list is the current page.
 *
 * The footer lists both `/docs/` and `/docs/cli/`, and on the command reference both
 * cover the page — so marking every link that matched put `aria-current="page"` on two
 * of them, and `aria-current` is singular. The most specific wins, which is the longest
 * href, because a longer path is a deeper one.
 *
 * A tie cannot arise between a longer and a shorter path. It can only arise from the
 * same href listed twice, and marking both of those is right.
 */
export function currentHref(
  hrefs: readonly string[],
  pathname: string,
  root: string,
): string | undefined {
  return hrefs
    .filter((href) => isCurrent(href, pathname, root))
    .sort((a, b) => b.length - a.length)[0];
}
