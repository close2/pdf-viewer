# ADR 0480 — The clause that was one subclause above the silence

Status: accepted, 2026-08-22. Session 651.

`issue19633.pdf` page 1 has been contradicted since it became comparable, under a group of its own
called `CONTRADICTED_NEGATIVE_LINE_WIDTH` whose note said, in so many words, that the standard does
not decide the page:

> §8.4.3.2 says the line width "shall be a non-negative number expressed in user space units", so
> the value is outside the parameter's stated domain, and the clause states no recovery for one
> that is. **Three readings are available and each renderer takes a different one.**

Both halves are false, and the round that found it out was asked to take *one* contradicted group
apart. This ADR records what the page is, what the five renderers actually do with a negative line
width, and which clause decides.

## 1. Why this group, and not one of the twenty-two others

`crates/pdf-model/tests/oracle.rs` holds fourteen non-empty contradicted groups. `git blame` over
the whole run of group comments gives, for each, the newest commit that touched it, and every group
but one has been re-opened since it was written. The exception is this one: written whole in a single
commit (`244fd96c`, "a negative line width is a choice, and §14.7 read as a family") and not touched
since.

The second tell is sharper than the first. Its note is the only one in the file that says what a
reference *draws* with no number behind it — "a very faint one, consistent with the magnitude", and
"`ghostscript` draws something between the two". Every other group states ink, or a bounding box, or
a closed form. A sentence about a picture with no measurement under it is a sentence nobody
measured.

## 2. What the page is

The note said "1 page, and one operator in it". The file is **312 652 bytes** of iTextSharp form
with an `/AcroForm`, three document-level JavaScript actions and seven form XObjects. What makes the
first sentence nearly true is the *crop box*: `[131.5 439.89 383.0 600.89]`, which the oracle renders
at 252 × 161. Of the eleven `Do` invocations in the content stream, ten place `/Fm1` to `/Fm6` — each
of them `0 TL q /Tx BMC EMC Q` under a `[32768 32768 -32768 -32768]` bounding box — at page y 771 and
above, outside the crop box entirely. The eleventh is

```text
q 0.85409 0 0 0.85409 43.38 44.22 cm /Fm0 Do Q
```

and `/Fm0` is

```text
0 TL q q 1 0 0 1 354.73 518.87 cm 0 0 0 RG -0.1 w 1 j 1 J 0 0 m -185.44 77.07 l S Q Q
```

So the visible page is one stroked diagonal, **171.51 points long at 22.56° from horizontal**, asked
for at a user-space width of −0.1 which the `cm` scales to a device width of 0.0854.

## 3. What the five renderers draw, measured

The mark's length is known, so ink ÷ length is the width a renderer actually painted. Ink over the
oracle's own artefacts, `-alpha off` on the R channel, in whole pixels of a 252 × 161 raster:

| | ink | ÷ 171.51 | what the note said |
|---|---|---|---|
| ours | 172.54 | **1.006** | one device pixel — correct |
| `hayro` | 170.87 | 0.996 | not mentioned |
| `ghostscript` | 96.81 | 0.564 | "something between the two" |
| `poppler` | 42.44 | 0.247 | "consistent with the magnitude, 0.1 of a pixel" |
| `mupdf` | 37.49 | 0.219 | the same |

The document asked for 0.0854. `poppler` paints **2.9 times** that, `mupdf` 2.6, `ghostscript` 6.6.
"Consistent with the magnitude" was true of nobody.

## 4. The instrument: ADR 0419's ladder, continued through zero

