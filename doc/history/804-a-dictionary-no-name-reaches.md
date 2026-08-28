# 804 — A dictionary no name reaches is a widget, not a field

The errata selection rule's fourteenth use, and the first run on the family guard the thirteenth
added to step 3 — which changed the head, so the repair paid on its first outing. Without the guard
the full ranking still tops out at §14.13.10 with six annotations, four of them Annex A's, exactly
the false head 799 diagnosed; with it that row holds two and both rankings top out at three. Step 4
prefers the settled row, the third use's tie-break picks the one whose caret puts a requirement
level into a table cell, and that head turned out to be Table 341's — clause 13, inside
`CLAUDE.md`'s exclusion, confirmed and paid nothing. **The walk downward then reached the live
head, §12.7.4.1, and it paid code.** Issue #28 strikes Table 226's `/T` requirement level and
writes *Optional*, because §12.7.4.2 already states what a dictionary with no `/T` is and the cell
had forbidden the case the paragraph describes. `view::widgets_by_field_name` quoted that
paragraph and applied it to a *kid*: at the root of `/AcroForm /Fields` there is no ancestor to
supply a name, so every dictionary no `/T` reached was keyed under the **empty** name — one field
made out of annotations sharing nothing, one control offered to a host over all of them, and one
value written into all of them by `ViewState::set_field`. Ten issues left the population; the code
moved.

Date: 2026-08-28.
ADR: [0736](../adr/0736-a-dictionary-no-name-reaches-is-a-widget.md), the number the briefing
reserved.

Touched: `crates/pdf-model/src/view.rs` (`widgets_by_field_name`, a new `ancestry`, `walk` and
`qualified_name` on `Option`, three new tests, and a stale `//` quotation of Table 224 found
beside them), `crates/pdf-model/examples/unnamed_field_census.rs` (new),
`doc/conformance/ledger.toml` (§12.7.3, §12.7.4.1, §12.7.4.2, §14.2 and §14.9.4, reformatted by
its own binary), `doc/errata-read.md` (fourteenth-use section and the fifth blindness),
`doc/todo/01` (the section), the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **71 were named nowhere** at this
round's base — the thirteenth use's closing arithmetic (73 less its two verdicts) reproduced by the
greps rather than read off the record, the fifth consecutive use at which base and derived closing
figure agree. The multi-issue parse counts 310 and 73 and reproduces the same way. Ten gain
verdicts here, so the closing population is 61, re-derived after the record was written.

Twelve annotations fall in the state the guard counts separately rather than attributing, and they
reproduce the thirteenth use's own split: 4 under Annex A, 2 under Annex H, and 6 under clauses 2
and 3, which the ledger starts after.

**The head the unrepaired instrument would have given, measured rather than assumed.** Both
rankings were run twice, with the family guard and without, on the same `emit` output and the same
two greps. Without it: §14.13.10, `inapplicable`, six. With it: six rows tied at three, §14.13.10
holding two. A round that read the recipe's arithmetic as settled would have spent its reading on
an `inapplicable` row for annotations belonging to an annex the ledger has no row for.

## What the eight rows turned out to be

`doc/errata-read.md` has every rectangle placed against `pdftotext -bbox` before its verdict was
written. In brief:

- **Issue #28 (§12.7.4.1, Table 226's `/T`)** — the finding, below.
- **Issue #618 (§12.7.4.1, Table 226's `/AA`)** — the cell's cross-reference gains Table 199
  beside §12.6.3. Every entry of Table 199 is an ECMAScript action, which §12.6.3's row excludes
  by name, so the amended cell points at a table this tree already declines whole.
- **Issue #166 (Table 341's `/Configurations`)** — `(Optional;` becomes `(Required;` and the array
  becomes *non-empty*. The settled head by the tie-break, and inside `CLAUDE.md`'s clause-13
  exclusion. Table 341 is §13.7.2.3.1's; the outline filed it under §13.7.2.3.2.
