# 0819 — The licence is Apache-2.0

Session 887. Status: **accepted**. The project owner's decision, recorded rather than argued.

## Context

The project owner, 2026-09-03:

> please switch the license of pdf-viewer and quorra to apache.

Two repositories, one instruction. `quorra`'s half was taken first, in its own tree
(`/home/cl/projects/render-lib`, commit `6043deb`, awaiting the owner's push); this is
`pdf-viewer`'s.

The tree was **MIT** from the hundred-and-thirtieth session and **MPL-2.0** before that. One
author in the whole history, so no relicensing here has ever needed anybody else's consent —
which is the only reason a decision like this is a commit rather than a campaign, and it is
worth writing down because it will stop being true the first time somebody else contributes.

## Decision

**`LICENSE` is the verbatim Apache License, Version 2.0**, taken from a canonical copy on this
machine (`~/.cargo/registry/src/…/thiserror-2.0.19/LICENSE-APACHE`) rather than retyped —
byte-identical to the one `quorra` took, checked with `diff`. Under it, the copyright notice and
the standard *"Licensed under the Apache License, Version 2.0…"* boilerplate naming Christian
Loitsch, 2026. The file keeps its closing paragraph about `NOTICE`, unchanged: that paragraph is
about the third-party *data* under `data/`, which no licence of ours touches.

**The workspace's `license` field is `"Apache-2.0"`**, and every one of the twenty-six member
crates continues to say `license.workspace = true`, so the identifier appears once in the tree
and reaches all of them. Nothing had to be added anywhere.

Everywhere else the tree names its own licence now says Apache-2.0. The whole list, from
`git grep '\bMIT\b'` read hit by hit rather than replaced by machine:

| file | what it said |
|---|---|
| `LICENSE` | the MIT text |
| `Cargo.toml` | `license = "MIT"` |
| `NOTICE` | "This program is MIT-licensed; see LICENSE." |
| `deny.toml` | "This project is MIT, and so is everything it links", and a remark ranking BSL-1.0 against MIT |
| `.github/workflows/ci.yml` | the packaging step's comment on why `LICENSE` and `NOTICE` travel with a binary |
| `doc/third-party-data.md` | "**This project is MIT** as of the hundred-and-thirtieth session"; a dependency remark reading "every one of them is MIT, **which is this project's own licence**"; and a CC BY-SA note about "a verbatim copy beside MIT code" |
| `doc/HAYRO_MERGE.md` | both direction options priced against our MIT — "taking the MIT arm is free", and "our MIT code would need dual-licensing" |

There is no `README.md` at the root; the packaging step is the only place outside `LICENSE` and
`NOTICE` that a distribution's licence is decided, and it copies both files verbatim.

**What was deliberately *not* edited: `doc/adr/` and `doc/history/`.** ADR 0188 says "[t]his
program is MIT and…" and a dozen history files record dependency decisions against the licence of
their day. Those are records of what was true when they were written, and ADR 0232 §2's rule is
that an ADR is not edited to follow a file that moved underneath it. A reader who wants today's
answer reads `LICENSE`; a reader who wants 0188's reasoning wants 0188's premises.

## The dependency graph, and what the move costs it

Nothing, and that is checkable rather than assumed. `deny.toml`'s allow-list already had
`Apache-2.0` as its **first** entry — because dependencies were under it long before we were —
so the policy file needed a comment change and no rule change. Every package in the graph is
MIT, Apache-2.0, BSD-2, BSD-3, ISC, Zlib, Unicode-3.0 or CC0, with two Windows-only packages
under BSL-1.0 named as narrow exceptions; all of those may be combined into an Apache-2.0 work.

**The finding to report rather than fix, and there was none.** No GPL or MPL package is in the
graph. The one GPL item this tree's records name is poppler's `cidToUnicode`, `nameToUnicode` and
`unicodeMap` data, examined in `doc/third-party-data.md`'s table and **not taken** — the
examination is the record, and it is unaffected.

**One obligation is genuinely new, and a file that already existed meets it.** Apache-2.0
section 4(d) obliges a redistribution of a work carrying a `NOTICE` text file to pass that file
on. `/NOTICE` is at the root, `pdf-viewer --licences` prints it, `?` puts it on the screen in all
three windows, and CI copies it beside every packaged binary — all of which the BSD-3-Clause font
programs already required. So the new obligation is met by construction, and `NOTICE` now says so
in a paragraph of its own rather than leaving a reader to work it out.

## Consequences

- The answer to "what may I do with a build of this?" gains Apache-2.0's patent grant and its
  notice requirement, and loses nothing MIT gave.
- `doc/HAYRO_MERGE.md`'s direction analysis is unchanged in its conclusions and changed in one
  premise each way: absorbing `hayro` (`Apache-2.0 OR MIT`) is still licence-clean and now takes
  the *other* arm; `hayro` absorbing us is still licence-costly, because Apache-2.0-only code
  cannot keep their MIT arm intact without the owner dual-licensing.
- `cargo deny check licenses` is the command that keeps the graph honest, and it is unmoved.
