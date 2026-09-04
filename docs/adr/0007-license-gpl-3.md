# 0007 — Project license: GPL-3.0-or-later

**Status:** accepted, 2026-09-04. Supersedes [ADR 0004](0004-license-mit-or-apache.md).

## Context
ADR 0004 chose `MIT OR Apache-2.0` for one reason: it is the Rust ecosystem convention.
That reason is real but it is about fitting in, not about what the license does.

What it does is this. `PLAN.md` §2 names the competition: FontBase — Electron, closed
source, 300 MB of RAM, paid tiers. Under MIT, FontBase can vendor `fontina-core`, ship
the parsing, the index, the health checks and the specimen inside a proprietary paid
tier, and owe their users nothing: no source, no right to modify, no right to pass it
on. Every hour spent on this project would become unpaid labour for a program that
denies its users the freedoms this one is built to protect. A permissive license does
not merely allow that outcome; against an incumbent with a proprietary product already
shipping, it invites it.

The project's own principles already answer the question. Principle 2 is that your data
stays yours in formats you can read. Principle 3 is that nothing leaves the machine.
Principle 9 says "free software". Those are promises about the user's control over
their own computer, and a license that lets an intermediary strip them is not the
license those promises call for.

The counter-argument for permissive licensing of a *library* is adoption: if a free
alternative is already entrenched, a copyleft library will simply be ignored in its
favour, and the copyleft achieves nothing. That argument is why the FSF recommends the
LGPL for a small number of libraries. It does not apply here. There is no entrenched
free font-metadata crate that `fontina-core` must dislodge; the crate's own reason to
exist (`PLAN.md` §2) is that no such library exists. Nothing is won by giving away the
copyleft in advance.

## Decision
Every crate in the workspace, and the CLI, is licensed **GPL-3.0-or-later**. The full
text is in `COPYING`; each source file carries the standard notice.

Contributions are accepted under the same terms. There is no CLA and no copyright
assignment: contributors keep their copyright, which is what makes the license hard for
any single party — including this project's maintainers — to revoke later.

Documentation (`README.md`, `PLAN.md`, `docs/`, the man page and the Texinfo manual) is
under the **GNU Free Documentation License 1.3 or later**, no invariant sections, no
front- or back-cover texts. Its text is in `docs/COPYING.DOC`.

Fixture fonts keep their own licenses and are listed in `fixtures/README.md`.

## Consequences
- The patent grant that motivated Apache-2.0 in ADR 0004 is retained: GPLv3 §11 grants
  it, and unlike Apache-2.0 it is compatible with GPLv2 code through the "or later".
- Dependencies must be GPLv3-compatible. `deny.toml` enforces this, and its allow-list
  is the place that argument gets settled. Permissive dependencies remain fine —
  permissive code can be taken into a GPL work; it is the other direction that is now
  closed.
- A proprietary font manager can no longer take this code. A free one can, on the
  condition that it stays free. That condition is the point.
- Anyone who wants `fontina-core` inside a proprietary product must write their own, as
  they would have had to before this project existed.

## Alternatives considered
- **LGPL for `fontina-core`, GPL for the CLI.** The usual split, and the honest reason to
  want it is adoption. Rejected for the reason above: no entrenched free competitor makes
  the trade necessary, and it would hand the incumbent exactly the piece worth taking.
  Revisit only if a specific, named free project is demonstrably blocked by the GPL.
- **AGPL-3.0.** Aimed at network services. A font manager runs on the user's own machine;
  the loophole the AGPL closes is not one this program can be put through.
- **Staying with `MIT OR Apache-2.0`.** Considered and rejected above. Note that this
  direction is one-way: relicensing from permissive to copyleft is possible now, while
  the contributor list is short. It gets harder with every contributor.
