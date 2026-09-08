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

// The releases, read from the repository's own tags at build time.
//
// The site used to name the current version in three places, and all three were stale
// within a day of the first release. A page that says which version is current should
// find that out rather than be told, and the tags are already right there: the deploy
// workflow checks out full history so that page footers can show a real last-modified
// date, and this uses the same checkout.
import { execSync } from 'node:child_process';

export interface Release {
  /** The tag, `v0.1.1`. */
  tag: string;
  /** The version without its `v`, `0.1.1`. */
  version: string;
  /** ISO date the tag was made, `2026-09-04`. */
  date: string;
  url: string;
}

const REPO = 'https://github.com/oddurs/fontina';

function fromGit(): Release[] {
  // Newest first, by version rather than by date, so a patch to an old series does not
  // claim to be current.
  const out = execSync(
    'git for-each-ref --sort=-v:refname --format="%(refname:short) %(creatordate:short)" refs/tags',
    { cwd: '..', encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] },
  );
  return out
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^v\d+\.\d+\.\d+ \d{4}-\d{2}-\d{2}$/.test(line))
    .map((line) => {
      const [tag, date] = line.split(' ');
      return { tag, version: tag.slice(1), date, url: `${REPO}/releases/tag/${tag}` };
    });
}

/** Every tagged release, newest first. Empty when the checkout has no tags. */
export const releases: Release[] = (() => {
  try {
    return fromGit();
  } catch {
    return [];
  }
})();

/** The current release, or `undefined` before the first one. */
export const current: Release | undefined = releases[0];
