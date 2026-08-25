# ADR 0617 — A divided consensus is not a consensus

Status: accepted, 2026-08-25. Session 729. Replaces the enumeration order ADR 0616 found with a
rule: **a verdict about our render is one every maximal consensus reaches, and where they reach
different ones the page is `ambiguous`.** Four pages move, all from `contradicted` to `ambiguous`,
and nothing else in 1945 moves.

## The question this had to answer

ADR 0616 established the fact and deliberately took no decision from it. Agreement is not
transitive, so with three references `a ~ b` and `b ~ c` while `a ≁ c` leaves two maximal agreeing
sets, `{a, b}` and `{b, c}` — neither contained in the other, neither a majority the other is not.
`pdfref::decide` counted one, and which one survived was the order `Reference`'s variants happen to
be declared in. On four corpus pages the two sets reach different verdicts about us, all four were
contradicted, and all four would have agreed under the set that was discarded.

So: **when the references divide into two maximal agreements, what is the honest verdict?** It is a
question about what a consensus is evidence *of*, and the answer has to come from ADR 0005 and from
`CLAUDE.md` principle 5 rather than from which candidate rule is kindest to this tree.

## The argument

ADR 0005 states two rules and a justification for each:

> - **Two or more references agree with each other and we differ** → a real bug. Two unrelated
>   implementations reaching the same answer is strong evidence it is right.
> - **The references disagree among themselves** → an ambiguous corner of the specification.
>   Recorded, but *not* a failure: there is no correct answer to hold us to.

**On a divided page both conditions hold at once**, and ADR 0005 never said which wins, because
nothing had noticed that a page could satisfy both. That silence is where the enumeration order got
in. The justifications settle it, and the load-bearing word is one article: *two unrelated
implementations reaching **the** answer*. Where they reach **two** answers, each backed by a
coincidence of exactly the same improbability and neither set contained in the other, the first
rule's evidence does not point anywhere. Mutual agreement is the only ranking this design has, and
between two maximal sets it supplies none. The second rule is what is left, and it fits: there is no
correct answer to hold us to, because the references have offered two.

**This is not a carve-out that dissolves the ordinary contradiction.** The usual 2-of-3 case —
`a ~ b` with `c` far from both — has exactly one maximal set. The dissenter there agrees with nobody:
one program alone against a coincidence of two, which is what ADR 0005 ranks and what a lone bug
looks like. What is new is a dissenter who is *himself* in a consensus.

### The control, and it is a measurement rather than a preference

`CLAUDE.md` principle 5 forbids choosing a rule because it flatters us, and ADR 0497's sixth
criterion has the instrument that keeps it honest: a control clause — *taking us out of the room does
not rescue the bound, because two other renderers fail it too.* Pointed at the room instead of at us,
it becomes a question with a computable answer: **put each reference where our render stands, and ask
what the maximal consensuses it is not a member of conclude about it.**

Run over the four divided pages, out of the gate's own between-reference comparisons:

| page | sets | a reference the taken set contradicts |
|---|---|---|
| `colorkeymask.pdf` p1 | `{poppler, mupdf}` / `{mupdf, ghostscript}` | `ghostscript`, at the same worst tile of 5.03 that contradicts us |
| `colors.pdf` p1 | `{poppler, ghostscript}` / `{mupdf, ghostscript}` | `mupdf` — and `poppler` is contradicted by the rival set |
| `colors.pdf` p2 | `{poppler, ghostscript}` / `{mupdf, ghostscript}` | `mupdf` — and `poppler` is contradicted by the rival set |
| `issue11403_reduced.pdf` p1 | `{poppler, mupdf}` / `{poppler, ghostscript}` | `ghostscript` |

So on every one of the four, the set that decided the verdict contradicts a **voting reference that
is itself a member of a maximal consensus** — and on both `colors.pdf` pages each of `poppler` and
`mupdf` is contradicted by the other's set. The general statement is the one worth keeping:

> **On a divided page no renderer in the room, ours or a reference's, is outside every reading the
> references have.**

`colorkeymask.pdf` is the case that needs no tolerance arithmetic at all. Our raster is
byte-identical to `ghostscript`'s over the whole 595 × 842 page, so the four numbers that contradict
us *are* `ghostscript`'s distance from `poppler`. The gate cannot call our render defective there
without calling one of its own three references defective, in the same figures, on the same page.

## What was rejected, and why

**Hold us to every maximal consensus** — contradicted if any set rejects us. It changes no verdict
today, which is its whole attraction, and the control above is what disqualifies it: applied
evenhandedly it condemns `ghostscript` on two of the four pages and both `poppler` and `mupdf` on
the other two. A rule that would call defective the very implementations whose agreement is the
evidence it runs on is self-undermining, and it keeps four verdicts that rest on picking one of two
incompatible readings.

