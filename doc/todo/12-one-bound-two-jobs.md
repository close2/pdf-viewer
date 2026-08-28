# One bound doing two jobs: the differing fraction on text pages

Status: **open**, from the four-hundred-and-seventh session.
Priority: 12 — demand-driven, and it is about the instrument rather than about a page
Code: `tools/pdfref/src/lib.rs` (`Tolerance`, `Judgement`, `widened_to`),
`crates/pdf-model/tests/oracle.rs` (`the_fixed_bounds_against_the_references_own_spread`)
Derivation and numbers: **ADR 0243**

## What is known

`Tolerance::TEXT_HEAVY::max_differing_fraction` is 0.05, and on the class it applies to it is the
one fixed bound of eight that sits **below** the spread of the implementations that set it. Over
2638 pairs of `poppler`, `mupdf` and `ghostscript` on text pages — each measure taken over the
pairs the other three bounds admit — it rejects **29.4%** of them, where its three siblings reject
0.0%, 1.2% and 0.5%. The same measure on vector pages rejects 2.8% and is unremarkable, so this is
one number in one class rather than a problem with counting channels.

**Most of the oracle's contradicted pages fail this bound and no other**, and almost all of those are
text. The share is a count and therefore not written here: it said *38 of 68* for long enough to be
wrong twice, once when the pool shrank under it and once when this round moved four pages out. The
gate's own per-page lines print `ours at worst … ; bound …` for each of the four measures, which is
where the current figure is.

**And since the seven-hundred-and-thirty-seventh session the gate prints that population in the one
unit this item never had** (ADR 0636). `rank_the_contradicted_by_the_bound` orders the pool by how
far outside its bound each page sits and says which of the four measures the ratio belongs to, so
the line under it counts the pages this bound is the binding one on **and gives the range they span**
— which is the difference between *how many pages this bound convicts* and *how much of a page's
verdict it is worth*. Read it before arguing the number again: a population whose members sit a few
percent outside is a different argument from one whose members sit twice outside, and until that line
existed nobody could tell which this was.

**And since the seven-hundred-and-forty-first the same number has a second population behind it, on
the other pool** (ADR 0643). ADR 0243 measured this bound on 2638 pairs of *references*; that round
measured it on 804 pages of *our own* comparisons, in the ambiguous bucket, where the bound is the
unwidened class floor because no consensus judged the page. Our differing fraction there sits at a
median **2.08** times the floor against the closest reference pair's **1.96**, and on 222 of the 804
pages ours is the smaller of the two. Two instruments, two populations, the same answer: this is the
one bound of the eight on which we and the references fail together.

**And since the seven-hundred-and-eightieth the population has a per-verdict control, and it is
trap 9's tenth mechanism sitting in the bound itself** (ADR 0717). Of the pages this bound is the
binding one on, nearly all are convicted by `poppler` and `mupdf` alone — the one voting pair that
hints its glyphs through a single `libfreetype.so.6`, where `ghostscript` carries its own
statically linked copy — and the gate's ranking prints that count every run. Measured over all 32
such pages with `examples/compare_rasters` on the gate's artefacts: the convicting pair's differing
fraction runs 0.00% to 4.37% (median 2.33%), every pair containing `ghostscript` runs 5.32% to
13.37% (median 6.8%) — the distributions do not overlap — and `ghostscript` fails this bound
against both members of the convicting pair on **32 of 32** pages, further than we do on 27. So on
this population the consensus half of the bound's two jobs is being done by shared hinting, and the
floor half then convicts whoever does not share it — us on 27 pages by less than the third voting
reference misses by. It is ADR 0243's 29.4% arriving per verdict, and it moves nothing by itself:
requirement 2 below still stands, because the measurement names no derived floor.

**What that settled is a question about an ordering rather than about the bound.** `Distance::of`
keeps three measures and not this one, and the seven-hundred-and-thirty-seventh round recorded the
consequence for the ranking built on it as unpriced. Priced: on the contradicted pool the head is the
same ten pages in the same order under either unit, and on the ambiguous pool a four-measure reading
would call 569 of the 804 pages ones we are alone on where three measures call 48. So the fourth
measure stays out of `Distance` and is printed beside it — and the reason is this item's number, not
a convenience about quoted figures.

Re-run the derivation at any time:

```sh
cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture \
    the_fixed_bounds_against_the_references_own_spread
```

## Why it was not simply raised

The bound does two jobs. `Tolerance::accepts` decides whether two references form a **consensus**,
and the same numbers **floor** the per-page bound `widened_to` derives. Raising it to the 99th
percentile of the reference spread, 0.12, was run over the corpus: 905/68/786 becomes
**1121/309/329**. 457 pages leave `ambiguous` and **278 arrive newly contradicted**, against 37
leaving. The number cannot be adopted without arguing 278 pages.

