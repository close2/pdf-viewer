# ADR 0102 — The file-only evidence population reaches zero

Status: accepted, 2026-08-01.

## Context

Four sessions ago 58 `implemented` rows named a whole test *file* as their evidence. The
argument for counting them is one sentence:

> A row naming `file.rs::a_test` is a claim something would fail if it stopped being true. A row
> naming `file.rs` is a claim nothing checks.

ADRs 0098, 0100 and 0101 took it to 23 by auditing §7.10, §7.5 and clause 8 — and **every one of
the three found something**: a row that had miscounted the standard and a `/Order` read nowhere,
§7.5.6's "most recent copy" rule not reaching a deletion, and the `DeviceCMYK` silence still
recorded in four places after `CLAUDE.md` disproved it. That record is the reason to finish
rather than to stop at a small number.

## Decision

Audit the last twenty-three — clause 7's thirteen, clause 11's four, clause 9's three, clause
14's two and §10.7.2 — and take the count to **zero**, at which point the assertion becomes `==`
rather than `<=`. A new `implemented` row arriving with a file for evidence now fails the build
rather than raising a number.

## What was owed, and what only had to be cited

Most of the twenty-three were already covered by a named test somebody had written for another
reason: `robustness.rs::a_lying_stream_length_is_recovered_from` is §7.3.8.2's whole subject,
`tounicode.rs::a_huge_range_is_stored_as_a_range_rather_than_expanded` is §9.10.3's,
`encryption.rs::an_identity_stream_filter_leaves_streams_alone` is §7.4.10's. Citing them costs
nothing and is worth doing: a row that names the test which would fail is a row a future session
can *check*.

Four rows had nothing, and the four are the session's work:

- **§7.4.1, the filter pipeline.** Every codec had a test beside itself; the *cascade* had none,
  and neither did `/DecodeParms` in its array form. `filters.rs` builds an `ASCII85`-around-
  `Flate` stream and a `/DecodeParms [null <</Predictor 12 …>>]`, so a reader that runs one
  stage, or gives the array's first entry to the second filter, fails.
- **§7.4.4.4, the predictors.** The clause has two groups whose *shapes* differ: the PNG group
  carries a type byte per row and TIFF Predictor 2 carries none. Nothing in the tree pinned
  either, on the feature that trap 4 exists because of. The new test encodes one row four ways —
  None, Sub, Up and Average — because a reader that applies the declared `/Predictor` to every
  row gets the first right and the rest wrong, which on a cross-reference stream is a table of
  fabricated offsets.
- **§7.7.3's page tree.** `page_geometry.rs` had four tests about a page's *entries* and none
  about the walk. The clause requires a processor to "be prepared to handle any form of tree
  structure built of such nodes", so the fixture is deliberately none of the shapes its NOTE
  describes: unbalanced, unequal depths, an intermediate node with no `/Type`, and a `/Count`
  that lies. Each leaf carries its own width, so the assertion reads the page *order* back as a
  sequence — which is what makes a page number mean anything.
- **§10.7.2's flatness.** The row's content is a permission and the permission is taken, which
  is a decision like any other and had nothing holding it. A page with a filled curve and a
  stroked one is rendered with and without `i` and an `/ExtGState` `/FL`, and compared over the
  whole raster rather than at a point — because a coarser flattening moves a curve's *edge* and
  leaves its interior exactly where it was.

Nothing else changed. Tests 853 → 860, and every gate is where it was: 840 agreeing, 65
contradicted, 90 incomplete, 97.9% of `pdftotext`'s words.

## Consequences

**The ledger now has two ratchets at zero** — `unreviewed` since the fifty-sixth session and
file-only evidence since this one — and they answer different questions. The first says every
subclause has been read. The second says every row that claims execution names something that
would fail.

Neither says the *right* test was named. The gate cannot tell whether a named test covers the
clause, and three of the four false claims this population has hidden were caught by the oracle
rather than by a row. What the zero buys is that the next false claim has to be written into a
test's name, where a reader will see it.

The population to watch next is the one with no gate at all: **235 `partial` rows**, whose notes
say what is owed and which nothing checks. `doc/HANDOVER.md` has said for two sessions that this
is where the next map has to be drawn from.
