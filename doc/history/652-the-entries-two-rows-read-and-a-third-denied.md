# 652 — The entries two rows read and a third denied

Five rows off the top of the blame ordering, three defects, and one instrument measured and not
built. Every defect is the fifth failure shape inside a single family: the right answer already
written in a neighbouring row, in this project's own words, while the wrong row was rewritten
around the mechanism it denied.

Date: 2026-08-22.
ADR: [0481](../adr/0481-the-denial-a-parent-generalises.md).

Touched: `doc/conformance/ledger.toml` (§9.7.5, §9.8, §10.4.2, §10.4.2.2, §10.4.2.4, §10.4.2.5),
`doc/todo/01-ledger-partial-rows.md`, the ADR and this file. No code.

## The order the three instruments gave

**The eighteenth sweep first**, as `doc/todo/01` now says. `--bin overstated` printed **8
contradictions over 170 parent rows asserting 125 terms**, 7 of them marked, and the single
unmarked hit is §12.7's `/AP` against §12.7.5.5's "Table 236's `/P` is deliberately not read here"
— the third rung, a denial about another entry entirely, which is the rung the module doc prints
rather than drops. **Nothing new**: the eight are 645's eight, verdict for verdict. It took a fifth
of a second and it is the right thing to run first for exactly that reason.

