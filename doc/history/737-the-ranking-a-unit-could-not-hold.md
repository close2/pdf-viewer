# 737 — The ranking a unit could not hold

The ordering ADR 0349 argued for and left unwritten, built. Parallel round, worktree `r737`, branch
`round-737`. **No pixel moves, no verdict moves and no list changes**: the census and all 966
non-agreeing per-page lines are identical between the run before the round and the run after it.
ADR 0636 has the argument.

## The criterion, stated

722's rule for choosing: *do not invent a criterion where an existing one has an unevaluated
precondition.* The pool has such an item and it had been recorded three rounds running — 722, 727
and 729 each closed on the same line in *Owed*:

> Nothing ranks the pool by how far outside its bound each page sits. ADR 0349 left that ordering
> unbuilt and `outside_by` already computes it per page.

`doc/habits.md` asks a suspect to be ranked by *our worst measurement over the bound it is held to*;
`rank_the_contradicted` orders by distance from the nearest reference, which is the ambiguous
bucket's instrument borrowed unchanged. ADR 0349 took the other ordering by hand, recorded that its
head was a page the printed one never prints, and wrote the argument instead of the code. So the
criterion this round states is that one, made mechanical: **order the pool by how far outside its
bound each page sits, and read the head.** No rasteriser, no new idea — and one thing ADR 0349 could
not have settled, because ADR 0617 did not exist: *which* consensus the ratio is taken over on a page
that carries two.

## What the ordering is

The largest of `Tolerance::accepts`' four ratios, over the members of a consensus that rejects the
page, against **that consensus's** own widened bounds; and where a page carries several such sets,
the **smallest** of their numbers, because since ADR 0617 a contradiction is what every set reaches
and the exemption is only as strong as the set that rejects it least. The measure's name is printed
beside the number — 29× on the differing fraction and 29× on the mean are two different pages — and
the ranking does **not** filter on `complete`, which is ADR 0349's own finding rather than an
oversight.

Calibrated at both ends against figures rounds that could not see this code wrote down (trap 13):
the head is `xobject-image.pdf` page 1 at **127.75×**, ADR 0349's hand-taken number for the same
page, still incomplete and still absent from the ranking beside it; the foot is `issue6069.pdf`
page 1 at **1.00×**, ADR 0606's verdict made of six differing channels of eighty thousand.

## Two findings

**The ranking beside it is blind to the bound most of this pool fails on.** `Distance::of` reduces a
comparison to three ratios — mean, worst tile, structural similarity — and not the differing
fraction. That is right for `Distance`, whose figures are quoted in a hundred notes and have to keep
meaning what they meant, and it is ADR 0242's own defect surviving one level up: that round found
thirty of sixty-eight pages printing a *line* on which every visible number was inside the bound,
and fixed the line by printing the fourth measure while leaving the *order* in the unit that cannot
see it. The gate prints both halves now — how many of the pool are furthest outside on the differing
fraction and the range they span, and how many have a `Distance` at or under 1.0, that unit saying
*nothing here is wrong* about a page the gate has just contradicted. The figures are the run's.

**One page is convicted twice, at half the price.** `issue19633.pdf` page 1 is the only contradicted
page carrying more than one maximal consensus — the remainder of ADR 0617's census, whose other two
populations are the four divided pages and the thirty-six that concur in agreeing with us. Both sets
reject us, so the verdict is untouched, and they price it very differently:

```text
                          the pair agree to   which bounds us at   our worst member   outside by
  {poppler, mupdf}            0.99896           0.9900 (floor)     mupdf   0.97700      2.30x
  {poppler, ghostscript}      0.99088           0.98176            poppler 0.97959      1.12x
```

`mupdf` with `ghostscript` reaches 0.98828, under the class floor, so those two form no set and both
pairs are maximal. The taken pair agrees so closely that `widened_to` leaves the bound at the class
floor; the rival's own wider agreement doubles into a bound admitting nearly all of the same
difference. **Trap 12's arithmetic with its sign made visible — the tighter the pair, the harsher the
bound derived from it** — and the page's standing exemption is worth 1.12×, not the 2.30× its own
line and its note quote. Reproduced by hand through `examples/compare_rasters` on the run's own
artefacts, and by the gate restricted to that page.

## Measured

Three full oracle runs, `PDFREF_CACHE` on the shared warm cache at a **100% hit rate — 6707
reference renders from disk and 0 produced**, so no reference renderer was spawned and no figure
here measures another program. Load ran from 0.7 to 49 across the round, which is what three parallel
neighbours cost; **no timing claim is made and none was needed**, and the verdicts are pixel
arithmetic over cached rasters rather than a race any budget could lose.

The change is `crates/pdf-model/tests/oracle.rs` — a test target, no library code, so no pixel can
move — plus four documents and an ADR. §2's sequence was run whole anyway: `fmt` clean, `clippy
--workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest` **2620 passed, 18
skipped**, doctests clean, the fuzz check clean, and the corpus, oracle, text extraction, both
censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents and conformance gates all green.

Sweeps: `--bin unpriced` **93 failing bounds over 61 pages, 93 named by the note that holds the page,
0 not** — 722's property survives, and it still names `issue6069.pdf` as the one page whose printed
line cannot say what its verdict rests on. `--bin quoted` **170 figures read, 100 confirmed**, one
more of each than the round before, and no hit on either note this round touched — **two figures had
to be moved out of a measure word's three-word reach first**, because a between-reference structural
similarity written beside the word `ssim` reads as a gate figure the gate never prints, which is
729's lesson arriving a second time. `--bin overtaken` **43**, unchanged: `AMBIGUOUS_DIVIDED_CONSENSUS`
was overtaken by this round's own ADR about a page its prose argues, and citing it is the fix.
`--bin pointers` and `--bin quotations` unchanged.

Not a fifth round (`tools/round.sh`), no pixel moved, so §5's binaries were not rebuilt and
`doc/todo/00` step 7 was not re-run — neither has an input that changed.

## Changed

- `crates/pdf-model/tests/oracle.rs` — `rank_the_contradicted_by_the_bound`, `outside_the_bound`,
  `worst_ratio` and `rank_the_pools`; `Examined` gains one field; `outside_by` delegates so the
  arithmetic has one implementation; doc comments on `rank_the_contradicted`,
  `CONTRADICTED_NEGATIVE_LINE_WIDTH` and `AMBIGUOUS_DIVIDED_CONSENSUS`.
- `doc/oracle-and-corpus.md` §3b, `doc/todo/12`, `doc/habits.md`'s ranking rule.
- ADR 0636.
- No ledger row: the round implements no normative requirement and touches no clause.

## Owed

- **`Distance` and this ratio disagree about the pool and nothing reconciles them.** Both are
  printed and each has its argument; which a round reaches for first is `doc/habits.md`'s preference
  and not a derivation.
- **The pool is ordered and not yet read in that order.** Everything above rank ten is diagnosed;
  the long tail just above 1.0 is a population nobody has asked a question of *as* a population,
  which is `doc/todo/12`'s number wearing a different unit.
- Unchanged from 729: a *width* division and a *camp* division are treated alike; `unpriced` cannot
  tell a bound named from a bound accounted for; a voting reference whose raster is constant still
  votes; `freeculture.pdf` page 255; the owner's `git stash drop`.
