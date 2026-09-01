# ADR 0774 — Identity is the easy page's base rate, not the pair's signature

Status: accepted, 2026-09-01. Session 847, an oracle round on `doc/todo/12`'s consensus half.

ADR 0773 found that on 97 of the 117 comparable pages of one row of one table, `mupdf` and
`ghostscript` produce **byte-identical** rasters, and handed on the question it could not answer:
*should a consensus whose two rasters are identical be a consensus at all?* The gate now counts
that over the whole population, every run, and the answer is **no rule** — the consensus stands as
it is, ADR 0243 extended, and the reason is measured rather than cautious.

**Identity is a property of the page, not of the pair.** It runs at **0.4% on text pages and
68.9% on vector ones**, it is **depleted** rather than enriched in the pool where a manufactured
consensus would do damage, and **39% of it is a three-way identity including the reference ADR
0773's row excludes**. A rule demoting it would dissolve the strongest agreement this instrument
can record.

## 1. The census

`oracle.rs`'s `what_the_consensus_was_made_of`, printed on every run of the corpus gate. It adds
no render and no comparison: `Triangulation::between_references` already holds every pair's
`raster_compare::Comparison`, and `max_error` is the field that separates identity from closeness
— trap 9's own tell, *because every other number on that line rounds to zero long before the
rasters are equal*. `ConsensusIdentity` reads the head of `consensuses`, which is the set
`verdict_of` names on the page's own line and the set `ExcludedReading` is already taken against,
so the three lines are about one set rather than three.

Two populations, because `doc/todo/12` asked for both — identical, and *near-identical enough to
leave the bound at its floor*:

| | pages | of 1044 judged by a consensus |
|---|---|---|
| every pair in the deciding set is `max 0` | **176** | 16.9% |
| the bound never left the class floor | **629** | 60.2% |

The second is the wider fact and nobody had it: on **three pages in five** that a consensus
decides, `Tolerance::widened_to` widened nothing at all and the verdict rests on the bare class
floor. Identity is a subset of it — a spread of zero forces a floor, and so does any spread small
enough that twice it is under every class bound.

## 2. What each pool looks like with those pages set aside

| pool | judged by a consensus | by identical rasters | left, judged by two readings |
|---|---|---|---|
| agrees | 980 | **172 (17.6%)** | 808 |
| contradicted | 60 | **4 (6.7%)** | 56 |
| ambiguous (divided) | 4 | 0 | 4 |

| class | judged by a consensus | by identical rasters |
|---|---|---|
| text | 793 | **3 (0.4%)** |
| vector | 251 | **173 (68.9%)** |

| the set that is one raster | pages |
|---|---|
| `mupdf` and `ghostscript` | 95 |
| **`poppler` and `mupdf` and `ghostscript`** | **68** |
| `poppler` and `mupdf` | 10 |
| `poppler` and `ghostscript` | 3 |

## 3. Why no rule follows, in three readings the census makes and ADR 0773 could not

**It varies with the page and not with the pair.** Two Artifex programs share their code on every
page they draw, so a mechanism of *dependence* would show at about the same rate whatever is on
the sheet. Between the two tolerance classes the rate moves by **more than a hundredfold** — 3
pages of 793 against 173 of 251. What produces
identity is a page with one answer at 72 dpi — axis-aligned, integer-aligned, bilevel or flat —
which is exactly what `Tolerance::widened_to`'s own doc comment has said since it was written:

> a spread of zero — two references producing identical pixels, which happens on simple pages —
> would otherwise demand exactness of us that no third implementation can deliver.

The floor exists **for** this population. The census is the first measurement of how large it is.

**It is depleted where a manufactured consensus would cost something.** If identity were the
signature of a conviction resting on one reading, the contradicted pool would carry more of it
than the agreeing pool. It carries **2.6× less** — 6.7% against 17.6%. Read the other way: 172 of
the 176 identical-raster verdicts are *agreements*, so a rule that refused them would take 172
pages out of `agrees` and into `ambiguous` and move four convictions.

**And 68 of the 176 are three-way.** On those pages every voting reference in the room produced
the same bytes, `poppler` included — the reference ADR 0773's row is about excluding. Two Artifex
programs agreeing is a dependence one can argue for; three programs from three source trees
agreeing to the byte is not an argument about source trees at all. Under a rule that "identical
rasters do not form a consensus", the *strongest* evidence the oracle can record would become no
evidence, which inverts ADR 0005.

## 4. And the four convictions are the group already named for the mechanism

Named by the gate rather than by this document (ADR 0772's rule):

- `bitmap-halftone-composite.pdf` page 1 — `mupdf` and `ghostscript`
- `bitmap-refine-page-subrect.pdf` page 1 — `mupdf` and `ghostscript`
- `bitmap-symbol-symhuffrefineone.pdf` page 1 — `mupdf` and `ghostscript`
- `xobject-image.pdf` page 1 — `poppler` and `ghostscript`

The first three are **`CONTRADICTED_SHARED_JBIG2_DECODER` exactly** — the group named for
`jbig2dec` twice, whose note has carried trap 9's fifth mechanism since ADR 0499 and whose *right*
answer is ADR 0381's, taken out of the documents themselves rather than out of anybody's raster.
The fourth is `CONTRADICTED_ON_A_PAGE_WE_REPORT`, a page this tree reports on and the gate does not
gate. So the whole population where a conviction rests on one raster is a population this file was
already holding by name for the same mechanism, and **the census moved no page because there was
no page to move**.

## 5. The general shape, which is ADR 0771's arriving one round later

ADR 0771's sentence was *a control measured on the population it was invented for is a hypothesis
until it is run on the population it excludes* — and it retired a rule resting on *32 of 32* when
the whole pool turned out to be 52 of 60. ADR 0773 then measured a new mechanism on a new
population of 117 and handed it on as a question. The same sentence answers it: 97 of 117 is what
**vector pages** look like, and the row was a row of vector pages.

That is worth more than the instance. A mechanism found by measuring one row is a claim about that
row until the denominator is widened, and this project has now paid for that twice in four rounds.
The remedy in both cases was the same and is cheap: **make the gate count it, on the whole
population, every run** — 176 and 629 and the class split are three lines of arithmetic over
numbers the gate already had.

## 6. What this changes

- `oracle.rs` gains `ConsensusIdentity`, `the_consensus_that_decided_it` and
  `what_the_consensus_was_made_of`; the last names the four contradicted pages rather than
  counting them.
- `CONTRADICTED_SHARED_JBIG2_DECODER`'s note records that its three pages are three quarters of
  that population.
- Trap 9's identical-rasters bullet gains the base rate, which is what stops it being read as a
  mechanism that accuses a verdict.
- `doc/todo/12` item 3's remaining question is answered and closed.
- No bound, verdict, page or pixel moves. The gate reports **980 agrees / 60 contradicted / 836
  ambiguous** over 1945 pages before and after, because nothing but a `println!` was added to the
  judging path.

## 7. What this does not claim

That `mupdf` and `ghostscript` are two independent readings on the 95 pages where they are one
raster — ADR 0773's mechanism is real and stands **as a description of those pages**. What is
denied is only its generalisation: byte identity is not a test for it, because identity is what a
page with one answer produces in any renderer that gets the answer right. Nor that the 629 pages
at the class floor are a defect; that number is printed so that a round reading `doc/todo/12`
knows the *relative* bound is inert on most of the pool before it argues about the floor's value.
