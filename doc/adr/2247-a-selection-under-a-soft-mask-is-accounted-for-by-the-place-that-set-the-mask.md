# 2247 — A selection under a soft mask is accounted for by the place that set the mask

Session 1706, the archive slot of batch one hundred and nine. Status: **accepted**, **built** in
`crates/pdf-archive/tests/cross_check.rs`. It **extends ADR 1055**'s scope rules with an account of
the one construct A61 put outside them. Context: `doc/questions/A61`, `doc/history/1698` (the
thirteen selections "unjudged"), `doc/history/1706`.

## What A61 settles

A61 holds the survey and the interpreter to each other within **the constructs the survey walks**.
The survey does not walk a soft mask's group (`survey.rs`'s header: under-report rather than
mis-report), so a resource the interpreter selects inside one is a thing the survey decided not to
judge, not a thing it failed to. Judging the thirteen by A61 is therefore not comparing them — it
is checking that each really is of that kind, and that nothing the survey *does* judge depends on
it.

## The thirteen, judged

The verbose cross-check names them: twelve veraPDF suite files, `6-2-10-t0{1,3,4,5,6}-fail-d`,
`6-2-10-t07-fail-c` (part 2) and `6-2-9-t0{1,3,4,5,6}-fail-d`, `6-2-9-t07-fail-c` (part 4), each
selecting `/Shading /SH0` inside a luminosity mask's group, under the page, a form, a Type 3 glyph,
an appearance or a tiling pattern; and Isartor `6-4-t01-fail-a`, selecting `/ExtGState /GS0` inside
the mask. In every one the innermost frame is the soft mask, so each is A61's kind. In every one the
mask was installed by a `gs` in a place both machines ran and compared, so the `/SMask` entry the
requirement reads was judged there. Each of the twelve files is built to fail the blending-space
sentence of ISO 19005-2 section 6.2.10 or ISO 19005-4 section 6.2.9, which reads the page's
`/Group` and the presence of the mask, never the mask's content, and the validator fails each of
them by that row, beside the device-colour rows the fixtures' own pages trip. None is a disagreement, and none could become one without the survey
walking the mask.

## Decision

The gate keeps the thirteen out of the comparison and **accounts for each** instead: it counts the
selections whose mask was set from a place both machines compared, names any set from a place one
machine alone ran — a planted setter that names the mask's own frame moves all thirteen there —
and **holds the survey's own selections under a soft mask at zero** — the
premise A61's scope rests on, asserted rather than assumed, so a survey widened to walk masks fails
here until the scope is narrowed to match.

## What stays open

Twelve of the masks paint a `DeviceRGB` shading in a file with no output intent. ISO 19005-2
section 6.2.4.3 states no exemption for a soft mask's group (TechNote 0010 A026 exempts only
`DeviceGray` as a soft-mask *image*'s space), so that colour is a candidate failure the survey does
not report. That is the survey's stated direction of error, and widening the walk is validator
work with its own false-failure question, not this gate's.
