# 725 — The `shall` that was paid, and the row that never heard

The ledger's `partial` rows read as a family for the fifth round running, on ADR 0538's method for
the eighth block, with the pair chosen by ADR 0567's search under 0593's third rule — **take the
strongest pair the previous round named and did not read.** ADR 0600 §1 named two and read one, so
this round took the one it left: **§10.7.4 ~ §10.7.5**, a family no round of this method had opened.
The pair's two rows quote each other's clauses and disagree outright about what those sentences leave
standing.

Date: 2026-08-25.
ADRs: [0610](../adr/0610-the-rule-that-was-paid-and-the-row-that-never-heard.md),
[0611](../adr/0611-a-claim-written-outside-its-sweeps-syntax.md).

Touched: `doc/conformance/ledger.toml` (§10.7.5), `crates/pdf-model/examples/absence_audit.rs` (one
claim block), `crates/pdf-model/tests/oracle.rs` (one page-list note), `doc/errata-read.md` (two
table numbers and the calibration), `doc/adr/0496` (one marked quotation), `doc/todo/01`, the two
ADRs and this file. **No status moves, no pixel moves, and no report is added or removed.**

## Why the pair, and the ranking run

The search was run rather than read out of a document, with 710's two rules and 716's third. **The
family order did not move** — §12.5 heads it, §12.8 second, §12.7 third — and once the clause-level
parents are stripped the three strongest pairs anywhere in the ledger are §12.4.4 ~ §12.4.4.1,
§12.8 ~ §12.8.3 and §10.7.4 ~ §10.7.5. The first is what 720 read and the second is inside a family
705 read, so the third rule points at the third, exactly as it pointed 720 out of §12.5.

The pair scores on each row quoting the *other's* clause — §10.7.4's zero-width-stroke exemption
stands in both rows, and §10.7.5's NOTE about the thinnest line the device can draw stands in both.

## The findings

- **§10.7.5's row said a `shall` was unpaid and the four-hundred-and-fifty-fifth session had paid
  it.** The row narrates ADR 0268's measurement of `tiny-skia`'s hairline at exactly one device pixel
  — a 45° `1 w` rule carrying 141.42 of its own 200 where the fill of the same outline carries 177.44
  — and ends "Not paid … `doc/todo/11`". ADR 0285 paid it: `at_or_under_the_quantum` is `<=`, the
  turned ladder's `1.0` rung is gated and its doc comment says it "is the rung that used to fail",
  §10.7.4's own row records the whole of it, and **`doc/todo/11` — the pointer the sentence ends
  with — heads that item "closed (ADR 0285)"**. Worse than stale: **the row's two reasons for not
  paying are the two ADR 0285 decided the other way**, so it argued from correct facts to the
  opposite conclusion the tree had already reached in four places. ADR 0101's shape and 710's, and
  no sweep in this tree could print it — the defect is a *conclusion*.
- **Two places counted `/SA true` and disagreed, and neither named a command.** The ledger row said
  49 and `oracle.rs` said 30, neither naming a population, and a growing corpus cannot take 49 down
  to 30. A name census cannot arbitrate, because the clause fires "[w]hen stroke adjustment is
  enabled" and a `/SA false` states the entry too — so `absence_audit` gains a block asking the
  **value** by the two routes its §10.7.2 `/FL` block already uses. **50 of the 974 pdf.js documents,
  60 of the 1251 curated, 15 207 of the 65 944 crawled files it reads.** The same run reprints
  §10.7.2's recorded `/FL` figure of 88 exactly, which is the backwards planted-witness control, and
  names `bug1743245.pdf` — the page the group is about — as a witness.
- **The same wrong table number twice more, in a form the ninth sweep counts instead of printing.**
  `doc/errata-read.md` attributed `/FL` to **Table 58**, the path construction operators, where the
  graphics state parameter dictionary is **Table 57** — the confusion §10.7.5's own row records
  having carried until the three-hundred-and-eighty-ninth. Table 58 states no entries, so the
  citation lands in `--bin tables`' *keyless* count rather than among its absences; three rows down
  the same document writes the number with no key beside it, which is not an attribution at all.
  Calibrated per trap 13, one instrument over three states of the cell: with 58 the sweep prints
  nothing, with 166 it prints the citation **and names Table 57 as the table stating the key**, with
  57 it agrees. `oracle.rs` had the third form of the same failure in the same family — a pointer
  written as the prose "the handover's list of departures", for a list the handover has not held
  since it became an index, which `--bin pointers` cannot resolve because it is not a path.

**`emit` over the §10.7 pages found nothing new**, which is the answer the round wanted: the only
erratum in the family is Issue #371 on §10.7.2's flatness, already recorded in that row and in
`doc/errata-read.md` — and reading it is what turned up the table number.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this is a fifth round, so §2 ran whole and §5's binaries were rebuilt and
installed — `round.sh` had flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.
The only clippy output was `viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines, which §2
documents as not lints. **The three lines that spawn another program were held until the load fell**,
which is §2's own rule: they ran at a one-minute load average of 3 on 24 cores, down from 38, and the
extraction cache took 958 hits and no misses.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them, and
`quoted` and `unpriced` against this round's own oracle log as levels rather than deltas.

**One level moved into a defect bucket on this round's own prose and it was put back**: `--bin
tables`' absences went 100 to 101 on a calibration table in ADR 0611 whose cell attributes `/FL`
possessively to Table 166 — the sweep firing on a *record of running the sweep*, which is the shape
`doc/HANDOVER.md` warns about for an ADR's markdown cells. The cell names the numbers without the
possessive now and the level is back at 100, with the hit list byte-identical to the run before the
edits.

**One overtaken hit left and one arrived, and both are the sweep working.**
`AMBIGUOUS_STROKE_ADJUSTMENT` came *off* rung 1 because the rewritten note cites ADR 0610, which is
that sweep's own cheapest rule; `AMBIGUOUS_SUB_PIXEL_LINE_WORK` arrived on rung 2 because ADR 0610
names `bug1743245.pdf` and that note mentions the page once, in a cross-reference to the group that
owns it — the documented noise shape, and nothing in ADR 0610 touches what it says.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after ← before: `counts` 7744 ← 7706 sentences with 411 ← 410 attributed counts, **58 "no
such way" and 4 places counting one family twice both times**; `quotations` 6117 ← 6093 document
quotations over 921 ← 919 documents with **diverging unchanged at 36**, and 1922 ← 1920 ledger
quotations with **diverging unchanged at 2**; `tables` 6416 ← 6404 sentences and 2395 ← 2392 key
citations with **absent unchanged at 100 and contradicted denials at 6**, keyless 58 ← 56 on this
round's own two quotations of the number it retired; `pointers` 8040 ← 8017 with **absent unchanged
at 131 and undefined at 13**; `owed` 3805 ← 3802 terms over 223 `partial` rows with **179 unnamed
over 112 rows unchanged**, §10.7.5's own row going 13 to 16 terms with every one still named;
`overtaken` 550 ← 548 decision records with 42 ← 43 overtaken; `blockers`, `entries`, `unread`,
`inapplicable`, `overstated`, `capabilities` and `callers` unmoved but for line numbers.
`spec-errata check` and `moved` are **byte-identical** before and after, and `applied`'s three
comparison counts — 90 quoting a replacement, 10 matching both sides, 171 quoting what an erratum
struck — are unchanged over 1643 comparisons and a population that grew from 54 583 to 54 655 places
read, which is this round's own writing.

The before-and-after pair for the three errata commands needed the tree at `HEAD`, taken with **named
paths** and the two new ADRs moved aside first — 720's lesson, and the submodule links were checked
intact afterwards.
