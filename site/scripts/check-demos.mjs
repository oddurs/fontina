// The demos on the front page, checked against themselves.
//
// Three things here break without anything going red: a typing animation whose step
// count and string length disagree loses its last characters; a slideshow frame with no
// keyframe delay never appears; and a class the reduced-motion block forgets keeps
// animating for a reader who asked it not to. All three are silent — the page still
// builds, still deploys, and still looks nearly right.
//
// `npm run check:demos` after a build, and in the site workflow. Plain Node against the
// built HTML and the stylesheet: no framework, no browser, and it reads what ships
// rather than what the source implies.
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('../dist/index.html', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/styles/site.css', import.meta.url), 'utf8');

const problems = [];
const check = (ok, message) => {
  if (!ok) problems.push(message);
};

// ---------------------------------------------------------------- typing
// Every `.type` span animates its width to a character count. The count and the string
// have to agree exactly: one short and the last character never arrives, one long and
// the cursor sits in empty space. DESIGN.md says to measure the string rather than count
// it; this is that measurement.
const typed = [...html.matchAll(/class="type (t\d+)">([^<]*)</g)];
check(typed.length > 0, 'no typing animation found on the front page: has the demo lost its `.type` spans?');
for (const [, cls, text] of typed) {
  const rule = css.match(new RegExp(`\\.demo \\.${cls} \\{ animation: type-(\\d+) `));
  check(rule !== null, `the page types \`${text}\` as .${cls} and the stylesheet has no rule for it`);
  if (!rule) continue;
  const steps = Number(rule[1]);
  const chars = [...text].length;
  check(
    steps === chars,
    `.${cls} types ${chars} characters and animates ${steps} steps: \`${text}\`. ` +
      'The last characters would never appear. Measure the string; do not count it.',
  );
  check(
    css.includes(`@keyframes type-${steps} { to { width: ${steps}ch; } }`),
    `@keyframes type-${steps} is missing or does not animate to ${steps}ch`,
  );
}

// ------------------------------------------------------------- slideshow
// One frame, one caption, one delay each, or a frame nobody ever sees.
const frames = [...html.matchAll(/class="term frame f(\d+)"/g)].map((m) => Number(m[1]));
const captions = [...html.matchAll(/class="caption c(\d+)"/g)].map((m) => Number(m[1]));
check(frames.length > 0, 'no browser frames on the front page');
check(
  frames.length === captions.length,
  `${frames.length} frame(s) and ${captions.length} caption(s): the strip narrates a session, so every frame needs its own line`,
);
for (const n of frames) {
  check(
    new RegExp(`\\.frames \\.f${n} \\{ animation-delay:`).test(css),
    `frame f${n} is on the page with no animation-delay: it would never come up`,
  );
  check(
    new RegExp(`\\.captions \\.c${n} \\{ animation-delay:`).test(css),
    `caption c${n} has no animation-delay`,
  );
}

// -------------------------------------------------- reduced motion covers it
// Motion is a preference. Anything that animates has to be answered for in the
// reduced-motion block, or a reader who asked for stillness gets it anyway.
const reduced = css.slice(css.indexOf('@media (prefers-reduced-motion: reduce)'));
check(reduced.length > 0, 'there is no prefers-reduced-motion block');
for (const [, cls] of typed) {
  check(
    reduced.includes('.demo .type'),
    'the reduced-motion block does not neutralise `.demo .type`',
  );
  void cls;
}
for (const n of frames) {
  check(
    reduced.includes(`.f${n}`),
    `the reduced-motion block says nothing about frame f${n}: it would keep cross-fading`,
  );
  check(reduced.includes(`.c${n}`), `the reduced-motion block says nothing about caption c${n}`);
}
for (const [, cls] of [...html.matchAll(/class="cursor (k\d+)"/g)]) {
  check(
    reduced.includes(`.${cls}`),
    `the reduced-motion block says nothing about cursor ${cls}: it would keep blinking`,
  );
}
// One cursor has to survive, or the settled frame ends without a prompt.
check(
  /\.demo \.k\d+ \{ visibility: visible; animation: none; \}/.test(reduced),
  'with motion off, no cursor is left visible: the transcript ends without a prompt',
);

if (problems.length > 0) {
  console.error(`the demos on the front page have ${problems.length} problem(s):\n`);
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}
console.log(`demos: ${typed.length} typed line(s), ${frames.length} frame(s), all consistent`);
