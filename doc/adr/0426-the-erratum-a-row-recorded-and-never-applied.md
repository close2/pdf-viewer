# ADR 0426 — The erratum a row recorded and never applied

Status: accepted, 2026-08-18 (session 591).

## Context

The five-hundred-and-ninetieth session found the §14.8.4.7.2 ledger row naming Errata Collection
3's Issue #437 — recorded there since the four-hundred-and-eighteenth — and then quoting the
sentence that erratum struck out, two sentences later, while four places in `crates/` quoted the
same struck sentence as current text. Its lesson, in its own words:

> **a row that records an erratum is not a row that has applied it.**

That is a whole failure shape and nothing in this tree looked for it. `spec-errata check` asks
whether a quotation lands on struck text and knows nothing about whether the writer had read the
erratum, so a place that names the erratum and quotes the old words reads exactly like a place
that never heard of it — and it sorts into whichever of `check`'s two buckets `Landing::in_clause`
puts it in. **A row that names the erratum looks maximally diligent, which is precisely why nobody
re-reads it.**

`doc/todo/02` §4's closing paragraph already knew the neighbouring hole — "an erratum over text
nobody has quoted is invisible to it" — and `moved` closed that one in the
five-hundred-and-sixty-fifth. This is the third direction of the same question.

## Decision — a seventeenth sweep, and it lives in the sidecar

```sh
cargo run --release -p spec-errata -- applied doc/*.pdf     # two seconds over all fourteen
```

**Not `cargo run -p conformance --bin …`, which is where the other thirteen committed sweeps
live, and the reason is a rule rather than a convenience.** ADR 0252 makes the dependency run one
way — `spec-errata` → `conformance`, never back — because the gate must keep checking quotations
against a conversion this project did not make; if the checker read a conversion we generated, a
defect in our extractor would become a defect in the standard. This sweep needs the errata, and
the errata are read out of fourteen PDFs by `pdf-model`. So it is a subcommand beside `check`,
`emit`, `census` and `moved`, exactly as `doc/todo/02` §4 already records the twelfth sweep as
"not run from here at all".

**What it asks.** For every place in this tree that *names* an erratum — a ledger note, a run of
comment lines, a Markdown block — does a quotation inside that place match what the erratum struck
out, and **not** match what the erratum put there instead?

**Its discriminator is that the erratum is named as data, by the writer, inside the place
itself.** Every other sweep over quotations has to guess an attribution: `check` infers the clause
from the nearest citation above a quotation and says so by calling its own buckets a sort order
rather than a verdict. Here nothing is guessed, and the erratum supplies *both* sides of the
comparison — the `StrikeOut`'s covered text and the `Caret`'s `/Contents`, joined by Table 172's
`/IRT`, which is the new `Note::change` field.

**Three unit tests establish that it discriminates rather than that it runs**, and the first of
them plants the defect session 590 fixed: the §14.8.4.7.2 note as it stood, naming Issue #437 and
then quoting the struck sentence in `CLAUDE.md`'s own `[e]` spelling. Planted into the *real*
`ledger.toml` and run, the sweep names it on the read-first list with the erratum's own
replacement printed underneath, and the unmarked count moves by exactly one.

### The noise, printed rather than filtered

- **A correction quoting the wording it retired**, which is this family's oldest false positive
  and here the commonest hit by construction: the honest way to record an erratum is to say what
  the sentence used to be. Marked `[history]` from a window round the quotation, and still
  printed.
- **`doc/errata-read.md` is that shape from end to end**, being the reading itself. Counted apart.
- **A `#NNN` this collection does not carry** is dropped and counted, so a clean run says how much
  of the population it could judge.
- **An erratum that only deletes** has no replacement side, so every quotation of its struck text
  is a hit. Correct rather than a defect: there is nothing for the writer to have moved to.

### Two decisions the window cost

**The history phrases are not `conformance::blockers::HISTORY`'s**, and the reason is the finding
itself: that list carries `said` and `this row`, which are the ordinary connective tissue of a
ledger note — the §14.8.4.7.2 row's stale quotation is introduced by "that is this row's one
reader-side requirement" — so borrowing it would have marked the one defect this sweep exists for
as noise. What is here is a phrase that retires *the quoted words*.