## What the work is

**Separate the consensus threshold from the judgement floor**, or establish that they must be the
same number. Three things have to be true before either is done:

1. **A rule, stated before the pages are looked at.** ADR 0243 uses the 99th percentile of the
   reference-against-reference distribution because that is where the other three TEXT_HEAVY
   bounds already sit; any other rule needs the same kind of justification. A number chosen so
   that 37 pages pass is the curve-fitting `CLAUDE.md` forbids outright.
2. **A floor derived from a pair that includes a non-hinting renderer, and neither member ours.**
   Across the hinting boundary the median differing fraction doubles — 1.69% to 3.42% — but the
   only renderer on the far side of it is `hayro`, which shares `skrifa` with this tree.
   `widened_to`'s standing sentence is still unanswered: what would justify a change is a
   measurement of how far a **fourth independent** rasteriser sits from the three. `pdfium` is the
   candidate `Reference` already names and Arch does not package.
3. **The 278 pages.** Whatever is decided about the consensus half, those pages are real work: they
   are pages where two references would agree under a derived bound and this tree would then be
   contradicted. Several hundred of them are `doc/todo/00`'s dense-text population, which is the
   first mechanism anybody has named for why that bucket is the size it is.

## A neighbouring question, asked and answered — it is not this one

**Is a *consensus of two* the same evidence as a consensus of three?** ADR 0575 asks it on the six
pages ADR 0542 printed, and the answer does not touch this item: two references stay enough,
because ADR 0005's inference is about a **pair** and a third multiplies the improbability rather
than creating it. It is worth knowing here for one reason — the *bound* on such a page is derived
from one pair's spread rather than the maximum of three, so it is trap 12's shape rather than this
file's, and a page judged on two is held tighter rather than more leniently. The six turned out to
be about why the third reference could not read the document, which is `doc/oracle-and-corpus.md`
§3g.

## A second neighbouring question, asked and **answered** — *which* pair forms the consensus

**From the seven-hundred-and-twenty-seventh session** (ADR 0616), and it is this item's code rather
than its number: `Tolerance::accepts` decides whether two references form a consensus, and
`pdfref::decide` then picks one. **Agreement is not transitive**, so `a ~ b` and `b ~ c` with
`a ≁ c` leaves two maximal agreeing sets, neither contained in the other. The loop skipped a subset
no larger than the best so far, so the second was discarded without being counted, and the survivor
is the one whose subset bitmask is smaller — the order `Reference`'s variants are declared in.

`Triangulation::consensuses` now holds them all and the gate counts the pages carrying more than
one, naming those where the sets reach **different verdicts about us**. `AMBIGUOUS_DIVIDED_CONSENSUS`
in `oracle.rs` is that list with the reading of each page. Every member of it was contradicted and
every one would have agreed under the set that was thrown away, including a page whose discarded
pair contains a renderer our raster is byte-identical to.

**The seven-hundred-and-twenty-ninth session took the rule** (ADR 0617): **a verdict about our
render is one every maximal consensus reaches**, and where they reach different ones the page is
`ambiguous`. That is ADR 0005's second rule at the granularity its first is stated in, and what
disqualified the two alternatives is a control rather than a preference — put each *reference* where
our render stands, and on all four pages the set that used to decide the verdict contradicts a voting
reference **that is itself in a consensus**. Holding us to every set would therefore condemn the
implementations whose agreement is the evidence; taking the tightest set is undefined over four
measures that do not rank together and ranks readings by the quantity trap 9 says shared code
manufactures. Four pages moved, contradicted → ambiguous, and nothing else in 1945 did.

Two things it left, and neither is this item's number:

- **A *width* division and a *camp* division are treated alike**, and `issue11403_reduced.pdf` page 1
  is the witness: one reference is in both sets, we are further from all three than any two are from
  each other, and what takes the page out of `contradicted` is the rival pair's own spread doubling.
  Separating the two needs a reason a rival set's widening should count for less than the taken
  set's — and there is none today, because a page whose *only* consensus is that rival is judged by
  exactly that bound and agrees. Which is `widened_to` again, one line below.
- **36 pages carry sets that concur in agreeing with us**, printed by the gate. Each is an agreement
  a moved pixel could cost by dividing them, which is the direction none of the four was in.

It is *not* the same question as the one above: this one is about which pair the bound is derived
from, not about how wide the bound then is. They meet in `widened_to`, which is why both are here.

## What this is not

Not a licence to loosen. If the answer turns out to be that 0.05 is the right consensus threshold
and the floor is a separate number, the floor still has to be derived from renderers that are not
us — and if no such measurement is available, the bound stays where it is and the 38 pages stay
listed with the reason beside them, which is where the four-hundred-and-seventh session left it.
