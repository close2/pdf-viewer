Status: complete
Given: 2026-10-23, in conversation — transcribed by the round
Owes: the build — the four collections' faces shipped as a second download beside the base program
(a CJK pack): one CID-keyed program per collection ISO 32000-2 §9.7.5.2 names, ordered by that
collection's CIDs, from an open-licensed family (Source Han, SIL OFL 1.1, or another whose notice
this program can carry); the composite CID-embedding route, which is ADR 2162's pattern at
composite scale (`as-if-printed` discards, `as-shown` stops); the A550 notice pattern and a
`doc/third-party-data.md` row; and `doc/stack.md` asked about the data dependency. The
`Adobe-Identity` half is not settled by this answer — the same message reopens it, and Q570
carries the two remedies proposed there (acting round)

> as already mentioned, the spec is the goal.  The corpus is just a nice to have to test.  I
> would definitely ship the four faces as a second download.  But I also would like to revisit
> the Adobe-Identity case.  Couldn't we create a pixel font from the provided font and embed it?
> I think pdf/a requires a ToUnicode map, but if one exists, shouldn't we be able to create such
> a raster-font?  Another idea: if there is no ToUnicode, we could convert the text to images.
> This would still be good enough for the "as-printed" profile and would become a valid pdf/a
> (obviously this should be the last resort and only if an image would be an equivalent to what
> the input pdf represents in terms of usability -- no selectable text anyways,...)

Reading: the recommendation ("not now") is overruled, and on the project's own stated ground. The
owner restates the rule that decides it: the specification is the goal and the corpus an
instrument — a requirement is not declined because no document in the drawer exercises it, which
is Q436's philosophy and this file's two-denominators rule ("work is chosen from both, never
one"). The census keeps its real job: it says how much moves when the faces ship (the
collection-named half of the composite refusals), not whether the work is owed.

What the answer settles:

- **The four faces ship, as a second download.** The base program keeps its size and its startup
  rules — compiled-in data costs zero parse at launch, and a pack loads only when a collection
  font is needed — so the tens of megabytes sit beside the exe, not inside every download and
  not inside every converted document, whose embedded programs the subsetter bounds to the used
  glyphs.
- **The collection-named half of the composite refusals becomes convertible**, under the same
  named `machine-face` loss ADR 2162 rules for simple fonts: `as-if-printed` discards it,
  `as-shown` stops, and `--font` stays the faithful route for an operator holding the producer's
  program.
- **The `Adobe-Identity` half is reopened, not settled.** The same message proposes two remedies
  — a font keyed from a trustworthy ToUnicode where one exists, and a rasterized page where
  nothing trustworthy does — and Q570 is that reopen, prepared for the owner's word.