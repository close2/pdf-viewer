# 1166 — The groups around the mode, and the question a half cannot answer

Batch twenty-five, branch `batch-1165-1170`. Rendering round. Contract: §11.7.4.3 and §11.7.4.4,
`partial` for the two constructions ADR 1158 named as reported.

## What the clause said

Both are built (ADR 1170). §11.7.4.3's group leaves §11.6.4.4's constants and the soft mask on
the object and moves only the blend mode, which makes the clause's own NOTE 3 exact. §11.7.4.4's
lifts the two constants off the parts and carries them itself, before the parts are painted
because a paint carries the constant inside its colour.

Two readings closed cases rather than opening them. §11.7.4.4 states the construction for a
combined fill and stroke exhaustively — two bullets and an "[i]n all other cases" — so
§11.7.4.3's paragraph is not owed a third group around such a pair. And §11.4.6's NOTE 6 is about
this nesting: inside a knockout group whose initial backdrop is transparent, the clause's group
*is* §11.4.5's isolated one, exactly rather than approximately.

## The defect the fixtures found

ADR 1157 asked "is the special mode in force" of each half of §11.4.7's page pair, from that
half's three tints. A colour whose only zero tint is black keeps every channel of the black
raster and none of the chromatic one, so the runs carried different blend modes,
`geometry_digest` refused the pair, and **the page fell back to the device's three components
with nothing reported**, `is_complete()` true. The question is now asked of the four tints the
clause decides; `Overprint::new` is infallible because the empty set is one of its values, the
protocol's blend tags go from seven to eight, and `implicit_knockout_group` refuses a
non-isolated group as an element (ADR 1169).

## Rows and what moved

§11.7.4.3 `partial` → `implemented`; §11.7.4.4 stays `partial` for its second bullet's unstatable
elements; the aggregate follows it. `MAX_INCOMPLETE` 62 → 61: `issue12798_page1_reduced.pdf`
completes and draws differently — its mark's chromatic half is the backdrop multiplied with
itself — so it is judged again and rejoins `AMBIGUOUS_NON_ISOLATED_POSTER`, outside the bound on
the worst tile alone, where the references part company by the same amount. One position still
reports: a direct element of a non-isolated knockout group.

Handed over: `conformance --bin quotations` did not name a misquotation planted in a rustdoc
blockquote in `crates/`; this round's quotes were grepped against `doc/md/`, and one was wrong.