Session 584 built a ladder of one rule at seventeen widths on a 200 × 200 page to price each
renderer's sub-pixel **floor**, and it is the right instrument here — except that every rung of it is
non-negative, so it never asked the question this page turns on. Same geometry, same metric (a
160-unit rule, mean ink in levels of 255, so the geometry's own answer is `1.02 × w`), 72 dpi, run
down through zero:

```text
   width   geometry      ours   poppler     mupdf        gs     hayro
     1.0     1.0200    1.0200    1.0200    1.0241    1.3000    1.0241
     0.8     0.8160    0.8160    1.0200    0.8160    1.0960    1.0241
     0.5     0.5100    0.5120    1.0200    0.4761    0.8201    1.0241
     0.3     0.3060    0.3040    1.0200    0.3399    0.5439    1.0241
     0.2     0.2040    0.1999    1.0200    0.2040    0.2721    1.0241
  0.1366     0.1393    0.1359    1.0200    0.2040    0.2721    1.0241
    0.05     0.0510    0.0479    1.0200    0.2040    0.2721    1.0241
    0.01     0.0102    0.0079    0.1280    0.2040    0.2721    1.0241
   0.001     0.0010    0.0079    0.1280    0.2040    0.2721    1.0241
     0.0     0.0000    1.0200    1.0264    0.2040    0.2721    1.0241
  -0.001     0.0000    1.0200    0.1280    0.0000    0.2721    1.0241
   -0.01     0.0000    1.0200    0.1280    0.0000    0.2721    1.0241
   -0.05     0.0000    1.0200    1.0200    0.0000    0.2721    1.0241
 -0.1366     0.0000    1.0200    1.0200    0.0000    0.2721    1.0241
    -0.2     0.0000    1.0200    1.0200    0.0000    0.2721    1.0241
    -0.3     0.0000    1.0200    1.0200    0.0000    0.5439    1.0241
    -0.5     0.0000    1.0200    1.0200    0.0000    0.8201    1.0241
    -0.8     0.0000    1.0200    1.0200    0.0000    1.0960    1.0241
    -1.0     0.0000    1.0200    1.0200    0.0000    1.3000    1.0241
```

**The positive half reproduces ADR 0419's 72 dpi table to the digit**, which is how the instrument is
checked before its new half is believed — with one rung moved, and in the direction that ADR wanted:
ours at 0.001 of a pixel was **0** there and is 0.0079 here, one level of 255 spread over two rows,
so the "one number in the whole table that no reading of the clause permits" is no longer produced.
Nothing in this session did that; it is recorded as an observation and its cause is not chased here.

Three readings come off the new half.

- **`poppler` and `ghostscript` stroke the magnitude.** Every negative rung equals the positive rung
  of the same size, exactly, at every width. A sweep of the same rule at 0°, 1°, 5°, 10°, 20°, 29.4°,
  45°, 60° and 90° says the same for `poppler` at every angle, to four figures.
- **`mupdf` does not, and does not take one reading either.** Within 5° of an axis a negative width
  paints **nothing at all** — mean ink exactly 0, and no warning on stderr. Beyond 10° it paints
  precisely what it paints for a width of *zero*, which is its own 0.2-device-pixel floor. One
  renderer, two answers, selected by the angle of the line.
- **Ours and `hayro`'s are one device pixel at every negative rung**, the same answer both give for
  zero. (`hayro` gives it for every positive width below one as well, which is a floor rather than a
  reading of the sign; the ladder cannot separate the two for that renderer, and does not need to.)

And one thing the *positive* half says that ADR 0419's table could not, because every rung of it was
axis-aligned: **`poppler`'s 1.0-pixel floor is a property of the orientation, not of the renderer.**
Run on a 29.4° rule instead, its ink rises monotonically with the width all the way down to 0.02 —
0.1981, 0.2194, 0.2693, 0.3127, 0.3676 of a pixel for widths of 0.02, 0.05, 0.1, 0.15 and 0.2, which
is the width plus about 0.17 of anti-aliasing spread and no floor at all. That is why the floor table
in §8.4.3.2's ledger row now says which rule it was measured on.

## 5. The consensus that outvotes us is two mechanisms meeting by accident

The gate's line reads `poppler and mupdf agree, we differ`. On this page `poppler` is painting
|−0.1| × 0.85409 plus its own anti-aliasing spread, and `mupdf` is painting its floor — the same
picture it would paint for **any** width it declines to go below. They land 0.247 and 0.219 apart and
form a consensus; at `-1 w` they are 1.02 and 0.00 apart, which is the widest disagreement anywhere
on the ladder.

That is trap 9's second shape (a renderer's default standing in for a reading) sitting on its first
(two programs agreeing without agreeing about anything), and here it has a third property worth
recording: **it is decided by the angle of the line.** Had this file drawn the same rule
horizontally, `mupdf` would have drawn nothing, the pair would not have formed, and the page would
have been `ambiguous`.

## 6. The clause, which is one subclause above where the note was looking