- **Issue #367 (§14.2)** — "This feature has been deprecated since PDF 1.4" becomes *considered
  unnecessary since PDF 1.4 and was deprecated in PDF 2.0*, separating two judgements the sentence
  had run together. The `shall` §14.2's `inapplicable` rests on is untouched.
- **Issues #356 and #364 (§14.9.4)** — the clause's own EXAMPLE: `Actual Text` respelled
  *ActualText*, and the example numbered *2* with a new EXAMPLE 1 above it. Filed under §14.9.5.
- **Issues #687 and #699 (§F.3.1)** — an annex example's `stream` and `>>` the right way round,
  and a closing bracket in a cross-reference.
- **Issue #331 (§F.3.2, filed under §F.3.3)** — linearisation may be applied to a file of version
  *1.2* or greater rather than 1.1. A floor on a writer.
- **Issue #153 (§F.3.3)** — three sentences into Table F.1's `/Linearized` cell about how its
  version number is read. A producer's rule about an entry this reader does not consult.

## The finding, and the half the corpus witnesses

§12.7.4.2's sentence is unconditional and the code applied it one level too low. Two corrections
in one function:

- **A node whose ancestry states no `/T` is left out of the name table.** It is still drawn —
  `crate::annotation` needs no field — and handed to nobody, which is the path `form::fields`
  already documents for a widget the field tree does not reach. The test is the **entry** rather
  than the string, so a `/T` of zero length still names a field.
- **A `/Fields` entry stating a `/Parent` takes its ancestors' name.** §12.7.3 makes that array
  the document's root fields, "those with no ancestors in the field hierarchy", so such an entry
  has contradicted the clause — and Table 226's `/Parent` is the file's own answer to which field
  it belongs to, which §12.7.4.1's inheritance walk has trusted for `/FT`, `/Ff` and `/V` since it
  was written. A recovery, recorded as a choice in §12.7.4.2's and §12.7.3's rows.

`examples/unnamed_field_census` walks `/Fields` with `pdf_syntax` alone — never through the
function under test, trap 8 — and over `doc/pdf.js` and `doc/corpora`: **1239 documents open**,
176 state an `/AcroForm /Fields`, 19 hold a field dictionary with no `/T`, **1** holds one no
ancestor names, and **0** hold one in `/Fields` with no `/Parent` to recover from. The witness is
`doc/pdf.js/test/pdfs/opt_demo.pdf`, which lists a radio group's two buttons in `/Fields` instead
of the field above them: its table entry goes from `"" -> [22, 23]` on `main` to `"veg" -> [22,
23]` here, measured both ways. So the corpus witnesses the recovery and not the refusal, and the
refusal is pinned by a fixture — which is what a population of zero asks for.

## Calibration

Trap 13, above the commit that makes the change, three plantings, both directions:

| planted | fails |
|---|---|
| a nameless node keyed under the empty name again | `a_root_field_that_nothing_names_is_not_a_field`, `left: [""]` |
| the `/Parent` ancestry not consulted for a root | `a_root_field_takes_the_name_its_parent_chain_states`, `left: None` |
| a name dropped for being **empty** rather than absent | `an_empty_partial_name_is_still_a_name`, `left: None` |

The third is the over-correction: a reader that asked whether the name came out empty instead of
whether the entry was there passes the first two and loses a field whose partial name is the empty
string. Each planting was reverted with `git checkout -- crates/pdf-model/src/view.rs`, one file
named rather than a directory.

## The fifth blindness

`check`'s blind spots were four after the twelfth use. Issue #327 is a fifth and it is the first
the *other* half of the instrument shares: it closes §7.3.3's numeric grammar with railroad
diagrams, `check` sees nothing struck, and `emit` prints two `FileAttachment` annotations whose
whole content is their titles — the substance is an **attached file**, and both instruments read
text. The eight-hundredth session read the diagrams anyway (ADR 0733), so nothing is owed; the
shape is now written down in `doc/errata-read.md`, where the blindnesses live, because the next
erratum of this kind would be invisible in exactly the same way.

