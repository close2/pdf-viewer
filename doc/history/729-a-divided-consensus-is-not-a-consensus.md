# 729 — A divided consensus is not a consensus

The rule 727 left owed. Agreement between references is not transitive, so a page can carry two
maximal agreeing sets that reach different verdicts about our render; the survivor was the order
`Reference`'s variants are declared in. **A verdict is now one every maximal consensus reaches, and
where they reach different ones the page is `ambiguous`.** Four pages move, all
`contradicted` → `ambiguous`, and nothing else in 1945 moves. Parallel round, worktree `r729`,
branch `round-729`. ADR 0617 has the argument.

## The argument, and where it comes from

ADR 0005 states two rules, and a divided page satisfies **both** conditions at once — two or more
references agree with each other, *and* the references disagree among themselves. It never said which
wins, because nothing had noticed a page could do both; that silence is where the enumeration order
got in. Its own justifications settle it, and the load-bearing word is one article: *two unrelated
implementations reaching **the** answer*. Where they reach two, each backed by a coincidence of
exactly the same improbability and neither set contained in the other, mutual agreement — the only
ranking this design has — ranks neither. So the second rule governs: there is no correct answer to
hold us to, because the references offered two.

It does not dissolve the ordinary contradiction. The usual 2-of-3 page has one maximal set and its
dissenter agrees with nobody, which is what ADR 0005 ranks. What is new is a dissenter who is himself
in a consensus.

## The control, which is what kept it off "the rule that flatters us"

ADR 0497's sixth criterion has a control clause — *taking us out of the room does not rescue the
bound, because two other renderers fail it too*. Pointed at the room instead of at us it becomes a
computation over the gate's own between-reference comparisons: **put each reference where our render
stands, and ask what the maximal consensuses it is not a member of conclude about it.**

On all four divided pages the set that decided the verdict contradicts a **voting reference that is
itself a member of a maximal consensus** — `ghostscript` on `colorkeymask.pdf` and
`issue11403_reduced.pdf`, and on both `colors.pdf` pages each of `poppler` and `mupdf` contradicted
by the other's set. So on a divided page no renderer in the room, ours or a reference's, is outside
every reading the references have. `colorkeymask.pdf` needs no tolerance arithmetic at all: our
raster is `ghostscript`'s to the byte, so the worst tile of 5.03 against 5.00 that contradicts us
*is* `ghostscript`'s distance from `poppler`.

That disqualifies holding us to every set — it would condemn the implementations whose agreement is
the evidence it runs on. Taking the *tightest* set is disqualified twice over: it is undefined (four
measures that do not rank together, and on `colors.pdf` each candidate pair wins a different subset
on each page), and it ranks readings by exactly the quantity trap 9 says shared code, a shared ICC
file and a shared published standard manufacture — the tighter pair here being the two that read the
same 187 484 bytes of `default_cmyk.icc`.

## Measured

Two full oracle runs in one sitting, load under 1, `PDFREF_CACHE` on the shared warm cache at a 100%
hit rate — 6707 reference renders from disk and **0 produced**, so no reference renderer was spawned
and no figure here measures another program.

| | agrees | contradicted | ambiguous | our geom. | ref. geom. | not comparable | no render |
|---|---|---|---|---|---|---|---|
| before | 983 | **65** | **832** | 3 | 2 | 42 | 18 |
| after | 983 | **61** | **836** | 3 | 2 | 42 | 18 |

A line-by-line diff of all 962 non-agreeing per-page verdicts shows four changed lines and no others;
`agrees` is unchanged, so nothing entered or left it either.

**The rule is not a one-way ratchet — it is one-way today, and the gate now prints the number that
says so.** Of the 41 pages carrying more than one maximal consensus, **36 carry sets that concur in
agreeing with us**: every one is an agreement a moved pixel could cost by dividing them, which is the
direction none of the four is in. The other five are contradicted, and one of those —
`issue19633.pdf` page 1 — carries two sets that both reject us and therefore stays contradicted,
which is what makes the condition *disagreement* rather than *number*.

**One of the four is not flattered by the rule and the group note says so.**
`issue11403_reduced.pdf` page 1 divides by *width* rather than by camp: `poppler` is in both sets, we
sit 6.24%, 6.14% and 5.20% of channels from the three references — further from every one of them
than any two are from each other — and what takes the page out of `contradicted` is that
`{poppler, ghostscript}`'s own 4.815% spread doubles to a bound admitting us. Nothing about this
render improved. Its cap-height diagnosis stays where it was measured.

Gates: the §2 sequence whole. `fmt` clean; `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` clean; `nextest` **2596 passed, 18 skipped** (one new in `pdfref`);
doctests clean; the fuzz check clean; corpus, oracle, text extraction, both censuses, dates, xmp,
jpeg2000, quorra corpus, fixed documents and conformance all green.

Sweeps: `--bin unpriced` — **93 failing bounds over 61 pages, 93 named by the note that holds the
page, 0 not**, so 722's property survives the move. `--bin quoted` **169 figures read, 99 confirmed**,
and none of the four notes this round touched is among the hits — one figure it wrote was rephrased
before committing, because a between-renderer structural distance written to five places reads as a
gate figure the gate never prints. `--bin overtaken` names none of them either; they cite this
round's own ADR. `--bin pointers` and `--bin quotations` unchanged.

Not a fifth round (`tools/round.sh`), no pixel moved, so §5's binaries were not rebuilt and
`doc/todo/00` step 7 was not re-run — neither has an input that changed.

## Changed

- `tools/pdfref/src/lib.rs` — `decide` returns `Outcome::Ambiguous` where the maximal consensuses
  disagree about us; `Triangulation::divided`, `Consensus::agrees_with_us`; the module's triangulation
  rule gains the third bullet; `two_maximal_consensuses_can_disagree_about_us` now asserts the
  ambiguous outcome and the class bounds beside it, and `two_maximal_consensuses_that_concur_still_reach_a_verdict`
  is the new fixture holding the discriminating half (trap 13).
- `crates/pdf-model/tests/oracle.rs` — `AMBIGUOUS_DIVIDED_CONSENSUS` replaces `DIVIDED_CONSENSUS` and
  is chained into `diagnosed_ambiguous()`; `divided_by` names both sets and the page's own verdict
  line carries them; the census line gains the concurring-agreement count;
  `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE` is empty and keeps its §10.7.4 reading,
  `CONTRADICTED_TIGHT_CONSENSUS` keeps `issue7891_bc1.pdf`, `CONTRADICTED_SUBSTITUTED_FONT` keeps
  eleven pages and the cap-height table.
- `doc/conformance/ledger.toml` — §10.7.4, whose row said both `colors.pdf` pages had moved to
  `CONTRADICTED_TIGHT_CONSENSUS`.
- `doc/traps/oracle-and-references.md` trap 12, `doc/oracle-and-corpus.md` §3b, `doc/todo/12`,
  `doc/todo/00` (a third way a page arrives in that bucket).
- ADR 0617.

## Owed

- **A *width* division and a *camp* division are treated alike**, and `issue11403_reduced.pdf` is the
  witness. Separating them needs a reason a rival set's widening should count for less than the taken
  set's, and there is none today: a page whose *only* consensus is that rival is judged by exactly
  that bound and agrees. It meets `doc/todo/12`'s own question in `widened_to`.
- Unchanged from 727: nothing ranks the pool by how far outside its bound each page sits; `unpriced`
  still cannot tell a bound named from a bound accounted for; a voting reference whose raster is
  constant still votes; `freeculture.pdf` page 255; the owner's `git stash drop`.