**The window reads both ways.** This project writes a correction in both orders: `standard.rs`
says the clause "used to say so in a `shall` and no longer does" and then quotes the retired
sentence, while `appearance.rs` sets the retired blockquote first and explains underneath that
"the blockquote above is the old one". Four hundred characters either side is the largest window
under which the planted §14.8.4.7.2 defect still reads as unmarked and the smallest under which
both of those read as marked. At 200 characters and backwards only, the first run put 72 hits on
the read-first list; at 400 either side it put 26, and every one of the 46 that moved was a
correction.

## The comparison was half-blind, for the third time, and that is the round's other half

`squeezed` — what `check` and now `applied` compare with — was `conformance::quote::normalise`
plus dropping the spaces and folding case. It kept **square brackets** and it kept **dash
shapes**, and both of those are things this project's own conventions produce:

- `CLAUDE.md` writes an altered first letter as `"[e]ncloses one or more PDF annotations"`, which
  is exact quotation with a mark round the change. A comparison that keeps the brackets calls that
  passage absent.
- `doc/md/` writes `Table 118 -Additional entries` where the standard sets an em dash, so a
  quotation of a sentence containing a table caption cannot match either.

`conformance::prose::folded` had already answered both, for the sweep over this project's Markdown
(ADR 0375). `squeezed` is that function now — one comparison in the crate rather than two, which
is ADR 0360's argument about a sweep whose rule is retyped. **Every folding is applied to both
sides, so it can only ever hide a finding and never invent one**, which is the same argument
ADR 0253 made for the spaces and ADR 0375 for the quotation marks. This is the third instance of
one instrument being blind to a spelling this project's own rules ask for.

What it moved: `check`'s struck-passages-still-in-the-conversion list from 151 lines to 178, its
in-clause landings from 73 to 86, its elsewhere bucket from 272 to 293. **Thirteen new landings,
read one by one, and three were defects.**

## What the two runs found

- **§9.6.2.2's ledger row, twice, and it is this sweep's own subject.** The note opened "[t]he
  clause asks for '[t]hese fonts, or their font metrics and suitable substitution fonts'" — the
  sentence Issue #47 and #48 strike outright — three thousand characters above the same note's own
  record that they do. And the five-hundred-and-twenty-third session added a second quotation of
  it beside ADR 0358's substituted-glyph reading, four sentences from that record. Both corrected;
  the second's conclusion is *stronger* without the sentence, because with the availability
  `shall` retired the clause states no requirement about a substituted face at all.
- **§9.6.2.1's ledger row**, which quotes both of Issue #47 and #48's struck sentences and names
  the erratum nowhere — so `applied` could not see it and the repaired `check` could. Session 418
  corrected this exact sentence in three doc comments and the four-hundred-and-nineteenth in two
  more; the row that the code's own ledger entry points at was never swept with them. It rests on
  Table 109's permission and §6.3.2.2 now, which is the warrant `pdf_font::standard` already
  carries.
- **§9.10.3, in two rustdoc blockquotes** — `pdf-font/src/loading.rs` and
  `pdf-model/tests/composite_fonts.rs` — quoting the sentence Issue #462 strikes, as current text,
  in the **one population this project gates**. §9.10.3's own row recorded that erratum in the
  five-hundred-and-eighty-seventh session and closed with the words "a later round quoting it as
  current would be quoting text the collection has removed"; the quotations were already there
  when it was written. Neither behaviour moves — `/UseCMap` is the entry under both readings — and
  both comments now say so.

**Everything else the two sweeps print has been read.** `applied`'s remaining twenty-two unmarked
hits are the annotations sessions 417 to 419 wrote in place, where the retired words are kept
deliberately because `doc/md/` is what the gate verifies against, plus four dated ADR records.

## Consequences

- A seventeenth sweep, committed, in `doc/todo/02` §4 and `doc/todo/01`'s list, running in two
  seconds over 43 976 places.
- `conformance::prose::blocks` is public, and `quotations` is one step on top of it. A sweep
  asking what the prose *around* a quotation says needs the block and not only the span, and the
  rule for what a block is — a table row is its own — stays in one place.
- **`check`'s three levels moved and its reading list grew by 27 struck passages**, of which four
  are in clause 13 and three in an informative annex. That reading is owed and is recorded in
  `doc/errata-read.md` and `doc/todo/48` with its number, not left to be rediscovered.
- The general lesson is the one session 590 wrote and this round generalised into an instrument:
  **recording a change and applying it are two acts, and the first is the one that makes the
  second look done.** It has a shape wherever this project writes down what something *used to*
  be — a ledger correction, a retired refusal, a renamed symbol — and the defence is the same
  every time: the record and the claim have to be read together, by something that never gets
  tired.