**Then the blame ordering**, re-derived rather than taken (616's rule): 851 commits, **240**
`partial`-or-`reported` rows with a blamed note. 648 left the list beginning at §10.4.2.4 and
§10.4.2.5 and both were still ranks 1 and 2. Read: ranks **1, 2, 3, 5 and 13** — §10.4.2.4,
§10.4.2.5, §9.7.5, §9.8, §10.4.2. 620's rule chose within the band for the seventh time running,
and it chose right three times out of three: every defect below is a row whose stated reason is a
claim about *this codebase*.

**Then enumeration**, and its yield this round was a **bound** rather than a row. Walking
`content/ext_gstate.rs`'s Table 57 arms against every ledger sentence denying one of that table's
entries finds §10.4.2.4 and nothing else live — §10.6.5's `/HT` and `/HTO` and §8.6.7's `/OP`,
`/op` and `/OPM` are true denials, §11.5.1's `/AIS` and §9.6.4's `/FontBBox` are corrections
quoting what they retired. Walking `ColourSpace::to_cmyk`'s arms corroborates §10.4.2.3: a `Gray`
space falls through to the `rgb_to_ink` arm, so that clause's own grey-to-CMYK arithmetic still has
no caller. Neither found a row the ordering could not, and that is worth recording as honestly as
648's opposite result — what enumeration bought here is the word *only*.

## The three defects

**§10.4.2.4 said Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` were "read by nobody".** All four
have been read since the four-hundred-and-twenty-sixth session: `content/ext_gstate.rs` sets
`black_generation_stated` where an `/ExtGState` states any of them, and the flag withholds
§11.4.7's page-group pair and §11.6.6's group-scoped press and names the reason — "an /ExtGState
states Table 57's black generation or undercolour removal, which §11.7.5.3 puts inside the
conversion into the space". So the entries decide a **report** rather than nothing, and what is
genuinely owed is that they are not *evaluated*. **The answer stood in two other rows the whole
time**: §8.4.5's has said all four are read since the five-hundred-and-sixty-fifth, §11.7.5.3's has
said they are read as a condition and not evaluated since the four-hundred-and-twenty-seventh — and
this row's own last two paragraphs were rewritten by the four-hundred-and-twenty-sixth and -seventh,
about the very mechanism its earlier sentence denied.

The row's attribution went with it. It sent the reader to "§11.6.6's departure, reported there",
and the report a person actually sees names §11.7.5.3. Its `test` array now carries the witness:
`transparency_groups.rs::the_blending_space_is_the_one_in_force_rather_than_the_one_declared`,
whose `/GK gs` states a `/BG2` and is the one lever on that fixture that keeps a `/DeviceCMYK`
group undrawable.

**§9.8 said "[t]he dimensional metrics are read by nobody".** `/Ascent` and `/Descent` are read —
`pdf_font::vertical_extent` builds the band a selection highlight is laid over,
`variable_text::Metrics::read` builds a form field's baseline, each believing the pair only where
it could be a measurement of a face — which §9.8.1's row has said since the
three-hundred-and-seventy-eighth session. The parent *denying* what its child asserts. The
enumeration behind it: no source under `crates/` names `/FontBBox`, `/Leading`, `/CapHeight`,
`/XHeight`, `/StemV`, `/StemH`, `/AvgWidth`, `/MaxWidth`, `/FontFamily` or `/FontStretch` outside a
census, so the corrected sentence is exact in both directions. The list itself now stays in
§9.8.1's row alone — the duplication is what let the sentence drift.

**§10.4.2 was `partial` "for what two of the four conversions below owe".** Three: §10.4.2.4 is
`partial` too, for a debt of a different kind — not a direction nobody takes but a pair of
functions a file may state and this tree does not evaluate. `--bin counts` cannot see it, because
the cardinal governs *conversions* rather than one of the ledger's own words for a row.

## The two kept rows, and both were 620's third shape

*The row is right and its evidence is not*, for the eighth round running.

**§10.4.2.5** cited `colour_paths.rs::a_cmyk_colour_is_the_same_however_it_is_drawn`, which asserts
that `k`, `scn` and an image's samples agree with **one another**. That is trap 6's one-conversion
rule; it is true whichever conversion is used and cannot see this row's subject at all. The test
that can was already in the tree, cited by nothing:
`colour.rs::the_conversion_into_ink_round_trips_through_the_classic_formula_and_not_the_cube`
writes the clause's own formula out, pins §10.4.2.4-then-§10.4.2.5 exact on pure red and on
§10.4.2.3's grey — which is what makes the standard's own pair cost an opaque mark nothing — and
pins the ink cube as a different answer, `1 0 0 rg` coming back at (237, 28) rather than (255, 0).
Both cited now, and the second is what fails if anyone swaps the cube for the clause.

**§9.7.5** cited one embedded-CMap test for a sentence whose other two thirds are the Identity pair
and the 239 predefined files. §9.7.5.2's two are cited beside it now, `predefined.rs` joins the
`code` array, and **the 239 has a command**: it is `pdf_font::predefined::PREDEFINED`'s length,
which `build.rs` writes from a directory walk, so `ls data/cmaps | wc -l` re-derives it as 242 less
the three things beside the data that are not data (`LICENSE_ADOBE`, `SHA256SUMS`, `PROVENANCE.md`).
The row's `partial` still rests on §9.7.5.4's c) alone, and the three siblings above it are
`implemented` above notes naming nothing owed.

## The instrument measured and not built

The obvious next sweep after ADR 0475 is that sweep with the sign flipped — a parent **denying**
what a descendant asserts — and it costs almost nothing, because both vocabularies and the splitter
exist. Measured first, which is trap 11: **170 parent rows with descendants, 14 denied
term-mentions between them, 3 contradicted, all three noise.** Against 125 asserted terms in the
forward direction. The asymmetry is structural rather than a small sample — a row asserting a
capability *enumerates*, which is what makes it a summary; a row denying one *generalises*, and a
generalisation names no term to match.

**This round's own §9.8 is the proof.** It is exactly the mirror shape, in the family the mirror's
own noise points at, and the mirror would not have printed it: "the dimensional metrics" is neither
a `/Key` nor a `Table NNN`. Reaching it needs a program to decide what an English category noun
governs, which is the judgement every sweep here refuses by construction. Not built; the numbers
are in `doc/todo/01` so the next round to have the idea reads them instead of deriving them.
ADR 0481.

## `spec-errata`, and a filing two clauses off for the second round running

`emit` over all fourteen documents before writing. **Nothing at all under §10.4.2.5, §10.4.2,
§9.7.5 or §9.8**; §9.8.1 carries five errata, none of which touches a metric this round read
(#11 is `/FontName` and Type 3 fonts, #178 a `/FontWeight` NOTE, #190 the sign of `/Descent`,
#152 `number` → `integer`, #474 a `/FontWeight` range).

**Issue #640 prints under `## 10.4.2.4` and belongs to §10.4.2.2 and §10.4.2.3**: it strikes the
*grey* of `red = grey` and of `black = 1.0 − grey` and writes *gray*, the spelling both clauses use
everywhere else. The annotations sit on the page whose last line is §10.4.2.4's heading, and `emit`
files by the clause the outline puts a page in — 648's finding, unchanged, and the reason the
briefing says `emit` prints an annotation's text while *where it points* is a separate question.
Recorded in §10.4.2.2's row; it changes nothing this tree does.

## One thing the round found by running a sweep over its own writing

The first draft of §9.8's correction read "What nothing reads as an input is the metrics …", and
`--bin overstated` went from 8 contradictions to **12**: the part matched `ASSERTIONS`' `reads` on
a word boundary and matched no `unread::CLAIMS` phrase, so a denial was scored as an assertion and
four of Table 120's entries printed as false hits against §9.8.1. The fix is not to widen the
vocabulary — it is shared with the second sweep on purpose — but to write the denial in the
ledger's own idiom, which is `unread`. Rewritten, the sweep is back to its standing 8. **A sweep is
also a check on the sentence a round is about to add**, and it is cheaper than a reviewer.

The same run of `--bin unread` caught a second one: a correction quoting its own retired wording
put `/Ascent` and `/Descent` back on that sweep's list, because the retired phrase and the entry
names sat in one sentence. Splitting them into two sentences — and moving a full stop outside its
bold markers, which is what `sentences` splits on — took it off again.

## Gates

The change is **documents only**, so `doc/todo/02` §2's map asks for the core four lines, the
conformance gate, and the sweeps a moved ledger owes. `tools/round.sh` called this a fifth round
on a session count of 650, which is 650 and 651 not yet being merged; the parent round named the
narrower set and that is what ran.

`cargo fmt --all --check` exit 0. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`
exit 0. `cargo nextest run --workspace` **2381 passed / 17 skipped**. `cargo test --workspace --doc` clean,
every result line `ok`.
`RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` exit 0.
`cargo test -p conformance` green — **875 rows**, and the status breakdown is unchanged at 436
implemented, 222 partial, 18 reported, 78 inapplicable, 8 writer-side, 113 out-of-scope. **No
`silent` row**, 0 unreviewed. No status moved, which is right: every row here keeps what it claimed
and gains the evidence that reaches the claim.

All twelve committed sweeps run, because the ledger moved. `overstated` — 170 rows with
descendants, 125 terms, 52 corroborated, **8 contradicted with 7 marked**; the same set as 645's.
`unread` — 69 rows claim, **182** keys, 45 confirmed, 137 quoted over 59 rows, 68 by the row's own
code: four keys fewer than before this round, and the four are §10.4.2.4's black-generation entries
coming off the list. `quotations` — 1722 ledger quotations, 1307 verbatim, **1 diverging**, and that
one is §8.4.4's and was there before. `counts` 356 attributed counts, 4 places counting one family
twice; `tables` 2146 attributed key citations, 6 denials the table contradicts; `pointers` 6851
paths with 118 absent and 13 undefined symbols, every one of the thirteen standing and none of them
this round's; `blockers`, `capabilities`, `entries`, `inapplicable`, `owed` and `callers` printed
their standing populations and no new hit.

**Overlap with the parallel rounds: none seen.** 650 and 651 were briefed to touch a row or two;
nothing this round wrote is outside §9.7.5, §9.8, §10.4.2, §10.4.2.2, §10.4.2.4 and §10.4.2.5.
