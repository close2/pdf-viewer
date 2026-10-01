# 1486 — An accented character is subset as the outline it draws

Session 1325. Status: accepted and built. Code: `crates/pdf-font/src/embed/accented.rs` (new),
`crates/pdf-font/src/embed/reach.rs` (`Walk::Seac` carries `Accented`; the width is read under the
note's section 4.1 Note 4), `crates/pdf-font/src/embed/cff.rs` (`Closure::composed`).
Amends: ADR 1449 section 6, whose first sentence (an accented face is written whole) this replaces.

## 1. Why keeping the two components is not enough

Adobe Technical Note #5177 Appendix C gives `endchar` a second form, `adx ady bchar achar endchar`.
It draws two other glyphs of the face. The note says how each is found: the code goes through
`StandardEncoding` to a glyph name, and the charstring is then located by that name. The subset is
always written CID-keyed (ADR 1449 section 2), and a CID-keyed program's charset maps glyphs to
CIDs, not names. So the form has nothing to resolve against in the subset, and keeping the two
component glyphs beside it (what the brief proposed) would leave a charstring that no conforming
reader can finish.

## 2. The decision: write what it draws

The subset holds, in place of the accented glyph's own charstring, a composed one: the width
operand the original stated, then the accent's path moved to `(adx, ady)` and the base's path at
the origin, as `rmoveto`, `rlineto` and `rrcurveto`, then `endchar`. Each component is evaluated in
16.16 fixed point, so every coordinate is the one the original reaches, without rounding. The
components are not kept unless the text shows them too. The accent comes first because that is the
order in which the face's own reader evaluates the form. A fill does not depend on that order.

**The cost is hints.** The composed charstring carries no stem hints. Each component's hints are
stated against its own outline, and the note's hint operators must come before the first path
operator, so the two sets cannot be laid end to end. Merging them means renumbering stems and
rewriting `hintmask` bytes, which is a hint compiler. A hint changes how an outline meets a coarse
grid, never the outline. Every other glyph of the subset keeps its own bytes.

**Where it is not done**, the face is written whole as before: a CID-keyed source (it has no names
for the form to reach), a code `StandardEncoding` leaves empty or the charset does not name, a
component that is itself accented (the note forbids nesting), and a charstring that drew a path
before its `endchar` (the note gives that no meaning).

## 3. Measured

No face on this machine uses the form: a fontTools scan of every `CFF ` face `fc-list` offers found
none. The witness is therefore built, a hand-made name-keyed face in `embed::cff`'s tests. An
`Aacute` over a hinted `A` and an `acute`, with a width past a non-zero `nominalWidthX`, is subset
to two glyphs. Its moves land where Appendix C puts them, worked by hand, and its advance is the
stated one. It draws, outline for outline, as the tree's reader draws the whole face's accented
glyph. With the vertical offset dropped from the composition, the test fails. A face whose
accented glyph names a base it lacks is still written whole.