`spec-errata emit doc/ISO_32000-2_sponsored_EC3.pdf` prints one annotation under the heading
**8.4.3.2 Line width**: Issue #368, a Caret, `Review/Accepted`, saying "See 9.4.1, 'General' for
additional information that must be managed as part of the graphics state stack when q and Q
operators occur within text objects." **It does not point at §8.4.3.2.** The tool keys a note to the
section the outline puts its *page* in; this caret's `/Rect` is `[358.768 239.123 367.781 246.468]`
on physical page 175, which `pdftotext -bbox` places between the last line of §8.4.2's q/Q paragraph
and the §8.4.3 heading. So it is an addition to §8.4.2, and **Errata Collection 3 touches nothing in
§8.4.3.2** — the same conclusion ADR 0419 reached, arrived at again because a heading is not a
pointer.

§8.4.3.2 gives the parameter its range and stops:

> It shall be a nonnegative number expressed in user space units

What decides a value outside a range is §8.4.1, and it names this parameter while doing so:

> Parameters that are numeric values, such as the current colour, line width, and miter limit, shall
> be clipped into valid range, if necessary. However, they shall not be adjusted to reflect
> capabilities of the raster output device, such as resolution or number of distinguishable colours.
> Painting operators perform such adjustments, but the adjusted values shall not be stored back into
> the graphics state.

Three sentences, and this tree obeys all three:

1. `content.rs`'s `w` handler and `ext_gstate.rs`'s `/LW` clip −0.1 to 0, because the first sentence
   requires clipping and §8.4.3.2 states the range;
2. `Stroke::device_width` substitutes one device pixel, because §8.4.3.2 requires that of a width of
   zero — and it does so at *painting* time, which is what the third sentence permits;
3. the substituted width is never written into `GraphicsState`, which is what the third sentence
   requires.

**The magnitude reading is the one the clause forbids**, because |−0.1| is not a clip of −0.1 into
`[0, ∞)`; and the "paint nothing" reading, which §8.4.3.2's definition of stroking would give for a
negative half-width, never arises, because the clipping happens before the stroking. So the page has
a right answer, we have it, and the two renderers that outvote us do not.

## 7. What this cost, and where the answer already was

The sentence in §8.4.1 was **already quoted in the same crate** — `content.rs`'s `miter_limit`,
under the words "which §8.4.1 asks for", since the twenty-fourth session — for the parameter the
same list names one *after* the line width. And the conformance ledger's own §8.4.1 row has said
since that session that numeric values "shall be clipped into valid range" — "the line width at 0,
the miter limit at 1, the alpha constants at 0..1" — while §8.4.3.2's row, two rows down, called the
same clamp a documented choice among three readings. The ledger contradicted itself and the code
followed the wrong row.

This is `CLAUDE.md` principle 5's standing warning at the shortest range it has been observed at:
*"the specification defines nothing here" is itself a claim about the specification, and it decays.*
Reading the titles around the subject would have cost a minute; §8.4.1 is titled *General* and sits
directly above *Details of graphics state parameters*.

## 8. Decision

- **Nothing about the rendering changes.** No pixel moves and the page stays contradicted, because
  the clause is on our side and the gate should keep watching it.
- The clamp is documented as a **derivation** wherever it is made or explained: `content/run.rs`,
  `content/ext_gstate.rs`, `pdf_render::Stroke::device_width`, the §8.4.1 and §8.4.3.2 ledger rows,
  and the oracle group.
- `line_parameters.rs::a_negative_line_width_is_clipped_into_range` asserts the clipped value by both
  of §8.4.1 NOTE 1's routes, so the derivation has a test rather than a comment.
- The group **keeps its name**. `issue19633.pdf` is about the negative line width and nothing else,
  which makes it the first contradicted group whose name survived being measured — thirteen
  examinations, twelve names wrong. What was wrong here was everything written under the name, so
  `doc/traps/pixels-and-rasterisers.md` and `doc/oracle-and-corpus.md` now say that the note is the
  thing to distrust and the name is only its first sentence.

## 9. What is left owed

- **`mupdf` painting nothing for a negative width within 5° of an axis, and its own floor beyond
  10°, is a defect of that renderer** by §8.4.1's clipping sentence, as is `poppler`'s and
  `ghostscript`'s magnitude. None is reported upstream by this session; the ladder is here and the
  probe generator is trivial to rebuild from §4.
- **`hayro`'s one-device-pixel floor at every width below one** is the same shape as the floors ADR
  0419 priced and belongs with them; `doc/HAYRO_ISSUES.md` does not name it.
- **Ours at 0.001 of a device pixel changed from 0 to one level since ADR 0419** and nothing recorded
  it. It is the direction that ADR argued for, so it is not urgent — but a table in an accepted ADR
  moved without a session claiming it, and finding out which one is a small, honest task.
