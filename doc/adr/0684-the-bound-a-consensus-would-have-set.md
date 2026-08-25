# 0684 — The bound a consensus would have set, and where *we are alone* stops

**Status.** Accepted. Session 761.

`rank_the_pages_we_are_alone_on` requires our own number to be outside the tolerance class's
**floor** (ADR 0663). The floor is the weakest bound in the gate — it is what `pdfref::decide`
returns *because* no consensus formed, not a judgement anybody made about the page — so a page can
be on that list while the references are further outside it than we are, and most of the list is.
The bound the gate actually applies wherever a consensus does form is `Judgement::CORPUS`'s: twice
the consensus's own spread, floored. **Asking that question of the closest pair splits the list into
a head and a tail**, and the head is where a round diagnosing it stops.

## Context

751 filtered this list at 1.0 and the argument was a good one: below the floor, the nearest
reference would have *accepted* our page had it been in a consensus, so the page is not one we are
alone on. What 751 could not say is where the filter's own reasoning runs out, and it runs out
immediately above 1.0. Every page on this list is outside a constant; the constant is the same for
every text page in the pool; and the references on the same page are measured against that same
constant and are frequently further outside it than we are. `freeculture.pdf` pages 315 to 333 are
the standing instance — five pages where a ratio above 3 sits over ladders that agree to 0.032 of
255 (ADR 0647).

The gate has a better bound and applies it everywhere else. `Tolerance::widened_to` takes a
consensus's own comparison and widens the class bounds to `factor` times it, per measure, floored;
`Judgement::CORPUS` is that judgement at `factor: 2.0`, and its doc comment states the reason —
"a third correct implementation is not required to sit between them: it may differ from both in the
same direction, at the same magnitude". That sentence is about a page with a consensus, and it is
just as true of a page without one.

## Decision

`Examined::outside_what_the_closest_pair_would_allow` asks, per page: **take the closest pair of
references, widen the class bounds to `Judgement::CORPUS`'s factor times that pair's own comparison,
and measure our nearest comparison against the result.** Outside it, the page is marked
`[widened: outside]` on the printed list and counted underneath; inside it, the row is unmarked.

The rule that comes with it, and it is the whole point of the mark:

- **Marked** — no reading of these references forgives our render. Whatever the page is, the answer
  is a clause or a mechanism in the divisor, and it is worth a round.
- **Unmarked** — a consensus at that pair's spread would have accepted us. The page is alone against
  a constant, and the ratio is measuring how closely two references happen to agree.

**A round working this list works the marked rows and stops at the first unmarked one.** The list
prints ten rows or the whole marked head, whichever is longer: a count that names a head of thirteen
under a list of ten is a queue a reader cannot open, which is ADR 0643's defect in a different
place.

## Why the factor is read rather than written down

`corpus_widening_factor` matches on `Judgement::CORPUS` instead of naming 2.0 a second time. The
number is the whole of this decision's argument, and a number stated in two places is a number that
will eventually disagree with itself — `doc/todo/02` §2's own history is a list of such numbers.
`Judgement` is `#[non_exhaustive]`, so the match has a wildcard arm, and it reads a variant that is
not a widening as no widening rather than assuming one.

## Why this is not a filter

751's cut removes pages, because below 1.0 the numerator says the opposite of the list's name. This
one does not, and the difference is real: above the floor and below the widened bound, *we are
alone against the floor* is still a true sentence about the page — it is a weaker sentence than the
list's name suggests, which is what the mark says. Filtering there would hide pages whose divisor is
the finding, and a divisor being the finding is what half of trap 9's tenth bullet is about.

## What it costs and what it is not

**It is a sufficient condition read against an exact one.** Both halves of the printed ratio are a
maximum over three normalised measures, so a ratio at or above 2.0 implies the mark: our worst
measure then exceeds twice the pair's worst, which exceeds twice the pair's on that same measure.
The converse fails whenever our worst measure and the pair's are *different* measures, and this run
has two such pages — `freeculture.pdf` page 1 at 1.83× and `copy_paste_ligatures.pdf` at 1.65×, on
both of which our number is the structural similarity and the pair's is the mean. That is why the
gate computes it per page rather than leaving a reader to divide.

**And it is a counterfactual, stated as one.** No consensus formed on any of these pages; the
question is what the gate *would* have held us to had one formed at that spread. The honest reading
of a mark is therefore about the gate's own calibration and not about a verdict — nothing here
changes a verdict, a ratchet, or a page's bucket.

## Measured

The list is 26 pages and the marked head is 13: the eleven whose ratio reaches 2.0, plus
`freeculture.pdf` page 1 and `copy_paste_ligatures.pdf` page 1. Every one of the thirteen carries a
priced reading — the eleven from sessions 518, 744, 751 and 756, each naming the mechanism in its
numerator or in its divisor and measuring it, and the last two from this one, which name the
**measure** as well because on both of them our worst measure is not the pair's. The census, every
non-agreeing per-page line and every other ranking are unchanged: this round moves no pixel and the
whole diff under `crates/` is `tests/oracle.rs`.

**What is owed and was not taken**: nine of the eleven older readings price a mechanism without
naming which of the three measures their number is. `outside_by_in_three_measures` returns a
maximum and throws the name away, where `worst_ratio` keeps it — so the gate could print it, and
`--bin unpriced`'s rule for the contradicted pool ("a round that writes a group note names the bound
its pages fail on") could then be asked of this list too.
