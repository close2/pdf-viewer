# 3420 — An outline's resolution is counted in walks of the page tree, not timed

Session 2179. Status: accepted. Context: ADRs 0124, 3114, 3134, 3277.

## Decision

`outlines::an_outline_resolves_against_the_page_tree_once` compared the wall-clock time of
resolving 988 outline items against one `Pages::index_of`, and failed under three lanes' load
(42 ms against 1.7 ms) with nothing wrong. `Pages` now counts the walks it begins —
`Pages::searches`, one relaxed atomic increment for each `index_of` and each `indices`, never one
per node — and the test asserts one search for the whole outline, after a control that one
`index_of` is one search. A per-item walk counts nearly a thousand and fails; ADRs 3114, 3134
and 3277 made the same move for other tests.

## Cost

One `AtomicUsize` in `Pages` and an increment per walk, against walks that read tens to
thousands of dictionaries; atomic so that `Pages` stays `Sync`.
