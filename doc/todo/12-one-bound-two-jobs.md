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

**38 of the oracle's 68 contradicted pages fail this bound and no other**, 37 of them text.

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

## What this is not

Not a licence to loosen. If the answer turns out to be that 0.05 is the right consensus threshold
and the floor is a separate number, the floor still has to be derived from renderers that are not
us — and if no such measurement is available, the bound stays where it is and the 38 pages stay
listed with the reason beside them, which is where the four-hundred-and-seventh session left it.
