# One bound doing two jobs: the differing fraction on text pages

Status: **the question this item was opened for is answered** — the two jobs stay one number, and
the reason is measured rather than cautious. What is left is named at the bottom and is smaller
than the item was.
Opened in the four-hundred-and-seventh session, answered in the eight-hundred-and-forty-fourth.
Priority: 12 — demand-driven, and it is about the instrument rather than about a page
Code: `tools/pdfref/src/lib.rs` (`Tolerance`, `Judgement`, `widened_to`),
`crates/pdf-model/tests/oracle.rs` (`the_fixed_bounds_against_the_references_own_spread`,
`substitutions_of`, `the_excluded_reference_the_consensus_also_convicts`)
Derivation and numbers: **ADRs 0243, 0717 and 0771**. Read 0771 first; it supersedes the reason
the other two give for leaving the bound alone without changing what they measured.

## What was asked

`Tolerance::TEXT_HEAVY::max_differing_fraction` is 0.05 and does two jobs: `Tolerance::accepts`
decides whether two references form a **consensus**, and the same number **floors** the per-page
bound `widened_to` derives. ADR 0243 measured that it sits below its own references' spread and
left it, because separating the two needed a floor derived from a pair including a non-hinting
renderer with neither member ours, and the only candidate shares `skrifa` with this tree.

## What was found

**The population that requirement asked for existed all along, behind an `ldd`.** `poppler` and
`mupdf` load one `libfreetype.so.6`; `ghostscript` names none and carries a statically linked
copy. So `ghostscript` against either is two separate FreeType copies, neither member ours — a
weak independence, being one algorithm twice, and enough to derive a floor from. Split that way,
on text pages, the differing fraction runs median 0.86% within the sharing pair and 2.50% across
the boundary, while the class's other three measures do not move across it at all.

**The floor was derived at ADR 0243's own rule — the 99th percentile — implemented, and priced.**
Floored at 12.04% for our judgement alone with consensus formation untouched, the corpus gate
reports 1017 agrees / 24 contradicted / 835 ambiguous against 980 / 60 / 836: 36 pages leave
`contradicted` and none arrives.

**It was not taken, and six named pages are why.** Five are `CONTRADICTED_CALRGB_TO_SCREEN` and
one is `CONTRADICTED_SUBPIXEL_IMAGE` — a §8.6.5.3 colour reading and a §10.7.4 departure, each
measured in its own note. A differing fraction is a threshold count, so 5–12% of it is reached
either by a sub-pixel phase on every glyph edge or by a small colour error over a large area, and
**a bound cannot separate what a mechanism separates**. That is the answer: not that no number
could be derived, but that the measure the number bounds conflates two mechanisms.

**And the *different verdict* branch is closed by a base rate.** The candidate rule was trap 12's
control — where the consensus would contradict the voting reference it excludes, the bound is not
one an independent implementation meets. The gate counts it every run now, and it holds on **52 of
the 60** contradicted pages, across the JBIG2 pages, the colour pages and the link border alike.
ADR 0717's *32 of 32* is the pool's base rate rather than that population's signature; a rule
resting on it would acquit us wherever two references agree for any reason at all.

**What replaced `widened_to`'s standing request.** It asked for "a measurement of how far a
*fourth* independent rasteriser sits from the three", and `pdfium` is still not packaged. The
question a verdict asks needs no fourth renderer: `decide` takes the *closest* pair in the room,
so the bound is a selected minimum and what a third implementation owes is to sit as near that
pair as the excluded one of three manages. `substitutions_of` measures exactly that, by running
the gate's own judgement with a reference standing where our render stands — and the answer is
that the same 5% floor convicts a known-good reference on 0.6% of text pages under the one
consensus whose members do not share the FreeType object and on 9.1% under the one whose members
do. **The bound is not what varies; the consensus is.**

Re-run either derivation at any time — one command, both tables:

```sh
PDFVIEWER_ORACLE_SPREAD=1 cargo test --profile gates -p pdf-model --test oracle -- \
    --ignored --nocapture the_fixed_bounds_against_the_references_own_spread
```

## What is left

Three things, and none of them is the number this item was named for.

1. **The consensus half was never moved and its 278 pages are still a programme.** Raising
   `max_differing_fraction` for consensus formation makes 457 `ambiguous` pages judgeable and 278
   of them contradicted — ADR 0243 measured it. Several hundred are `doc/todo/00`'s dense-text
   population, which is the only mechanism anybody has named for why that bucket is the size it
   is. Nothing here argues for or against it; what ADR 0771 removes is the *floor* as a reason to
   go near it.
2. **The three pages the control does not excuse.** `bug847420.pdf`, `issue19633.pdf` and
   `issue7891_bc1.pdf` page 1 are the whole of the pool on which a voting reference outside the
   consensus meets the bound while we do not. That is the sharpest population the oracle produces
   and it has never been read as one.
3. **The vector row of the substitution table, which nobody went looking for.** The `mupdf` +
   `ghostscript` consensus contradicts `poppler` on 119 of 226 vector pages against 13 of us.
   Every one of our contradictions under that consensus sits beneath an error rate of 52.7% on a
   program nobody suspects, and trap 9's shared-data bullet is the hypothesis rather than the
   finding.

## Two neighbouring questions, both asked and answered elsewhere

- **Is a consensus of two the same evidence as a consensus of three?** ADR 0575: same kind, one
  factor less, and nothing moved. The six pages it printed are about why the third reference could
  not read the document, which is `doc/oracle-and-corpus.md` §3g.
- **Which pair forms the consensus, where two maximal sets exist?** ADRs 0616 and 0617: a verdict
  is one every maximal consensus reaches, and a page whose sets divide about us is `ambiguous`.
  Two residues, both recorded in trap 12: a *width* division and a *camp* division are treated
  alike, and 36 pages carry sets that concur in agreeing with us.
