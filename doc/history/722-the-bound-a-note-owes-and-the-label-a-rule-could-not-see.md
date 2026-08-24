# 722 — The bound a note owes, and the label a rule could not see

ADR 0497's sixth criterion, pointed at the whole contradicted pool instead of at one bucket of it,
which took building the link five rounds had recorded as owed. Parallel round, worktree `r722`,
branch `round-722`. **No pixel moves, no verdict moves and no list changes**: what changed is one
new sweep, two group notes, two ledger rows, four documents, and a defect in two existing sweeps.
ADR 0606 has the argument.

## The criterion, stated

672 forbade a seventh criterion and said what to do instead — *the sixth, pointed at a
population*. 675 and 680 spent it on the eight middle-bucket groups. What was left was the pool,
and the pool was not readable, because the sixth criterion has a precondition nobody could
evaluate at scale: **which of the gate's four bounds does each contradicted page actually fail
on?** Five rounds closed on that same sentence — 489, 668, 672, 675, 680 — and each of their
thirteen diagnoses began by reading it off a log by hand.

So the criterion this round states is the sixth's precondition made mechanical: *for every page in
the contradicted pool, which bound does the gate fail it on, and does the note holding it name
that measure?* It needs no rasteriser. The gate already prints all four measures beside all four
bounds on the page's own line, so the answer is `Tolerance::accepts`' arithmetic over a log the
round has already run.

## `--bin unpriced`, the twenty-first sweep

`quoted` checks a figure a note *quotes*; its own closing sentence says it cannot ask for one that
is **missing**. This asks. Discriminator: a measure the gate fails one of a note's own pages on,
in a verdict of `CONTRADICTED`, that the note's prose never names. Three rungs, `doc/todo/01` has
the reading, and two design points earned their keep:

- **The population is that verdict and no other**, which is trap 11 and not tidiness: on an
  `ambiguous` page no two references agreed, so the bound beside them decided nothing.
- **Word presence, not `quoted`'s word-plus-figure.** *"All three fail on mean and structural
  similarity"* names both bounds and quotes neither, and that is a good note.

Calibrated per trap 13 against a **live** defect rather than a plant — the finding below was in
the tree, came out at rank 1, and the run is silent with the note written.

## Three findings, and the third is about the instruments

**`CONTRADICTED_TIGHT_CONSENSUS` names one measure in a hundred and sixty lines and it belongs to
one of its three pages.** `colors.pdf` pages 1 and 2 fail on **structural similarity and on
nothing else**; the note's whole account of them is four decimals under the words *bound* and
*ours*, with no unit near them. Its own opening paragraph condemns it, having said of a different
page that "between them sat no account of the metric that fails". The group's sentence — *a bound
no analytic-coverage renderer meets* — was argued from mean distances to a closed form, so it is
now argued in the failing metric, and the result is stronger than the sentence:

```text
                          page 1     page 2
  poppler <-> ghostscript 0.99431    0.99201   the pair: 1 − 2×(1−ssim) is the gate's bound
  ours                    0.98786    0.98024   fails
  hayro                   0.98772    0.98011   fails, by more than ours
  mupdf                   0.98739    0.97943   fails, by more than either
```

**Two renderers that are not this tree fail the same bound and both fail it by more than we do.**
Taking us out of the room does not rescue it.

**`issue6069.pdf` page 1's verdict is six channels of eighty thousand.** The sweep asked which
bound fails it and got *none*: the gate prints `differing 6.55%` against `bound … 6.55%`. At full
precision the bound is 6.5475% and ours is 6.5550% on a 400 × 50 raster — 5244 differing channels
against an allowance of 5238. The page stays contradicted; what is worth recording is that **a
page's own line can stop being able to say what its verdict rests on.**

**And sixty-nine page names were invisible to the nineteenth and twentieth sweeps.**
`overtaken::documents_in` rejects a `.pdf` token preceded by `/`, for a reason written beside it
and true when written — `doc/ISO_32000-2_sponsored_EC3.pdf` is the standard. One round later ADR
0541 gave every submodule-corpus page its corpus's label, `pdfbox/attachment.pdf page 1`,
*because* three of those documents share a bare file name with one of the 974. The label is the
identity, not a path. The exclusion was never needed either: `Corpus` is built from the lists' own
members, so a token no list holds is narrowed away whatever its shape. Measured on one log:

| | before | after |
|---|---|---|
| `overtaken` vocabulary | 320 documents | **340** |
| `quoted` confirmed / unanchored | 86 / 21 | **91 / 13** |
| `unpriced` contradicted pages held by no note | 5 | **0** |

**A rule written to exclude one file excluded a naming convention that did not exist yet**, and it
did so inside the very sweep whose subject is a sentence nothing pointed at when the tree moved
under it.

## Measured

Load was 2 to 25 for the gates and 430 to 450 during the first build — the oracle ran twice, both
times at 100% cache hit rate against the shared warm cache (`PDFREF_CACHE`,
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`), 6707 reference renders from disk and **0
produced**, so no reference renderer was spawned and no wall-clock figure here is a measurement of
a program. No timing claim is made and none was needed.

The change reaches `tools/conformance`, two doc comments in `crates/pdf-model/tests/oracle.rs` and
four documents, so §2's map asks for the core, the conformance gate and — because oracle.rs must
still compile and its ratchets still pass — the oracle gate. Not a fifth round (`tools/round.sh`),
so §5's binaries were not rebuilt and no measurement was taken that would have required them.

`fmt` clean, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest`
**2571 passed, 18 skipped**, doctests clean, the fuzz check clean, conformance **5 + 1 + 1**
green after one self-inflicted TOML break (a `note` value's closing quote is not the row's last
quote — the escaped ones inside it are). The oracle before and after: **1945 pages — 983 agrees,
65 contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no
render**, and every one of the 1945 per-page verdict lines is identical between the two runs.

Sweeps: `--bin unpriced` reports **every** failing bound in the pool named by the note that holds
its page, from one finding before the notes were written. `--bin quoted` reads 167 figures against
160 before — the seven this round added — and confirms **98 against 91**, with its contradicted
count unmoved, so every figure written here is one the gate prints. `--bin overtaken` 44 → 43,
because a rewritten note cites its own ADR. `--bin pointers` and `--bin quotations` unchanged.

## Changed

- `tools/conformance/src/unpriced.rs`, `src/bin/unpriced.rs` — new, the twenty-first sweep.
- `tools/conformance/src/overtaken.rs` — `documents_in` takes a corpus label into the name; two
  tests, one of them new.
- `tools/conformance/src/quoted.rs` — `Measure::words` made public, so two sweeps cannot disagree
  about how this tree spells a measure.
- `crates/pdf-model/tests/oracle.rs` — `CONTRADICTED_TIGHT_CONSENSUS` and
  `CONTRADICTED_SUBSTITUTED_FONT`, doc comments only.
- `doc/conformance/ledger.toml` — §10.7.4 and §9.5.
- `doc/todo/01`, `doc/todo/02` §4, `doc/oracle-and-corpus.md` §3b.
- ADR 0606.

## Owed

- **Nothing ranks the pool by how far outside its bound each page sits.** ADR 0349 left that
  ordering unbuilt and `outside_by` already computes it per page.
- **`unpriced` cannot tell a bound *named* from a bound *accounted for*.** The vocabulary is
  complete now; whether each mechanism owns its margin is still the sixth criterion by hand, and
  it is done for eleven of the thirteen groups.
- Unchanged from 680: a voting reference whose raster is constant still votes; `freeculture.pdf`
  page 255; the owner's `git stash drop`.
