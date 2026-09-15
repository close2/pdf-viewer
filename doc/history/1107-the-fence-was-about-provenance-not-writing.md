# 1107 — The fence was about provenance, not who writes

2026-09-15. ADR 1120. Ratifies `CLAUDE.md`'s **fourth** amendment to the authoring exclusion, on
the owner's `doc/questions/A65` (answered 2026-09-14, "Rule provenance."). Files: `CLAUDE.md`
(the exclusion paragraph), `doc/adr/1120-the-fence-is-about-where-the-marks-came-from.md`. Nothing
under `crates/` — this round is scope, not construction.

**What the owner settled.** ADR 1099 §4 had asked whether a `preserve` remedy may write an
annotation's normal appearance back onto the page it came off, rather than onto an appended page,
and had left the marks on an appended page because the third amendment's fence — "it composes new
content *over* pages" — reads as closing that door. `A65` answers that the fence is about the
**provenance of the marks**, not the operation of writing: where a clause requires the content to
move and fixes where it goes, the operators that place it decide nothing a reader sees. §12.5.5
computes the matrix `AA` from the annotation's own `/Rect`, `/BBox` and `/Matrix`, so the placement
is the standard's and the producer's, not this program's.

**The class, named and bounded.** *Clause-driven relocation of producer-written appearance
streams*, admitted by a two-part test: a clause requires the move, and that clause fixes the
position. It will take in later sites of the same shape — a flattened form field the standing one.
It stops where the marks become this program's invention: the watermark, and the redaction's
overlay and fill (§12.5.6.23's `/RO`, `/OverlayText`, `/IC`), which `A64` had already put on the
far side for this fence's own reason. That one sentence is the coordination point with the
redaction round.

**Scope, not construction.** The on-page construction is deferred to a clean build next batch
(ADR 1121, future). ADR 1099's appended page stays the mechanism today. The amendment records the
two honest costs the construction will owe as refusals with sentences — a producer whose content
pops the graphics state stack further than it pushes (§8.4.2's balance rule, stated for exactly the
`/Contents`-array population), and a moved appearance a remaining annotation would be drawn over
(§12.5.5's "previously painted annotations" backdrop, computed from the rectangles) — without
claiming either is built.

**One reading corrected for the future round.** The standard states **no painting order for
annotations at all**; "previously painted annotations" is its only word and it names no order. So
the overlap refusal's population is every remaining unhidden annotation whose `/Rect` meets the
moved one, not those earlier in `/Annots` — §12.5.3's Hidden flag is the one exclusion that is
derivable. ADR 1120 §4 writes this down so ADR 1121 builds against it.

**Verified.** ADR 1120's three verbatim quotes — §8.4.2's balance rule, §12.5.5's matrix `A` and
its backdrop — match `doc/md/` exactly; the balance rule is §8.4.2, not §8.4.4 (the operator table).
