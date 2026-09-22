# 1205 — A line gets the chord the box leaves it, and three redaction refusals come off

## §12.7.4.3 — the `/DA` text matrix (ADR 1247)

ADR 1130 left a `Tm` turning the line off both of the box's axes reported, on "no length the box
states is the room that line has". True, and not a reason: the clause needs a room only for `/Q`,
and the room is the **chord** the box leaves *on that line* — the preimage of a rectangle under an
invertible linear part is a parallelogram whose horizontal chord is closed form at every baseline.

`Room` states the box's two coordinate ranges as two strips of the measuring space and intersects
whichever bound the line; `Stack` says which baseline a line sits on, so wrapping, auto-sizing,
`/Q`, the comb's cells and `overflows` each ask the room of the line they are about. ADR 1130's two
families are that restricted — `shrink` + `drift` is the constant-chord case term for term — so
`shrink`, `drift`, `turned`, `along`, `across`, `shear` and `Set` are gone, `place` is the linear
map and `shrink_point` its inverse, and no fixture of ADRs 1114 or 1130 moved.

`Owed::TransformedTextMatrix` became `Owed::SingularTextMatrix`: a linear part with no inverse
sends the plane onto one line, so no translation is more appropriate than another and the outlines
enclose no area. Still a report — the marks are the producer's matrix on the producer's value.
**And which of three things refuses the Arabic free text is settled**: the **face**. No compiled-in
face has one Arabic glyph (ADR 0348, `doc/stack.md`), so `encode` yields no codes and UAX #9's
ordering would have nothing to order — bidi ahead of a glyph source has no input.

## §12.5.6.23 — three redaction refusals (ADR 1248)

- **§8.5.4's clipping path.** The clause separates painting from clipping in time: the cut marks go
  first, under the clip already in force, then the boundary is re-stated from the producer's own
  bytes closed with `n`. Bytes taken at the **painting** operator, because §8.5.4 only *permits* the
  tidy order while what it bounds is "the newly constructed path".
- **§8.9.5.4's `/Alternates`.** Dropped from the redacted page's copy rather than destroyed: the
  entry is the only route to a variant and the writer's closure copies only what is referenced.
- **`JPXDecode`.** Cleared like the other three codecs on three conditions, each a refusal by name:
  the decode is on the grid the dictionary states (§7.4.9 NOTE 3), Table 87's `/SMaskInData` is
  absent or zero, no component is deeper than eight bits. The grid check is asked of every codec;
  the module doc had promised a JPX-specific reason the match arm never gave.

§12.5.6.6's `/DS` departure was already built — `Refusal::DefaultStyleUnapplied`, ADR 1224, with
its test — so the contract's third item was owed nothing. Gates: tier 1 scoped, tier 2 behind the
lock; siblings' mid-edits were red in `view.rs`, `archive/` and `viewer-core` throughout.
