# 716 — The fifth fill, and the count that was four in four places over five arms

§12.5's `partial` rows read as a family for the second round running, on ADR 0538's method for the
sixth block, with the pair chosen by ADR 0567's search. The pair the ranking pointed at turned out
to be clean — and it had still chosen the pages, because `spec-errata emit` over the tables both
rows argue from found an erratum that settles a reading this crate had taken and never written down,
a count that said *four* in four places over five arms, and the explanation for a sweep hit that had
been standing at the head of its own output.

Date: 2026-08-24.
ADR: [0593](../adr/0593-the-fifth-fill-and-the-count-that-was-four-in-four-places.md).

Touched: `doc/conformance/ledger.toml` (§12.5.6.6, §12.5.6.7 and §12.5.6.8),
`crates/pdf-model/src/appearance.rs` (one expression in `draw_ending`, three doc comments),
`crates/pdf-model/tests/annotations.rs` (one test renamed and widened),
`doc/errata-read.md`, `doc/adr/0192` (one marked correction), `doc/todo/01`, the ADR and this file.
**No status moves, no pixel moves, and no report is added.**

## Why §12.5 again, and the third rule for reading the ranking

The search was run rather than read out of a document, with 710's two rules applied — strip the
clause-level parents, let the pairs choose the reading. Its *order* is unchanged: §12.5 heads it,
§12.8 second, §12.7 third.

**What is new is a property of the instrument.** Measured with one instrument over the ledger before
710's commit and over the ledger now, §12.5.2 ~ §12.5.5 goes from 17 shared rare sequences to 21 and
§12.5's total from 221 to 225; nothing else moves. That pair is exactly what 710 read, and rewriting
two rows in one round's voice leaves more shared rare vocabulary than it found. **A family the last
round read scores higher for having been read** — so the pair to take is the strongest one the
previous round *named and did not read*, which was §12.5.4 ~ §12.5.6.8 at 24.

**That pair was clean.** Both rows quote §12.5.4's sentence naming the four subtypes whose `/BS`
supplies width and dash alone, and enumerating the `/BS` cells of every annotation table confirms the
division they rest on: Tables 176, 177 and 191 say "the annotation's border" and Tables 178, 180, 181
and 185 do not, so `Border::simulated` is asked by exactly the three subtypes whose entry is a border.
What the pair bought was the pages.

## The three findings

- **Table 179 fills five of its ten, and four places in this tree said four.** Errata Collection 3
  **Issue #515** (`Review`/`Completed`) is a `Caret` adding "filled with the annotation's interior
  colour, if any" to `RClosedArrow`'s row — placed by arithmetic: the caret occupies 514.41–521.81
  from the top of an 841.92-tall page and `pdftotext -bbox` puts exactly one line there, the second
  line of that description, whose last word ends two points before the caret's left edge.
  `Ending::filled` has returned true for that shape since ADR 0192, on the reading that a shape drawn
  "in the reverse direction from" a filled one is the same shape — right, and never written down as a
  reading. The prose was wrong four times over: the function's doc comment, the test's doc comment,
  the test's *name* and §12.5.6.6's ledger row. **`check` could not have printed it** — a `Caret` with
  no `StrikeOut` has no struck text to match, the first of that command's three blindnesses and the
  first met since 710 named the third.
- **The fill was decided twice, and trap 13 is what found it.** Calibrating the renamed test by
  removing `RClosedArrow` from `filled` — **the test passed.** `draw_ending` consults `filled` for the
  square, the circle and the diamond, and the arrowhead arm three matches below asked its own
  `closed && interior != Colour::None`. The two expressions agree on all ten names and always have, so
  nothing drawn moves; what the duplication cost is the reach of a correction, and the two shapes
  outside `filled`'s reach were the two the erratum is about. The arm asks `fill` now and the
  calibration fails on `/RClosedArrow` by name.
