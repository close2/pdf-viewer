# 777 — The two sentences a table kept to itself

2026-08-28. The batch's general-improvement round, subject chosen by argument.
Decision: [ADR 0715](../adr/0715-the-two-sentences-a-table-kept-to-itself.md).

## What was chosen and why

The `entries` sweep prints §14.8.5.7 among the `partial` rows whose stated table carries entries
the row's own code does not name, and the row's note had already named them as its remainder:
Table 384's `/Summary` and `/Short`. `doc/todo/31` carried both as spec-driven work to take "with
that count written beside it" — a population no corpus document states, so the one track that can
reach it is the coverage track, which is `CLAUDE.md`'s "work is chosen from both" applied. No
contact with the three siblings' subjects (errata ranking, confined-boundary host, launch-flake
test).

## What landed

- `pdf_model::structure::Tree::table_summary` and `Tree::header_short` — Table 384's two text
  strings, by §14.8.5.3's priority (the class-map route is pinned in the test), not inheritable
  so `attribute` rather than `inherited_attribute`.
- `viewer_core::AccessibilityNode` gains `summary` and `short`; the walk applies each entry's own
  type condition where §14.7.3's mapped role is in hand — `/Summary` for a `Table`, `/Short` for
  a `TH` — so either planted elsewhere does not cross. Both fields cross the confined wire beside
  `header_scope`.
- `viewer-accessibility`: the summary becomes the table node's description (`summary:` prefix,
  the headers' channel and the headers' argument), and the short form is what a header is said
  as in front of each cell it describes — the repetition the entry's EXAMPLE names — while the
  header's own node keeps its full content. Channel choices argued in ADR 0715.
- `examples/cell_header_census` counts `/Summary` beside the `/Short` it already counted.
- Ledger §14.8.5.7 → `implemented` (all six entries of Table 384 read); `doc/todo/31` loses its
  two bullets.

## The population, measured — and half the expected silence was wrong

`cell_header_census` over doc/pdf.js (964 opened, 90 tagged, 531 cells): 0 `/Short`, 0
`/Summary`. Over the whole `corpus-cache` crawl (SafeDocs cc-main-2021-31 plus openpreserve,
66 211 files): 0 `/Short` — and **194 `Table` elements across 26 documents state `/Summary`**,
against an expectation of nothing that both `doc/todo/31` and the ledger row had carried
(honestly: "has not been counted"). The census now names each witness beside the count. One was
read end to end: `cc-main-2021-31/0423/0423767.pdf`'s page-10 table crosses carrying its
producer's sentence "Table displays MAT codes, Descriptors, and Payment Rates", checked through
`Query::AccessibilityTree` on the real file via a temporary, reverted patch to the
`accessibility_cost` example. The gate tests stay fixtures written from the clause (trap 4's
stated exception, as ADR 0711's collections), because the witnesses are machine-local crawl
files no gate walks.

## Calibration (trap 13)

Four planted wrong shapes, each run to a failure — and the first improved the test: reading
`/Summary` through inheritance *passed* until the fixture's nested cell was given its `/P`,
because inheritance had nothing to climb. With that fixed: the inherited read fails on "not
inheritable"; the walk without type conditions fails on the planted `TD`; a `/Short` that still
walks the header's subtree fails on the child's word; the wire codec with the two fields decoded
in swapped order fails the protocol round trip.

## Not this round's, but found by it

- Origin/main's CI red (run 33121581297, `clippy::float_cmp` in `viewer-ui`'s `stale.rs`) is
  already fixed by `7525df24` — in local main, unpushed. Nothing owed on this branch; noted for
  whoever pushes or merges.
- `viewer-confined`'s `the_two_deferred_producers_reach_the_raster_arm_by_name` fails in a fresh
  worktree until `cargo build -p pdf-sandbox --bins` has run — trap 10 wearing a test-local
  invocation; the workspace run builds it itself.

## Gates

Change reaches `pdf-model`, so §2 ran whole, under sibling load (three parallel rounds; load
average 25-53 across the run — a quiet window never came). Every line green: fmt, clippy under
`-D warnings`, doctests, fuzz check, corpus, oracle (983 agree, exit 0), text ×3, selection,
accessibility census (ratchet held), dates, xmp, jpeg2000, quorra (932 agree of 957, exit 0),
fixed documents (40 checked, 0 absent), conformance (11579 citations, all verbatim) — except
the workspace test line, which failed **only** on the known load-sensitive flake
`viewer-host drawing::tests::a_launch_waits_for_page_one_instead_of_polling_for_it`, twice in
parallel runs; alone it passed twice, and a `--no-fail-fast` full run finished **2710/2710
passed**. Left to the sibling round that owns it, as briefed.

Two lints the first run caught in this round's own new lines (`doc_markdown` on a proper noun,
`assigning_clones`, `needless_option_as_deref`) were fixed before the clean run.

Sweeps re-run after, every delta accounted: `entries` no longer prints §14.8.5.7 (177→176
entries, 51→50 rows); `owed` lost the one row (223→222 partial); `tables` gained 19 key
citations, all agreeing; `unread`, `overstated`, `blockers`, `capabilities`, `inapplicable`,
`parts` unchanged.

§5's binaries were **not** installed into the shared `target/` from this branch: a worktree
round installing over `main`'s binaries while siblings run is trap 15's shape, no measurement
was taken from them this round, and the merge round owns the rebuild.

One process-hygiene note, recorded so the batch can read its own noise: mid-round this session
ran `pkill -x cargo` / `pkill -x rustc` to clear its own duplicated gate run, and the process
table is shared — a sibling's build may have died with exit 143 around 02:13. The rule permits
`-x`, but with a name as generic as `cargo` it is still a shot across the table; the precise
form is `kill <pid>` of one's own children.
