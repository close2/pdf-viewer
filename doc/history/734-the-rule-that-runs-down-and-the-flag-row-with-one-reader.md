# 734 — The rule that runs down, and the flag row whose second sentence had no reader

The seven-hundred-and-thirtieth reports ADR 0567's pairwise ranking with its head spent and 0593's
third rule falling through to a tie. This round measured a successor over the nine rounds of
findings that method left behind, and then used it once.

Date: 2026-08-25.
ADRs: [0627](../adr/0627-the-selection-rule-that-runs-out-and-the-one-that-runs-down.md),
[0628](../adr/0628-the-flag-row-whose-second-sentence-had-no-reader.md).

Touched: `doc/conformance/ledger.toml` (§12.5.1, §12.5.2, §12.5.3's note and its `test` array),
`crates/pdf-model/src/annotation.rs` (one guard, two comments),
`crates/pdf-model/tests/annotations.rs` (one test, one doc comment corrected, one duplicated line
removed), `crates/pdf-model/examples/unknown_subtype_census.rs` (new), `doc/errata-read.md`,
`doc/todo/01`, the two ADRs and this file. **No status moves and no pixel moves; one report is
removed and its whole population is two documents in the crawl and none in the corpus.**

## Half one: what was measured, and what it says

**Nothing cheap ranks the rows inside a family.** Nine rounds' records name the rows each found a
defect in — 21 that were `partial` or `reported` at their own round's base — so eleven candidate
signals were run over each round's ledger *as it stood before that round* and each defect row's
percentile taken. `doc/todo/01` has the table. The best is a count of §-references at 38.6 against
chance's 50; **the incumbent pairwise score is 48.3, which is chance**, because it ranks families
and never claimed otherwise. The hypothesis worth having — *how many commits have rewritten this
note*, since the recurring shape is a correction that reached one sentence and not another —
scored 46.0, worse than the note's word count. So a successor built out of a note's own properties
would be a guess with a table under it.

**The successor is not a ranking over notes.** Eight of the nine rounds found something through
`spec-errata emit`, and ADR 0593 §1 had already named the mechanism without following it: a pair
that survives its reading has still chosen where to look. So:

> **Rank each live ledger row by the errata annotations that fall on it whose issue number this
> tree names nowhere. Reassemble the issue from every clause `emit` files it under, and read the
> issue whole.**

Reconstructed at each of the nine base commits, the unread population **falls monotonically** —
103, 100, 97, 94, 91, 90, 89, 86, 86 issues on a live row, about two a round — where the pairwise
score *rises* on the family a round has just read. Eight of the eleven errata those rounds recorded
were in the population at their base; the other three were errata the tree already names and had
misread, which is the rule's own stated limit. And the head has not moved: §12.8.1, §12.5.2,
§12.7.5.5 and §9.8.1 are in the top six at every one of the nine bases, while the rows those rounds
landed on ranked 1, 4, 8, 17, 17, 22, 32, 39 and 50. §7.6.6 is the one row that left the head, after
the six-hundred-and-ninety-first read two of its issues.

`doc/todo/01` carries the recipe as commands, with the trap in its second step.

## Half two: the rule's first use

§12.5.2 was taken from the head over the two rows tied with it for a reason about the *rule*: its
pages are pages the seven-hundred-and-tenth opened with the same instrument and recorded a count
for. `emit` files seventeen annotation objects there carrying five issue numbers — #1, #22, #124,
#287, #577 — and that round wrote that one of the seventeen is named nowhere. **Three of the five
are.**

- **Issue #1 is §12.5.1's sentence, filed under §12.5.2 because it sits on that clause's page.** It
  strikes the reference in "An interactive PDF processor shall provide certain expected behaviour
  for all annotation types that it does not recognise, as documented in 12.5.2" and writes §12.5.5
  and Table 167's bit positions 1 and 2 in its place. Placed by arithmetic: the strike's two runs
  fall at 183.07–194.11 and 197.83–208.87 from the top of an 841.92-tall page, where `pdftotext
  -bbox` puts `12.5.2,` and `"Annotation dictionaries".`, and the caret sits at the end of the
  second run.
- **What it points at is where the report was.** Table 167's `Invisible` row has two sentences and
  only the first had a reader. The flag *set* suppresses an unknown subtype, which `decided` does
  on Table 171's list; the flag *clear* asks for the appearance stream "if any", and an unknown
  subtype **with** one has been drawn for a long time — but one **without** was reported, with the
  detail *its clause states no geometry*, a claim about a clause a subtype outside Table 171 does
  not have. One expression answering two questions, which is 701's shape for the fourth round
  running: the same arm catches the Table 171 subtypes whose clause really does state no geometry,
  where the sentence is true. Nothing is rendered and nothing is owed now; an annotation with **no**
  `/Subtype` keeps its report, because Table 166 makes the entry required.
- **Issue #124 moves no rectangle and corrected a justification.** Four strikes on Table 166's
  `/AP` bullet replace the index numbers 1, 3, 2 and 4 with 0, 2, 1 and 3 — the same two
  comparisons either way — and reading the bullet showed it is an **and** whose own NOTE says it
  "was changed from 'or' to 'and'", while `annotation::is_empty` is an **or**. Two places said
  Table 166 excused a writer "for exactly that shape"; the excuse is the degenerate point alone.
  The right reason was standing beside the wrong one all along — §12.5.5 scales the appearance's
  box onto `/Rect`, and a scale onto no extent leaves no mark.

**And there is a fourth way an issue goes unrecorded, which is this tree's rather than the
instrument's.** `&#124;` is a Markdown table cell's escaped pipe and the only numeric character
reference anywhere under `crates/`, `doc/` or `tools/` — two of them, in ADR 0484. A search for the
bare number `124` finds them and answers *recorded*. One collision exists in the whole tree and it
is on the issue that went unrecorded. The recipe's grep asks for the `Issue #` prefix.

## The population, measured before the report was taken away

`examples/unknown_subtype_census` is the command the counted-claim rule asks for, with one addition
to `free_text_census`'s shape: a directory argument is walked, so the crawl is one command and one
number instead of a dozen `xargs` chunks and a dozen partial answers. Over the 974: **0**
annotations outside Table 171. Over the crawl: **134 in 4 of the 65 703 documents that open** — 130
`/HeaderFooter`, two `/BJCA:Annot`, one `/CIDFontType0` and one `/CIDFontType2` — of which **132
state a `/AP` `/N`**. The two that do not are the two font subtypes, a producer writing a font
dictionary's `/Subtype` onto an annotation, and they are the whole population of the report this
round removes.

Calibrated per trap 13, two ways: with the new guard disabled the test fails on its own message and
prints the report the tree used to make; with the guard widened to cover an annotation stating no
`/Subtype`, the control for `issue7446.pdf`'s shape fails instead. Both plants removed.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this is not a fifth round, but the change is in `pdf-model`, so §2's map asks
for everything and everything was run. §5's binaries were rebuilt and installed — `round.sh` had
flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green. The only clippy output was
`viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines, which §2 documents as not lints.
`cargo nextest run --workspace` failed once and the failure was this round's: the new example's
module comment opened with a blockquote before it had cited a clause, which
`every_quotation_is_the_standards_own_words` prints by name. The clause is named now and the whole
sequence is green. The lines that spawn a reference renderer were run at a one-minute load average
of 14 and below on 24 cores. **No pixel moves and the oracle says so**: the same verdicts and the
same nine contradicted pages, and the corpus gate's incomplete and unusable lists are unchanged —
which is what a change whose corpus population is zero should look like.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side.

**Two levels moved into a defect bucket on this round's own prose and both were put back.**

- `--bin owed` went 179 unnamed terms over 112 rows to **183 over 112**, and the four are the four
  `/Subtype` names the crawl turned up — names no source in this tree carries, under a `partial`
  row, which is what that sweep reads as a debt named in a word. They are a *census result* rather
  than a debt, and the repair is `CLAUDE.md`'s own rule rather than a rewording to dodge the sweep:
  the command prints all four and the row now says so instead of listing them. Back at 179, and
  `--bin inapplicable`'s cousin count came back with it — §9.7.2's `CIDFont` had gained §12.5.3 as
  a cousin off the same sentence, 225 to **224**.
- `spec-errata applied`'s read-first list went 10 to **11**, and the mechanism is this round's
  exactly: §12.5.2's note has quoted Table 166's *is ignored* since long before, and naming
  **#287** in this round's list of the five issue numbers made the note a place that *names* the
  erratum whose wording it quotes. The note says now that #287 sharpens it to *shall be* ignored,
  which is true, was missing, and moves the hit into the demoted bucket — 91 reading like a
  correction quoting the wording it retired, from 90. Back at 10.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after ← before: `counts` 7875 ← 7822 sentences with 412 ← 411 attributed counts, **58 "no
such way" and 4 places counting one family twice both times**; `quotations` 6200 ← 6183 document
quotations over 942 ← 939 documents with **diverging unchanged at 37**, and 1930 ← 1925 ledger
quotations with **diverging unchanged at 2**; `tables` 6510 ← 6472 sentences and 2430 ← 2425 key
citations with **absent unchanged at 100, contradicted denials at 6 and keyless at 58**; `pointers`
8176 ← 8155 with **absent unchanged at 131 and undefined at 13**, and 137 ← 132 symbol pointers,
every new one resolving; `owed` 3849 ← 3832 terms with the figures above; `overtaken` 560 ← 558
decision records with **43 overtaken unchanged**; `blockers`, `entries`, `unread`, `overstated`,
`capabilities` and `callers` byte-identical. `spec-errata moved` is **byte-identical**, `check`
differs only in the line numbers this round's insertions shifted, and `applied` grew by 23
comparisons over a population that went 55 406 places read to 55 521, which is this round's own
writing.
