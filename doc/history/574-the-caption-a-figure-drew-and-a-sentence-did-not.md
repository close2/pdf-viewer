# 574 — the caption a figure drew and a sentence did not

**Finding:** §12.5.6.7's `/Cap` had been refused since the hundred-and-sixteenth session because
"a caption needs a font and no entry of a line annotation states one" — true, and the same table
cell says the text "shall be replicated as a caption in the appearance of the line". The silence
sits inside the `shall`. And when the caption was built, **the standard's own Figure 81 rejected
the first reading of it**: an implementation that auto-sized the caption to the line's length met a
figure whose third example is captioned *This is a caption that is longer than the line*.

Date: 2026-08-18.
Argued by: [ADR 0409](../adr/0409-the-caption-a-figure-drew-and-a-sentence-did-not.md).
Files touched: `crates/pdf-model/src/{appearance,variable_text}.rs`,
`crates/pdf-model/tests/annotations.rs`, `doc/conformance/ledger.toml` (§12.5.6.7),
`doc/HANDOVER.md`, `doc/adr/{0075,0106,0192}.md` (amended where a later round disproved them),
the ADR and this file.

## Why this item, and what was rejected for it

`tools/state.sh quick`, `doc/todo/README.md`, then the demand side and the spec side in that order.

**The demand side had nothing loose.** The oracle run at the top of the round printed an empty
"ambiguous, undiagnosed" list, a contradicted head that is `bitmap-*` — ADR 0381 settled those from
the corpus's own self-consistency — and `issue15716`, `issue14802`, `file_url_link` below it, each
already carrying a group. `doc/todo/03`'s corpora chunks are spent: every population on this disk is
ranked, and what §14 leaves is a decision rather than a defect.

**So the spec side, and the round's instruction named the shape**: a refusal justified by "the
specification states nothing here". Three sweeps ran first (`conformance --bin entries`,
`--bin blockers`, `--bin owed`) and a Python pass over every silence claim in `ledger.toml` — 89
rows. Rejected on the way, each with the reason:

- **§12.5.6.12's rubber stamp icons.** The parting sentence is a modal verb — Table 184 says a
  reader *should* provide predefined appearances where §12.5.6.4 says *shall* — and that reading
  was made deliberately in the hundred-and-twentieth session and still holds.
- **§12.5.6.11's caret.** Reconsidered against ADR 0109 in the hundred-and-twenty-fourth and kept:
  the clause says the annotation *is* "a visual symbol", indefinitely, and never says it shall
  appear as one.
- **§12.5.4's beveled and inset borders.** `grep -i "bevel\|emboss\|engrav"` over the whole standard
  returns Table 168's own cell and the line-join clauses, and nothing else — the highlight and
  shadow colours are genuinely unstated, the rectangle is drawn, and the illusion is reported.
  `examples/border_precedence_census` finds no witness among 33 781 appearance-less annotations.
- **§12.5.4's cloudy `/BE`.** Genuinely no curve stated, and `examples/witness_census` finds **two**
  documents stating `/BE` as a name at all.
- **§12.5.6.6's free text border colour.** Table 166's `/C` names three uses and none is this
  subtype's border; `/DA` is "the default appearance string that shall be used in formatting the
  text", and a border is not text.

## What the clause turned out to say

Table 178's `/Cap` is a `shall` about the appearance, and `/CP` and `/CO` state the position with
nothing left over — `/CO`'s "horizontal offset **along the annotation line**" and "vertical offset
**perpendicular to the annotation line**" are a statement in a frame whose axes are the line's, so
the caption turns with the line and the page's axes appear nowhere in the entry.

What is unstated is a **font and a size**, which is §12.5.6.4's silence exactly. The face is
§9.6.2.2's Helvetica out of this binary, through the stand-in ADR 0112 already built; the size is
12 points, which is what §12.7.5.3's own EXAMPLE sets variable text at and is the same worked
example `variable_text::LINE_HEIGHT` takes its 13/12 from.

## What the picture said, and it was the standard's

The first implementation auto-sized the caption to the line's own length, reasoning that "centred
inside the line" and "on top of the line" make the line the caption's only stated extent. Figure 81
— which the entry cites by name — draws three captioned lines, and the third is longer than its
line and set at the same size as the others. The reading was wrong and nothing in the prose could
have said so.

The same figure settled two more things: an inline caption sits in a **break** in the line rather
than over it, and the caption is the line's colour. Both were visible in the first render as
defects — a rule struck through the words — and both are the figure's to answer.

**`doc/md/` carries the figures inline as base64 and this project had never extracted one.** Four
lines of Python. Every clause that says "as shown in Figure N" is a clause whose reading is
incomplete without it.

## What it cost

`variable_text::LaidOut` gains an `advance` — the widest line's width at the size chosen, summed
from the advances the glyphs were positioned by rather than measured a second time — because the
break needs to know where the words are. `appearance::line`'s three exits become one, so that the
caption is read *before* the line is stroked.

Two rules bound the break, both ADR 0106's: a break that would leave no line is not taken, because
§12.5.6.7 makes the line required; and a `/CO` that lifts the caption clear of the line takes the
break with it, which is Figure 82's case.

## What moved

**No gate.** `examples/witness_census` finds two documents on this disk stating `/Cap` at all —
both pdf.js line fixtures, both `false` — so the witnesses are hand-built, which is trap 8's case
and §12.5.6.6's `/CL` precedent (ADR 0329). Corpus, oracle, text, quorra, dates, XMP and conformance
all reproduce.

Two tests added and one rewritten: `a_line_ending_is_drawn_and_a_caption_is_still_named_beside_the_line`
asserted the refusal, and a test that pins a refusal is rewritten when the refusal ends.

## What is left

`/IT` and `/Measure` on this subtype, which §12.5.6.7's row names. And the question the figure
raises: whether a sweep over every "as shown in Figure N" in the standard is worth a round, now
that reading one is known to cost nothing and to have overturned a reading on its first use.
