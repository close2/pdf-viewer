# ADR 0007 — Substituting fonts a document does not embed

Status: accepted, 2026-07-26. The standard-14 metrics question, left open in the first
draft, is now **resolved** — see the closing section.

## Context

A PDF may name a font without carrying it. Until now `pdf-font` reported those as
`NotEmbedded` and the text was not drawn, which is correct behaviour for a *harness* and
useless behaviour for a viewer. Every corpus document was affected: eight of the fourteen
reference a standard-14 `/Helvetica` with no descriptor and no `/Widths`.

Substitution is where a renderer most easily becomes machine-dependent without anyone
noticing. The shapes must come from whatever is installed — there is no alternative — but
if the *metrics* come from there too, then the same document lays out differently on
different machines, and the difference is invisible until someone compares two screens.

## Decision

Split the problem in two, and keep the halves apart in the code.

`substitute::Request` is derived from the document alone: the `/BaseFont` name with its
subset prefix stripped and its style suffixes parsed, then the `/FontDescriptor`'s
`/Flags`, `/ItalicAngle` and `/FontWeight` where the name says nothing. The same PDF
produces the same request everywhere. `substitute::find` then resolves that request
against this machine, and returns `None` — reported as `FontError::NoSubstitute` — rather
than inventing something when nothing fits.

**Metrics come from the document whenever it states them.** `/Widths` and `/W` are applied
whatever substitute is chosen, so glyphs land where the producer positioned them and line
breaks fall where the producer put them, even though the shapes differ. This is the
property worth protecting: a substituted page with the document's own metrics is correctly
laid out and merely looks different, whereas one with the substitute's metrics drifts.

Composite fonts are substituted through `/ToUnicode` rather than through their CIDs. A CID
indexes the glyphs of the font that defined it and means nothing in any other font, so the
only honest route from a code to a substitute's glyph is the character the code stands for.
A composite font with no `/ToUnicode` is therefore refused rather than guessed at.

## Consequences

Every corpus document now renders its page one with nothing reported unsupported except
shadings. The `/Helvetica` footers that were previously blank are drawn.

Discovery walks the platform font directories behind a `OnceLock`, so a document with no
missing fonts never pays for it — `CLAUDE.md` forbids that work on the launch path. The
walk is bounded in depth and file count, does not follow symlinks, and costs about 1.9 ms
the first time it is needed.

`substitute::PREFERENCES` puts the metric-compatible clones of the standard 14 first —
Nimbus, Liberation, Croscore — before generic faces, so that a substitute's *shapes* are
as close as the machine allows. The ordinary faces after them exist so that text in roughly
the right shape beats a blank page.

## Resolved: standard-14 metrics come from pdf.js

A standard-14 font may omit `/Widths` entirely, because a conforming reader is assumed to
know the metrics. Taking them from the substitute instead — the first version of this
decision — made a page's *layout* depend on which fonts the machine had installed, which
is the same class of defect as depending on the input in the wrong way.

The metrics are now compiled in, generated from `doc/pdf.js/src/core/metrics.js` by
`tools/gen-standard-metrics.py` into `crates/pdf-font/src/standard_metrics.rs`.

Why that source, and not the obvious one:

- The **fonts themselves** — Helvetica, Times, Courier, Symbol, Zapf Dingbats — are
  proprietary, owned by Monotype, Linotype, ITC and Adobe. They were never redistributable
  and are not what is wanted here.
- The **URW metric clones** installed on most Linux systems reproduce those advances by
  design, and their AFM files sit in `/usr/share/fonts`. They are **AGPL-3.0 with a font
  exception**, and that exception covers *embedding a font in a document*, not lifting its
  data into an unrelated program. Deriving the table from them would put a copyleft
  obligation on this crate, so they were deliberately not used.
- **pdf.js is Apache-2.0**, and Mozilla ships exactly this table for exactly this purpose.

A test cross-checks the result against a second, independent source: the URW clones may not
be *copied* from, but nothing stops reading them at test time, and 36 advances across four
faces agree. Two independently drawn implementations of one specification.

Advances are therefore resolved in descending order of authority: `/Widths` when the
document states it, the standard metrics when it does not, and the substitute's own
advances only for glyphs outside the standard character set. A substituted page now has the
document's layout everywhere it can.

## What this changed elsewhere

Wiring substitution required text extraction, since composite substitution runs through
`/ToUnicode`. That in turn made the `pdftotext` comparison metric cheap to add — and that
metric immediately found a defect neither the renderer nor any existing test could see: the
content interpreter silently dropped operands past the 64th, which truncated any `TJ` array
holding a full justified line. Three sentences on the specification's own title page ended
mid-word. The bound is now 8192 and reaching it is reported. See
`crates/pdf-model/tests/text_extraction.rs`.