**Take the tightest set.** Two objections, either sufficient. It is undefined — "tightest" needs an
order over four measures, and on `colors.pdf` the two candidate pairs each win a different subset of
them on each page. And it ranks candidate readings by exactly the quantity trap 9 says is
manufactured: shared code, a shared ICC file, a shared decoder and a shared published standard all
make a pair agree *more* closely, and on `colors.pdf` the tighter pair is `mupdf` and `ghostscript`,
which trap 9's second bullet records reading the same 187 484 bytes of `default_cmyk.icc`. That it
would move three pages our way is not a recommendation; it is the reason to look at it twice.

**Report both and move nothing**, which is what 727 did and was right to do for one round. The
reporting is now a round old, the argument above does not need another measurement, and leaving a
verdict standing on an enumeration order is not a neutral act — four pages carried an accusation
against this tree that nobody could defend on the merits.

## What moved

The whole corpus, before and after, from two runs of the same gate in one sitting at a 100% cache
hit rate:

| | agrees | contradicted | ambiguous | our geometry | reference geometry | not comparable | no render |
|---|---|---|---|---|---|---|---|
| before | 983 | **65** | **832** | 3 | 2 | 42 | 18 |
| after | 983 | **61** | **836** | 3 | 2 | 42 | 18 |

Four pages, one direction, and a line-by-line diff of all 962 non-agreeing per-page verdicts shows
those four lines and no others. `agrees` is unchanged, so nothing entered or left it either.

**A rule that only moves pages toward leniency should be suspected, and this one is not one-way — it
is one-way *today*.** The gate now prints the population that says so: of the 41 pages carrying more
than one maximal consensus, **36 carry sets that concur in agreeing with us**. Every one of those is
a page where a moved pixel that divided the sets would cost an agreement, and that is the direction
none of the four is in. The rule is live in both; the corpus is what is one-sided.

**And one of the four is not flattered by it**, which the group note says in its own entry.
`issue11403_reduced.pdf` page 1's division is of *width* rather than of camps: `poppler` is in both
sets, we sit 6.24%, 6.14% and 5.20% of channels from `poppler`, `mupdf` and `ghostscript` — further
from every reference than any two of them are from each other — and what takes the page out of
`contradicted` is that `{poppler, ghostscript}`'s own 4.815% spread doubles to a bound admitting us.
Nothing about this render improved. `ambiguous` is not an acquittal, the cap-height diagnosis stays
in `CONTRADICTED_SUBSTITUTED_FONT` where it was measured, and the page is named.

The other three divide by *camp*: on `colorkeymask.pdf` our render is `ghostscript`'s to the byte,
and on both `colors.pdf` pages it is `mupdf`'s to ssim 0.99989 and 0.99974 while `ghostscript`
straddles and `poppler` parts from both.

## Where the pages went

`AMBIGUOUS_DIVIDED_CONSENSUS` holds all four with the reading of each and is chained into
`diagnosed_ambiguous()`, so they are a *named* population rather than four more names in a bucket of
836. The three `CONTRADICTED_*` groups keep their notes: `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`
is empty now and its §10.7.4 reading — which concludes the specification answers *for* us — is
untouched, `CONTRADICTED_TIGHT_CONSENSUS` keeps `issue7891_bc1.pdf`, which has one pair and no rival
set at all, and `CONTRADICTED_SUBSTITUTED_FONT` keeps eleven pages and the cap-height table. A
verdict moved; not one measurement did.

## Consequences

- **`Outcome::Ambiguous` now has two shapes and they are worth telling apart**: nobody agrees, or
  two sets agree and divide. `Triangulation::divided` separates them and the page's own verdict line
  says which, because on the second every renderer is inside somebody's reading and on the first
  none is.
- **The condition is the sets disagreeing, never their number.** `issue19633.pdf` page 1 carries two
  maximal consensuses and both reject us, so it stays contradicted;
  `two_maximal_consensuses_that_concur_still_reach_a_verdict` holds that property against a fixture
  beside the calibration ADR 0616 wrote (trap 13).
- **Trap 12's second half gains its resolution**, and trap 9 is still not what this is about: nothing
  is wrong with any of these pairs agreeing.
- **`doc/todo/12`'s neighbouring question is answered and the item's own is not.** Which pair forms
  the consensus is decided; how wide the bound derived from it should be is still the 0.05 differing
  fraction doing two jobs.
- Owed: `issue11403_reduced.pdf` is the standing witness that a *width* division and a *camp*
  division are not the same thing, and this rule treats them alike. Whoever wants to separate them
  needs a reason a rival set's widening should count for less than the taken set's — and today there
  is none, because a page whose only consensus is that rival is judged by exactly that bound and
  agrees.
