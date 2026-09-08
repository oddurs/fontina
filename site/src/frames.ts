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

// Frames of the browser, read from the snapshots a test in the CLI asserts on.
//
// `fontina ui` draws text, so a picture of it is that text. Every frame on this site
// comes from `crates/fontina-cli/src/ui/snapshots/`, which means none of them can go
// stale without a test going red first — and it means the site never shows a frame the
// program does not produce. If a page wants to show something the browser does, the way
// to get it is a test in the CLI that pins it.
//
// Paths are resolved from the working directory, which is site/ for both `npm run build`
// and the deploy workflow, the same assumption Base.astro makes for `git log`.
// .github/workflows/site.yml lists the snapshots directory, so a new or moved snapshot
// rebuilds the site.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SNAPS = 'crates/fontina-cli/src/ui/snapshots';

/** One frame, by the name of the test that asserts it. */
export function frame(name: string): string {
  const path = resolve(process.cwd(), `../${SNAPS}/fontina__ui__tests__${name}.snap`);
  try {
    // insta writes a YAML header, then `---`, then the value.
    return readFileSync(path, 'utf8')
      .split(/^---$/m)
      .slice(2)
      .join('---')
      .replace(/^\n/, '')
      .trimEnd();
  } catch {
    throw new Error(
      `a page asks for the browser frame at ${path}, which is missing. Add a snapshot test for it in crates/fontina-cli/src/ui/, or fix the name; if the snapshot moved, update this path and the one in .github/workflows/site.yml.`,
    );
  }
}
