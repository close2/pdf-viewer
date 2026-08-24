# 727 — The consensus that was not unique

Four contradicted verdicts turn on the order `Reference`'s variants are declared in, because
agreement is not transitive and the gate counted one maximal agreeing set where the page had two.
Parallel round, worktree `r727`, branch `round-727`. **No pixel moves, no verdict moves and no list
loses a page** — every one of the 1945 per-page verdict lines is identical between the run before
the change and the run after it. ADR 0616 has the argument.

## The criterion, stated

722 answered ADR 0497's sixth criterion pool-wide and said what that left: *`unpriced` cannot tell a
bound named from a bound accounted for; whether each mechanism owns its margin is still the sixth
criterion done by hand.* The sixth criterion has a **control clause** as well as a question, and ADR
0606 exercised it on exactly two pages — *taking us out of the room does not rescue the bound,
because two other renderers fail it too*. That control is a statement about the **consensus** rather
than about our render, and it is the half nobody had asked of the pool. So the criterion this round
states is the sixth's control pointed at the population:

> For a page held contradicted, is the consensus its verdict rests on the *only* reading the
> references had of that page?

No seventh criterion, no rasteriser, and no new idea: the gate already computes every pair.

## What it found

`pdfref::decide` takes the largest set of references that all agree with one another, and its own
comment is right about why that is not *count pairwise agreements*. What it does not follow through
is the consequence: with three references, `a ~ b` and `b ~ c` while `a ≁ c` leaves **two** maximal
agreeing sets, neither contained in the other and neither a majority the other is not. The loop
skipped `subset.len() <= best.len()`, so the second was thrown away without being counted, and which
one survives is the subset bitmask — the order the `Reference` variants happen to be declared in.

The gate now prints how many pages carry more than one, and names those where the sets **disagree
about us**. That second population is four, all four contradicted, and all four would have *agreed*
under the set that was discarded:

| page | taken | discarded | held by |
|---|---|---|---|
| `colorkeymask.pdf` p1 | `poppler` + `mupdf` | `ghostscript` + `mupdf` | `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE` |
| `colors.pdf` p1 | `poppler` + `ghostscript` | `ghostscript` + `mupdf` | `CONTRADICTED_TIGHT_CONSENSUS` |
| `colors.pdf` p2 | `poppler` + `ghostscript` | `ghostscript` + `mupdf` | `CONTRADICTED_TIGHT_CONSENSUS` |
| `issue11403_reduced.pdf` p1 | `poppler` + `mupdf` | `poppler` + `ghostscript` | `CONTRADICTED_SUBSTITUTED_FONT` |

Each was reproduced by hand from the run's own artefacts through `examples/compare_rasters`.

Three of them say something the note holding them did not:

- **`colorkeymask.pdf`** is the cleanest, because it needs no tolerance arithmetic. Its note has
  said since the four-hundred-and-forty-third session that ours and `ghostscript` are byte-identical
  over the whole 595 × 842 raster. `ghostscript` is in the discarded consensus, so that consensus
  accepts us by identity — and a consensus containing a renderer our raster cannot be distinguished
  from cannot contradict us.
- **`colors.pdf`** is the briefing's own lead, and the answer is in a row of 722's own table that
  nobody had read: `ghostscript` and `mupdf` agree with each other **more closely** than the pair
  the verdict rests on — ssim 0.99625 and 0.99278 against 0.99431 and 0.99201 — on three of the four
  measures on each page, and inside every class bound. So the group named for a *tight* consensus is
  decided on two of its three pages by a pair that is not the tightest on the page.
- **`issue11403_reduced.pdf`** goes the other way and that is why no rule was adopted. Its note
  already records that the taken pair *differs by a mark one of them invented*; the rival is the
  pair that agrees **least**, at 4.815% of channels against a 5.00% class bound, and twice that is
  what forgives our 6.24%.

## Decision — count it, name it, move nothing

ADR 0499 set this gate's precedent: a change that moves pages between lists is a decision with its
own ADR rather than a corollary of the round that found the reason for it, and 651's rule about a
tally says the same thing. Three replacement rules are order-independent and each has a hazard —
holding us to every maximal consensus costs nothing today and keeps four verdicts nobody can defend;
`ambiguous` where the sets disagree can be reached by the *looser* pair, which one of the four
already is; taking the tightest set needs an order over four measures that do not rank together.
ADR 0616 has the table and `doc/todo/12` has the work.

## Measured

Load 0.8 for the baseline oracle run and 17 for one later run of it; both produced identical
verdicts, which is what a 100% cache hit rate means — 6707 reference renders from disk and **0
produced**, so no reference renderer was spawned and no wall-clock figure here measures a program.
No timing claim is made and none was needed.

The change reaches `tools/pdfref`, `crates/pdf-model/tests/oracle.rs`, one ledger row and four
documents, so §2's map asks for the core, the oracle gate and the quorra gate — `pdfref` is the
harness of both — plus the conformance gate for the ledger. Not a fifth round (`tools/round.sh`), so
§5's binaries were not rebuilt and nothing was measured that would need them.

`fmt` clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2587 passed, 18 skipped** (two new in `pdfref`), doctests clean, the fuzz check clean, conformance
green, the quorra corpus gate green. The oracle before and after: **1945 pages — 983 agrees, 65
contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no
render**, with all 1945 per-page lines byte-identical across the two runs.

Sweeps: `--bin unpriced` reads a fourteenth note and reports **every** failing bound in the pool
still named by the note that holds its page. `--bin quoted` **168 figures read, 99 confirmed** —
against 167 and 98 before the round — because one stale figure in `CONTRADICTED_SUBSTITUTED_FONT`'s
bound table was corrected to what the gate prints and one figure this round wrote was rephrased so
that a between-references number is not read as a gate figure. `--bin overtaken` unchanged; the
three rewritten notes cite this round's own ADR, which is what keeps them off it. `--bin pointers`
and `--bin quotations` unchanged.

## Changed

- `tools/pdfref/src/lib.rs` — `Consensus`, `Triangulation::consensuses`, `decide` split into
  `maximal_agreements` and `conclude`; two new tests, one of them the calibration against a
  synthetic divided consensus (trap 13).
- `crates/pdf-model/tests/oracle.rs` — `DIVIDED_CONSENSUS`, `divided_by`,
  `name_the_pages_with_a_divided_consensus`, two new `Examined` fields; doc comments on
  `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, `CONTRADICTED_TIGHT_CONSENSUS` and
  `CONTRADICTED_SUBSTITUTED_FONT`.
- `doc/conformance/ledger.toml` — §10.7.4, whose row said *the* consensus pair.
- `doc/traps/oracle-and-references.md` trap 12, `doc/oracle-and-corpus.md` §3b, `doc/todo/12`.
- ADR 0616.

## Owed

- **The rule**, and it is the whole of what this round did not do: ADR 0616's three candidates,
  measured over the corpus rather than over four pages.
- Unchanged from 722: nothing ranks the pool by how far outside its bound each page sits;
  `unpriced` still cannot tell a bound named from a bound accounted for; a voting reference whose
  raster is constant still votes; `freeculture.pdf` page 255; the owner's `git stash drop`.