## Sweeps

§4, before and after, the before half taken in a **separate pristine worktree at the base commit**
so that each sweep binary resolved its own tree from its own `CARGO_MANIFEST_DIR` — 799's method
note, applied. Fourteen of the conformance sweeps ran in both; `retired` declines without the nouns
a round's corrections were about and none of this round's were retirements; `quoted` and
`unpriced` were run against this round's own oracle log and touch page-list notes, which this
round did not.

Nine sweeps printed a delta and every one is this round's own text or a moved line number:
`capabilities` a line number in `view.rs`; `counts` five line numbers in `doc/todo/01` and seven
more attributed counts, all seven in the "attributed to a clause with no rows below it" bucket the
sweep already calls noise, with the family-agrees and no-such-way figures unmoved; `inapplicable`
and `owed` word frequencies up by one file, which is the new example; `overtaken` one more decision
record, the overtaken count unmoved; `pointers` more pointers with **absent unchanged at 98** and
undefined symbols unchanged at 13; `quotations` nine more document quotations and four more ledger
ones with **diverging unchanged at 38 and 2** — so every quotation this round wrote is verbatim or
shares too little to be one; `tables` eleven more key citations, all eleven in the "the table
agrees with" bucket, absent unchanged at 101; `unread` the same `/Fields` hit with the new example
added to its witness list. No new finding.

## Gates

Full §2 sequence — the change is in `pdf-model`, which the map puts under everything. Three
sibling rounds were live in their own worktrees throughout, which §2 warns about; nothing in the
timings looks like it. `fmt` clean after one reflow of a chained call; `clippy --workspace
--all-targets` under `RUSTFLAGS="-D warnings"` silent. nextest 2776 passed, 18 skipped — **after
one run in which `pdf-model::outlines::an_outline_resolves_against_the_page_tree_once` failed and
then passed alone and passed again in a full run.** It asserts a *ratio* of two wall-clock spans —
resolving five hundred outline destinations against one page-index search — so it is the shape §2's
quiet-machine paragraph is about, and three neighbours were building. Recorded rather than filed:
the test is a real bound on an algorithm and the reading it gives under load is a measurement of
the load. Doctests:
24 result lines, all ok. Fuzz `check` clean; both trap-10 binaries built before the gates that
need them. Corpus: 974 documents in 2.9 s — 0 unopenable, 8 locked, 2 encrypted beyond us, 6
pageless, 66 incomplete, 0 slow, **and the same 66 with `main`'s `view.rs` swapped in**, which is
the check worth having on a change that touches a widget's field. Oracle: 1945 pages in 141.1 s —
983 agree, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not
comparable, 18 no render; exit 0, no ratchet moved. Text extraction: 4 passed in 44.6 s,
10971/11163 words in bounds (98.28%), 487 of 508 documents fully in bounds. Selection census 6.5 s,
1000/1011 words selected (98.91%). Accessibility census 37.2 s, 102853 elements, 0 untagged pages
given structure, no ratchet moved. Dates 1514 of 1545 conforming; XMP 318 of 319 read; JPEG 2000 14
codestreams byte-identical. Quorra: 957 pages in 26.4 s — 932 agree, 22 differ, 3 refused, 17 not
comparable. Fixed documents: 41 checked, 0 absent. `cargo test -p conformance` green, including the
submodule guard.

## For the next round

- **The live head this round paid is off the ranking, and the settled plateau it tied with is
  read.** Six rows tied at three and all six now carry verdicts, so the fifteenth use starts from
  a population of 61 and a head it will have to re-derive rather than inherit.
- **The family guard is confirmed rather than merely built.** It has changed a head once; a use
  that finds the two rankings agreeing is not evidence against it.
- **§12.7.4.1 stays `partial`** for the reason its own note gives — the ancestry bound the clause
  forbids — which this round did not touch.
