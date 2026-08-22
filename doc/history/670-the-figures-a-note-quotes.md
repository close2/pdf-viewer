# 670 — The figures a note quotes, and the list one was not above

The question 665 and 668 left between them, settled by counting first. Parallel round, worktree
`r670`, branch `round-670`. **No pixel moves**: what changed is a new sweep, the corrected
figures of thirteen page-list notes, one note moved back above its own list, two notes' clause
reading, §10.3.1's ledger row, trap 1 and two catalogue entries. ADR 0495 has the argument.

## The count, which is what decided it

665 rejected *a note whose measurement disagrees with the gate* on a measurement of two tokens.
668 then found two stale figures by hand in one group and called the debt open. Both were arguing
about the same population, so this round measured it over the whole vocabulary — five spellings
across the four measures the gate prints, `differing` in either word order — before building
anything.

**32 of 124 page-list notes quote at least one, 137 figures in all**; over the oracle's own file,
25 of 98 notes and 116 figures, of which the gate confirmed 34 on the first run. A quarter of the
notes and over a hundred figures, and it is the quarter a round opens. So: a gate, and 665's
count was right about its two tokens and wrong about the population.

## What was built

`cargo run --release -p conformance --bin quoted -- <the oracle's log>` — the twentieth sweep, the
fifteenth to be a program, and the first whose right-hand side is another gate's *output* rather
than the tree's sources. It renders nothing: the oracle already prints all four measures for every
page it does not call agreement, and a round that touched a note has run it. Under a second.

Three rungs — contradicted with a confirmed figure beside it, contradicted at the gate's own
precision, contradicted only after rounding — and under every hit **what the gate says instead**,
nearest value first, so a note is corrected off the run rather than out of reasoning.

## The plant failed first, and that was the finding

`CONTRADICTED_UNEXPLAINED`'s `mean 0.17` was set back to `mean 0.22` — 665's own defect — and the
sweep said nothing. That list is `[&str; 0]`: the page moved to `CONTRADICTED_TIGHT_CONSENSUS` in
662 and the note kept four paragraphs about a page it no longer holds. **A sweep anchored to list
membership is blind to exactly the notes nothing else points at.** The anchor was widened to the
documents a note's prose argues, the plant was named, and the discriminating calibration was a
*confirmed* figure perturbed: `worst tile at 6.73` → `7.73`, named on rung 1 with `6.73` offered
first, counts moving by exactly one and returning on `git checkout --`. Planted before any
correction (645's rule).

## What the first run found

Figures corrected across **thirteen** notes; over the same report, contradicted went **69 → 48**
and confirmed **34 → 66**. One was the opening sentence of the note **665 itself corrected**,
stale again five sessions later because ADR 0492 changed what a group composites — and 665's
instrument could not see it, for the reason above. Four were *bands* over a population,
re-derived from the gate's own per-page lines, and one of those re-derivations found
**`freeculture.pdf` page 255 standing outside the band its note claims** — mean 16.63 where
nothing else in the book reaches 9 — never opened, and now named.

**And one thing no figure could have said.** The sweep reported a *one*-page `DeviceN` group
quoting a band `mean 3.51 to 9.93, worst tile 20.94 to 48.31`. Opened: forty lines diagnosing one
paper under fifteen names were sitting above that group's `const`, two groups away from the list
they open, whose own note therefore began mid-argument at *# And a second document*. A doc comment
attaches to whatever declaration follows it and both were page lists, so nothing said so. That is
a **fourth** way for a note to be wrong — after its name (0480), its reading, and its figures
(0491): *a note attached to the wrong list.* Moved back, and its band re-derived over the 155
pages it is about.

## The clause half

668 found `CONTRADICTED_CALRGB_TO_SCREEN` arguing from §10.3.1's "beyond the scope" sentence and
stopping one sentence short of the `shall`. The same half-read stood in two more notes, and
`overtaken`'s reading list had one of them at its head:

- `AMBIGUOUS_CALRGB_TO_SCREEN` — the same document's other eight pages.
- `AMBIGUOUS_ICC_MATRIX_PROFILE` — the same conclusion **and a false quotation**: it attributed to
  §10.3.1 the words "[t]he characteristics of the output device", which are §10.4.2.4's. A silence
  was asserted about one clause out of another clause's words.

`spec-errata emit` was run over the standard first: §10.3.1 carries no erratum under its own
heading, so both corrections state the `shall` in prose, as 0494's did.

## The gates

`doc/todo/02` §2 whole — this is a fifth round — run from `tmp/gates-670.sh`, every line exit 0
except the first `cargo fmt --all --check`, which caught a test body this round had just written
and was fixed and re-run green. `cargo clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` and `cargo fmt --all --check` were both re-run **after** the last edit
and both exit 0.

- `cargo nextest run --workspace` — 2437 tests, 2437 passed, 17 skipped, 75.7 s.
- corpus — 974 documents in 2.4 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless,
  68 incomplete, 0 slow.
- **oracle** — 1794 pages in 35.2 s: 908 agree, 65 contradicted, 786 ambiguous, 2 our geometry,
  13 not comparable, 18 no render. Identical, page for page, to the run this round's corrections
  were taken from.
- text extraction — 99.2% over 974 documents, 99.8% against PDFBox's frozen output over 40.
- selection census 974, accessibility census 988, xmp 319 documents, quorra 957 pages compared:
  933 agree, 22 differ, 2 refused, 17 not comparable. `fixed-documents`: 40 checked, 0 absent.
- `cargo test -p conformance` — 875 subclauses: 436 implemented, 224 partial, 18 reported,
  76 inapplicable, 8 writer-side, 113 out-of-scope, **0 unreviewed**. No status moved; §10.3.1
  gained a witness.

The reference cache was **copied** rather than shared — `PDFREF_CACHE` points at this worktree's
own copy of the 2.2 GB `pdfref-cache` — so the oracle's 908 agreements are not a read of a
directory three neighbours are writing.

**§5's binaries were deliberately not installed**, on 663's and 667's argument: this is a parallel
round told not to push or merge, `target/` is the *main* tree's, and putting an unmerged branch's
binaries where a person runs them while three rounds build beside it is what §5 exists to prevent
rather than to require. The merge round owns it.

## Changed

- `tools/conformance/src/quoted.rs`, `src/bin/quoted.rs` — new; `src/lib.rs` registers it.
- `tools/conformance/src/overtaken.rs` — `Note` keeps its own lines and its list's pages, so one
  scanner serves both sweeps.
- `crates/pdf-model/tests/oracle.rs` — thirteen notes' figures, one note moved, two notes' clause
  reading. Doc comments only.
- `doc/conformance/ledger.toml` §10.3.1 — the third and fourth home of the half-read.
- Trap 1 in `doc/traps/pixels-and-rasterisers.md` — the by-hand tell has an instrument, the
  emptied-list hole, and the fourth way a note is wrong.
- `doc/todo/01`'s *sweeps as commands* and `doc/todo/02` §4.

## Owed

- The other gates' notes: `render-quorra/tests/corpus.rs` and `text_extraction.rs` keep notes in
  the same shape and print figures in their own words. The module is general; the binary is scoped.
- Thirteen figures sit in notes the report names no page of and cannot be judged.
- The notes `overtaken` still names, and the ones that cite no ADR at all.
- Which evaluation of a matrix-shaper profile ICC.1 licenses is still unestablished.
