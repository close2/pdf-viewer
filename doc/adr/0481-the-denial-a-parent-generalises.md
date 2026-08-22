# ADR 0481 — The denial a parent generalises, and the sweep not built for it

Status: accepted.
Session: the six-hundred-and-fifty-second, a clause round under `doc/todo/01`'s binding rule.

## 1. What this decides

1. **The mirror of `doc/todo/01`'s eighteenth sweep is not built**, and the reason is a
   measurement rather than a preference: over the whole ledger it has a population of **fourteen**,
   of which three are contradicted and all three are noise.
2. **The eighteenth sweep is directional on purpose**, and this ADR is the sentence that says so,
   so that a later round reading ADR 0475 does not spend itself building the obvious next thing.
3. **An understating parent is found by reading, and the reading list is the blame ordering.**
   Three of this round's five rows were that shape, and none of them was reachable by any program
   in `tools/conformance`.

## 2. What the mirror would have been

ADR 0475 built `--bin overstated`: a parent row asserting that an entry or a table **is read**,
against a descendant's denial that anybody reads it. Both sides are sentences this project wrote
about its own code, so a contradiction is a contradiction whatever the standard says — which is
what makes it the only sweep here that opens no source file.

The mirror is the same comparison with the sign flipped: a parent **denying** that anything reads a
term one of its own descendants asserts. It is nearly free. `overstated::parts` already splits a
note into stance-bearing pieces, `unread::CLAIMS` already says what a denial is, `ASSERTIONS`
already says what an assertion is, and `keys_attributed_to` already extracts the terms. The change
is which vocabulary is applied to which side of the family.

Nothing about the *cost* argued against it. What argued against it was running it.

## 3. The measurement, which is trap 11 applied to an instrument

`doc/todo/01` requires a population to be measured before a debt is priced. The mirror was
approximated over the current ledger — parent rows with descendants, their denial parts, the terms
those parts name, against every descendant's assertion parts:

> **170 parent rows have descendants. Fourteen denied term-mentions between them. Three are
> contradicted by a descendant, and all three are noise on reading.**

- **§8.11.4's `/Name` against §8.11.4.3's** is the one-short-key shape the second sweep already
  prints every run: the parent denies an optional-content *configuration's* `/Name`, the child
  asserts a *group's*.
- **§9.8.3's Table 122 against §9.8.3.1's** is a pair of corrections each quoting the wording it
  retired, which is the oldest false positive in `doc/todo/01` and is marked rather than dropped in
  every sweep that has it.
- **§9.8's Table 120 against §9.8.1's** is a table denied in part beside a table read in part — the
  dominant noise shape of the eighteenth sweep, arriving unchanged in the mirror.

Compare the forward direction, which asserts **125 terms** over the same 170 rows and has found two
live defects. The asymmetry is a factor of nine, and it is not a small sample.

## 4. Why it is structural, and not a matter of waiting for the population to grow

A row asserting a capability **enumerates**. That is what makes it a summary of its children at
all: "Table 5's entries are read where they are used — `/Length` and `/Filter` and `/DecodeParms`
by the parser, `/F` by `Document::is_external`". Every noun in it is a term a program can match.

A row denying one **generalises**. It says *the dimensional metrics*, *the entries a synthesised
face would need*, *what a marking device describes*. A generalisation names no `/Key` and no
`Table NNN`, so there is nothing for the other side of the comparison to be about.

**This round's own defect is the proof.** §9.8 said:

> The dimensional metrics are read by nobody, because this tree selects an installed face rather
> than synthesising one from them

while §9.8.1 has said since the three-hundred-and-seventy-eighth session that `/Ascent` and
`/Descent` are read — by `pdf_font::vertical_extent` for the band a selection highlight is laid
over, and by `variable_text::Metrics::read` for a form field's baseline. That is exactly the mirror
shape, in the family the mirror sweep's own noise points at, and **the mirror sweep would not have
printed it**: "the dimensional metrics" is neither a key nor a table.

To reach it a program would have to decide that *the dimensional metrics* means Table 120's
`/FontBBox`, `/Leading`, `/CapHeight`, `/XHeight`, `/StemV`, `/StemH`, `/AvgWidth`, `/MaxWidth`,
`/Ascent` and `/Descent` — a judgement about what an English sentence means. Every sweep in
`doc/todo/01` refuses that judgement by construction, and the refusal is why their output can be
believed. Widening one to make this hit would cost more than the hit.

## 5. What is done instead

Nothing is built, and two things are written down:

- `doc/todo/01` carries the measurement, so the next round to have this idea can read the numbers
  rather than re-derive them.
- The header records that the eighteenth sweep is directional, with the reason in one clause.

And the rows themselves are read. This round's band — ranks 1, 2, 3, 5 and 13 of the blame
ordering — produced three defects, and all three are the fifth failure shape inside a single
family:

| row | was | is |
|---|---|---|
| §10.4.2.4 | Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` "are read by nobody" | read since the four-hundred-and-twenty-sixth as the condition that names a departure; what is owed is that they are not *evaluated* |
| §9.8 | "the dimensional metrics are read by nobody" | `/Ascent` and `/Descent` are read, by two readers, and §9.8.1 has said so for two hundred and seventy-four sessions |
| §10.4.2 | "`partial` for what **two** of the four conversions below owe" | three; §10.4.2.4's debt is of a different kind and was left out of the count |

**The common shape is worth one more sentence, because it is what the mirror sweep was supposed to
catch and the reading caught instead.** In all three the correct answer was already written in
another row of the same family, in this project's own words, and the wrong row was rewritten
repeatedly without its own first sentences being re-read. §10.4.2.4's tail was rewritten twice —
by the four-hundred-and-twenty-sixth and again by the four-hundred-and-twenty-seventh, both about
the very mechanism the stale sentence denied.

## 6. What this costs

If a note is written one day that denies a capability by *naming* the table or the entry, the
mirror will be worth building and this ADR is the thing to revisit. Nothing here says the direction
is uninteresting; it says the direction is empty at fourteen mentions, and that the emptiness comes
from how denials are written rather than from how many there are.

The one loss is a small one and it is named: the three noise hits above are real contradictions of
form, and a round that wanted them printed every session could have them for about forty lines.
They would be read and dismissed every session, which is the cost the ratio argument in ADR 0249
is about.
