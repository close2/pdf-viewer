# 647 — The budget a page had, and did not spend

`doc/todo/03`'s chunk for the ninth round running, and the last one: the **3944 crawled documents
no chunk had ranked**. The SafeDocs crawl is finished. The defect at the head of it is not a bound's
value but what the bound *did* when it was reached — a tiling that could afford four thousand sites
was given none.

Date: 2026-08-22.
ADR: [0477](../adr/0477-the-budget-a-page-had-and-did-not-spend.md).

Touched: `crates/pdf-model/src/content/pattern.rs` (`tile`, `affordable_span`, `MAX_TILES`'s
comment), `crates/pdf-model/tests/tiling.rs` (one new test),
`crates/pdf-model/tests/hostile_budgets.rs` (one doc comment),
`doc/conformance/ledger.toml` (§8.7.3.1), `doc/checks/fixed-documents.toml` (two rows),
`doc/todo/03` (header, §26's successor, §27, the population section), `doc/todo/49` (one bullet),
the ADR and this file.

## The chunk

**Archives `7557` and `7803` whole and all eighty-one twenty-four-member archives — 3944
documents**, on §16's instrument reused rather than rewritten: page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink minus
the lightest live reference's. **5 minutes 10 seconds** at twelve workers, at a load average between
5 and 13. **3924 rows produce a number and 20 do not.**

**Checked before it was trusted.** Both binaries built (619's lesson), no stray worker in
`release/examples/` (624's), §20's check run first — 33 checked, 0 absent, green — and the four
documents ADRs 0438 and 0471 name re-measured through the four-renderer instrument, reproducing
640's own table to the ten-thousandth (`1407194.pdf` +0.032, `6573247.pdf` −0.172, `7557734.pdf`
+0.025, `2145632.pdf` +3.560) before anything was read.

**With this, 65 944 of 65 944 are ranked.** There is no "n crawled documents unranked" successor for
the first time since the six-hundred-and-third session, and `doc/todo/03`'s header says so.

## The defect, and why it is the refusal's shape rather than the budget's value

**`7803372.pdf` −12.251**, the deepest row and the deepest of the last three chunks, reporting
`LimitReached { limit: "MAX_TILES" }` and nothing else. A French school-canteen menu whose *Jeudi*
and *Vendredi* columns are hatched by twenty-eight `/PatternType 1` dictionaries of the shape
`/BBox [0 0 1.6 1.6] /XStep 1.6 /YStep 1.6`, each cell one `Do` of an 8 × 8 one-bit image. At 1.6
units a side the two columns want something over twenty thousand sites apiece against a `MAX_TILES`
of 4096, so the fill was refused — **and refused entirely**, because the check sat in *front* of the
cell's interpretation.

§8.7.3.1 puts the requirement on the processor: "[w]hen performing painting operations such as S
(stroke) or f (fill), the PDF processor shall paint the cell on the current page as many times as
necessary to fill an area." A budget is this project's answer to a file asking for more times than
there is time for, and it decides *how many* — not *whether*. Painting the cell no times is the
furthest a processor can get from that sentence, and the four thousand sites the bound had already
been sized to afford are the producer's own marks. → **9.083 → 11.096** against three references
between 21.3 and 22.4.

**The asymmetry was legible in the ledger the whole time, which is the sharper half of this.**
§8.7.3.1's own row already records §7.8.2's prefix rule for the *cell's content stream* — a cell
that decodes part-way is replicated as far as it got (ADR 0359) — while the *lattice* threw its
prefix away. Two things make a tiling and the rule had reached one of them. ADR 0343's additive-or-
substitutive test decides it the same way: a site is one more copy of the cell, not a different
picture of it.

**The value and the worst case are untouched**, and that is what keeps this separable from
`doc/todo/49`'s open question: a fill cost at most 4096 sites before and costs at most 4096 sites
now, so no page can do work the old check would have refused. The remaining four fifths of that
hatching are still owed to the mechanism `doc/todo/49` asks for, and that file's "[n]either is a
defect today: both bounds refuse loudly" is amended rather than deleted — it was true and it was
not the whole sentence.

## What moved

**The reach is bounded by the code and confirmed by measurement, and the two are worth keeping
apart.** The diff is entirely inside the `total > MAX_TILES` branch, which is the branch that raises
the report — so a page whose raster can change is a page that reports `MAX_TILES`. That is a proof,
not a sample, and it is why the confirming run is 8011 documents rather than 65 944.

**The population is measured rather than inferred**: `examples/open_one` over every one of the
65 944 says **48 documents report `MAX_TILES` on page one**, over 35 archives — the same 48 ADR 0271
counted, re-derived rather than copied out of a document.

**Confirmed over 8011 documents, twice, over our own panel** (631's rule): this chunk's 3944, the
four previously-ranked archives holding such documents (`0100`, `1530`, `6204`, `7188`), all 48, and
every row of `doc/checks/fixed-documents.toml`. **42 rows move and every one of them reports
`MAX_TILES`.**

| document | ours before → after |
|---|---|
| `4650/4650000.pdf` | 41.262 → **49.214** |
| `1530/1530064.pdf` | 4.411 → **8.635** |
| `7803/7803372.pdf` | 9.083 → **11.096** |
| `0669/0669450.pdf` | 39.133 → 41.125 |
| `1899/1899774.pdf` | 60.284 → 61.393 |
| `1530/1530303.pdf` | 22.713 → 23.782 |
| `7188/7188511.pdf` | 24.718 → 25.676 |
| … 33 more between +0.0002 and +0.89 | |
| `1530/1530611.pdf` | 118.3530 → 118.3499 |
| `6081/6081466.pdf` | 152.481 → 152.426 |

**Forty gain ink and two lose a little**, and the two are the change working rather than against it:
a hatch laid over something darker than the page takes ink *away* when more of it is drawn.
**Six of the 48 do not move at all** — the affordable prefix falls where nothing of it is visible —
which is the difference between a population and a reach, and the reason the second is measured.

**A forty-third row moved and was not this change.** `1530980.pdf` is a 30 MB document that takes
about 9 s to draw; under a load average above 100 it lost the harness's 30-second budget in the
first pass and not in the second, so it read as `-` → 88.55. Re-measured alone, before and after
agree at **88.5497** to four decimals. That is 626's lesson on *our own* instrument rather than on a
reference's: a wall-clock budget measures the machine as well as the tree, whoever is holding it.

## The rest of the head, and the second instrument

**Two chunks running, the head is this tree's own departures rather than a misread clause** — and
this one over the population's remainder rather than a sample of it. Seven of the first eleven rows
were read to a cause; four were placed by where they *cannot* be and are named in `doc/todo/03` §27
so the next round does not re-derive them.

- **ADR 0308's abutting marks, on the strongest witness this project has.** `7803184.pdf` −6.381
  and `7803350.pdf` −6.639 are pages a producer states as thousands of thin image strips — **2217
  `Do`s on the first, 1882 of them 0.96 units tall, stepped alternately 0.96 and 0.72**, each a
  627-wide 1- or 2-row `DCTDecode` band. Each strip covers a fraction of a device pixel row and
  §11.3.7.3 composites the fractions as a union, so the page comes out a quarter of the way to white
  along a third of its rows. **Measured, not asserted**: against `pdftoppm` the gap is −6.381,
  −2.601, −1.011, −0.501 at 1×, 2×, 4×, 8× — halving per doubling, which is ADR 0308's
  boundary-over-area signature and nothing else's. Nobody had to construct it.
- **Trap 9's colour family with the references disagreeing among themselves**: `7650021.pdf` −5.951,
  whose dark ground reads (69, 69, 71) here, (65, 64, 66) in `mutool`, (57, 53, 54) in `pdftoppm`;
  `7557305.pdf` −3.988, a `/Separation` resolving to (15, 83, 143) on §10.4.2.5's arithmetic against
  (0, 75, 152) and (0, 82, 139). Three answers, not two. Not called a diagnosis.
- **The positive head is 613's `poppler`-draws-nothing note**: 18 rows above +10, **10 of them with
  `poppler` under a third of the heaviest reference**. The exceptions run the other way —
  `7557508.pdf` +16.686 and `7557287.pdf` +10.999 have `mutool` light while the other three agree
  within 0.3.

**The second instrument was run over the whole population for the first time** (640's habit, at
population scale): `open_one` over all 65 944, 24 minutes. **65 703 open** (241 do not), **65 659
have a first page**, and of those **720 report anything about page one** against **64 939 silent** —
98.9%. By leading class: 250 a font with no outline for the codes shown through it, 157 a
transparency group, 103 an image, 55 a budget, 51 text, 44 a damaged `/Contents`, 19 a stroke
coloured by a tiling pattern, 10 an annotation with no appearance and no geometry in its clause;
138 report more than one thing. The budgets are 48 `MAX_TILES`, 5 `MAX_FORM_DEPTH`, 4
`MAX_OPERATIONS`, one `MAX_STATE_DEPTH`, one `MAX_OPERANDS` — and **no page in 65 944 exhausts the
clip or soft-mask tables**.

## The erratum

`spec-errata emit` over all fourteen documents before writing. §8.7.3 and its three subclauses carry
**two annotations and both were already read** by session 632 and are in the ledger row: Issue #428
(`Review`/`Accepted`, p. 235), a caret adding "(implementation dependent)" to "unspecified and
unpredictable", and Issue #294 (`Review`/`Completed`, p. 236) inserting "stream" into Table 74's
caption. **Issue #428 is load-bearing for this change** rather than a formality — it is what makes
"the first 4096 sites" a prefix the standard leaves to the implementation rather than an arbitrary
choice. No unread erratum on the family.

## Gates

The full §2 sequence, because the change is in `pdf-model` and because `tools/round.sh` says this is
a fifth round. Load average is stated beside each because three other rounds share the machine and
626 is about exactly that.

| | | load |
|---|---|---|
| `fmt`, `clippy --workspace --all-targets` under `-D warnings` | clean, silent | 8.9 |
| `cargo check --manifest-path fuzz/Cargo.toml --bins` under `-D warnings` | clean, exit 0 | 8.9 |
| `nextest --workspace` | **2365 passed, 17 skipped**, 66.1 s | 7.5 |
| doctests | 1 passed, 1 ignored, 0 failed | 7.5 |
| corpus | **974 documents in 3.4 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 68 incomplete, 0 slow** | 10.8 |
| oracle | **1794 pages in 36.5 s — 907 agree, 66 contradicted, 786 ambiguous, 13 not comparable** | 10.8 |
| `render-quorra` corpus | **957 pages — 932 agree, 23 differ, 2 refused, 17 not comparable**, 36.8 s | 28 |
| `fixed_documents` | **35 checked, 0 absent, 35 rows** | 28 |
| `text_extraction` | 4 passed; **10 969/11 163 words in bounds (98.26%)**, 486 of 508 documents | 28 |
| `selection_census` | green, 0 selections differing from the readback | 28 |
| `accessibility_census` | green; 1502 of 1558 structured pages answer, 876 of 876 untagged | 28 |
| `dates`, `xmp`, `jpeg2000` | green | 28 |
| `cargo test -p conformance` | green (5 + 1) | 28 |

**Two clippy lints this round's own writing introduced**, both caught by `RUSTFLAGS="-D warnings"`
and neither by a plain run: an `arithmetic_side_effects` on `budget / columns` — answered with
`checked_div` and a comment saying why the divisor cannot be zero rather than an `expect` — and a
`doc_markdown` on `SafeDocs` inside a test's doc comment.

**The oracle's four tallies are identical to 644's**, which is what a change confined to 48 crawled
documents should do to a gate that walks `doc/pdf.js`: no page of the 974 states a tiling over the
bound, so nothing there could move.

**§5's binaries were built and installed**, because this is a fifth round.

## Owed

- **`doc/todo/49`'s mechanism**: the other four fifths of `7803372.pdf`'s hatching, and the 48. The
  count is still not the cost.
- **Four rows of §27's head placed but not settled** — `7557122.pdf` and `7557305.pdf` want trap 9's
  colour probe; `7557015.pdf` and `7803013.pdf` want `uncovered_share`.
- **`doc/todo/11` item 5**, which now has `7803184.pdf` as a witness nobody constructed: closing
  ADR 0308's conflation would be worth 6.4 of 255 on a page at fit.
- **A different corpus.** This one is finished; SafeDocs' issue-tracker set is 31 GB in six archives
  and nobody has fetched it.
- **The owner's session**: `tmp/pi.pdf`, for 628.