- **The standard says our instrument is a word short.** **Issue #513** is an EDITOR NOTE, not a
  change: the ISO PDF's own row height obscures the end of `OpenArrow`'s sentence. `doc/md/` carries
  the damage — that cell ends at "an open" and `ClosedArrow`'s begins with "arrowhead" — which is why
  `--bin quotations` prints ADR 0192's copy of the sentence as a diverging document span, at the head
  of its own output. The ADR is right, the conversion is short, and the specification says so.
  `doc/md/` is **not** patched (ADR 0593 §6): it is the instrument the conformance gate reads, and
  editing it would make the gate's agreement a property of our edits.

**Issue #524 was read and moves nothing**, which is worth as much as a finding: a one-word strike of
`/RD`'s type `rectangle` for `array`, in Tables 177, 180 and 187. Nothing here calls `/RD` a rectangle
and `differences` reads four numbers in the clause's order. A one-word strike is under `check`'s
four-word floor, so `emit` found this one too.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this is not a fifth round, but the change is in `pdf-model`, so §2's map asks
for everything and everything was run.

**The machine was not quiet and it cost a false failure**, which is §2's own rule demonstrating
itself. With three parallel rounds running and a load average above 20 on 24 cores,
`pdf-model::outlines::an_outline_resolves_against_the_page_tree_once` failed — a test that states its
bound as a *ratio* against a search it performs itself, precisely so that a slow machine cannot fail
it, and which a loaded one can still perturb because the two measurements are taken at different
moments. It passes alone, and the whole workspace passes. The lines that spawn a reference renderer
were held until the load fell rather than run against it.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.
The oracle and the text line were run with a one-minute load average of 5 on 24 cores. §5's binaries
were rebuilt and installed, which `tools/round.sh` had flagged as missing from this worktree
altogether.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
**Three levels moved into a defect bucket on this round's own prose and all three were put back:**

- `--bin quotations`' diverging **documents** went 36 → 41, on five copies of the erratum's own
  wording written with the full stop that ends its sentence — the published rows have none, so each
  matched 7 of 8 words and then diverged. Four are now quoted without the terminal stop and the
  fifth, a blockquote in `doc/errata-read.md` reproducing the sentence `doc/md/` is short of, is not
  reproduced at all: the file says so instead and points at the one copy that has to exist. Back to
  36, and the ledger's diverging spans never moved from 2.
- `--bin owed` went 177 unnamed over 112 rows to 178 over 113, and the A/B — the ledger checked out
  and put back, one instrument, one sitting — named §12.5.6.7. The term was **`EDITOR`**, from
  "an EDITOR NOTE": an all-capitals word is an identifier by the sweep's own rule, and the one file
  in the tree that carries it is under `tools/conformance`, which `NOT_SCANNED` excludes. The row
  says "an editor's note" now and the level is back.
- The same A/B found a typo the sweeps cannot see, ``/RD`s`` for ``/RD`'s``, and it is fixed.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after ← before: `counts` 7614 ← 7578 sentences with 409 ← 407 attributed counts, **58 "no
such way" and 4 places counting one family twice both times**; `quotations` 5988 ← 5971 document
spans with **diverging unchanged at 36**, and 1908 ← 1905 ledger spans with **diverging unchanged at
2**; `tables` 6327 ← 6313 sentences and **2372 key citations both times, with absent unchanged at
100 and contradicted denials at 6**; `pointers` 7915 ← 7897 with **absent unchanged at 131 and
undefined at 13**, the eighteen new ones being `doc/md/` and the rung it sits on; `owed` 3783 ← 3758
terms over 223 `partial` rows with **177 unnamed over 112 rows unchanged**; `overtaken` 539 ← 538
decision records with **44 overtaken unchanged**; `blockers`, `entries`, `unread`, `inapplicable`,
`overstated`, `capabilities` and `callers` all unmoved. `spec-errata check` is **byte-identical**
before and after, and `applied`'s three comparison counts — 90 quoting a replacement, 10 matching
both sides, 171 quoting what an erratum struck — are unchanged over a population that grew by 19
places naming an erratum, which is this round's own writing.
