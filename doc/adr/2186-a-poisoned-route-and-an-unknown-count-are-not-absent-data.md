# 2186 — A poisoned route and an unknown count are not absent data

Status: accepted. Session 1675. Context: ADRs 0417, 1254, 2170; ISO 32000-2
§11.3.4, §11.5.3, §11.6.6 and §11.7.2.

`Presses::rgb_route` used `lock().ok()?`: a thread panic made a valid RGB route
indistinguishable from a colour space for which no route could be built.
`Presses::named` and `presses_cached` made the same fault a count of zero.

Each now returns `Result`, preserving `NoPress::Poisoned` through the caller.
`rgb_route` retains `Ok(None)` for a space without a route. A page names its
fallback to device compositing, a group names inheritance from its parent,
and a soft mask names its device luminosity fallback. None calls the fault a
budget spent by the document. The public `Interpretation::presses_named`
retains the unknown result; the parallel-piece admission refuses an unknown
count and the census cannot print a fabricated zero. Cache instruments either
name the fault or stop explicitly.

The counter tests poison their actual mutex from a thread. The process cache's
test runs in a child test process so that its intentional poison cannot corrupt
neighbouring tests. The route test establishes that the same CalRGB space has
a route before poisoning. The budget sentence names the budget rather than an
obsolete ninth/eight count; the governing limit remains `MAX_PRESSES`.

This changes no successful conversion or cache policy. It makes absence and
failure different types at the API boundary, so a new caller must choose how
to handle a fault instead of inheriting silent fallback from `Option`.
