# 456 — A half that is not a group is drawn in one

**Finding.** quorra's `2c9bdd0` lifted both refusals `doc/QUORRA_FEEDBACK.md` §14.2 asked about, so
§11.4.6's staged pair became writable — and the part that needed a decision was not the clause but
the halves that are *not* groups: `Compose` sits on a fill and on a `GroupSpec` and on nothing
else, while a `Shaped` element's object may be a stroke, an image, or a fill whose paint takes the
image door. One rule — a group states the operator itself, everything else is drawn inside a group
of one element — draws all four corpus pages into agreement with the CPU oracle.

**Date.** 2026-08-12.
**ADR.** [0291](../adr/0291-a-half-that-is-not-a-group-is-drawn-in-one.md).
**Touched.** `Cargo.lock` (quorra `a35dc70` → `2c9bdd0`), `crates/render-quorra/src/scene.rs`
(`GroupParts`, `Encoder::group`, `shaped`, `stage`), `crates/render-quorra/tests/corpus.rs`
(`REFUSED` 5 → 1), `crates/render-quorra/tests/headless_quorra.rs` (two tests replaced),
`doc/conformance/ledger.toml` (§11.4.6, §11.6.4.2, §11.4.4, §11.3.5, §8.5.4),
`doc/QUORRA_FEEDBACK.md` (the head's verdict table, §9.1, §13.1, §14, §14.2, §14.3, §18, §18.1,
§20.6, §21), `doc/QUORRA_UPGRADE.md` (taken into the tree), `doc/todo/23-transparency-departures.md`,
`doc/adr/0291-*`, this file.

## The bump

`cargo update -p quorra-gpu --precise 2c9bdd05…`, pinned rather than taken from the branch head:
upstream had already moved one commit further to `7a58ced`, a shelf layout aimed at the sheet
exhaustion §6 of the upgrade note describes, and **nothing on this side documents it**. A round
that takes an undescribed revision measures two changes at once. It is written down in
`QUORRA_FEEDBACK.md` §20.6 so the next bump finds it rather than rediscovering it.

`GroupSpec` gained `compose`, which is one field at one site here — and
`StagedComposeReason::InsideKnockoutGroup` is *deleted*, so
`quorra_will_not_take_the_pair_where_this_tree_would_hand_it_over` stopped compiling. That test was
written in session 439 "so that it fails the day you lift the restriction" and it did exactly that.
Its replacement, `quorra_states_what_it_will_not_stage`, holds the two constraints that survived —
no blend mode on a staged half, and a staged group must be isolated — because both are load-bearing
for the translation this round wrote.

## Both coverage lanes, on this machine

`doc/todo/02` §2 owes the second lane whenever a round takes a quorra release. All four rows, whole
corpus, gates profile, RADV — and the pair with the upgrade note's own laptop table is the finding:

| | agree | differ | refused | quorra | oracle |
|---|---:|---:|---:|---|---|
| scale 1, `cpu` | 919 | 37 | 1 | 5.38 s | 2.44 s |
| scale 1, `gpu` | 918 | 38 | 1 | 5.14 s | 2.42 s |
| scale 4, `cpu` | 929 | 16 | 7 | 22.75 s | 11.43 s |
| scale 4, `gpu` | 930 | 14 | 8 | 19.59 s | 11.61 s |

**Every verdict count is quorra's laptop plus four agreements and minus four refusals, in all four
rows** — the four §11.4.6 pages this round wrote. Two machines, two adapters, and the page-level
outcome is identical, which says these columns are a property of the pair of rasterisers rather
than of the card. The clocks are this machine's and say something weaker than theirs: the device
lane is 14% faster over the corpus at 4× here where it was a third faster there, and the two lanes
are within 5% of each other at page scale.

The one refusal the device lane adds at 4× is `Test-plusminus.pdf`, over the frame budget by 4% —
and it is one of the two pages `7a58ced` is aimed at.

## Gates

`fmt`, `clippy -p render-quorra`, `nextest --workspace` (1634 passed), doctests, the pdf-model
corpus gate, the oracle (905 agree / 68 contradicted / 786 ambiguous / 18 no render — unmoved, and
the reference cache hit 99.7%), both text gates, dates, xmp, jpeg2000, the quorra corpus gate on
both lanes at both scales, and `conformance`. The oracle and the corpus gates were run to establish
something as well as to pass: **nothing this round did could move them**, and they say so.

## Two things the next round should know, neither of them about this work

1. **This tree was not clean when the round started.** A sibling session's uncommitted work sits in
   it — parametric interpolation for the mesh shadings, `crates/pdf-model/src/mesh.rs` and six
   files beside it, plus an untracked `crates/pdf-model/examples/mesh_census.rs` — written between
   19:03 and 19:12 and idle since. It calls itself session 456 in a ledger note and cites an
   **ADR 0291 that was never written**, which this round has now used for its own. That work was
   left exactly where it was and is *not* in this commit, which names its paths; whoever resumes it
   renumbers the ADR. Its effect on every gate above was checked rather than assumed: the quorra
   corpus gate's differing list is unchanged page for page and the oracle's four verdict counts are
   unmoved, so no number in this round's tables is theirs. It does carry four
   `clippy::cast_possible_truncation` warnings in `pdf-render/src/shading.rs` and
   `pdf-model/tests/shadings.rs`, which is why `clippy --workspace` is not silent in this tree
   today and why the clippy gate above names one package.
2. **The citation checker reads `/// - §` as a foreign document.** `another_document` takes the two
   whitespace-separated words before a `§`; in a doc-comment bullet those are `///` and `-`, and
   `///` passes the acronym test because `/` is allowed for names like `ISO/IEC`. The message it
   prints is confident and wrong ("a `§` after /// -, which is not ISO 32000-2"). The tree has no
   other instance, so the cheapest cure was to reword; the checker is one condition short and
   nobody has spent it.

## And one about the document

`doc/QUORRA_UPGRADE.md` arrived untracked and is **tracked now**, deliberately. It is the exact
counterpart of `doc/QUORRA_FEEDBACK.md`, which has been in the tree since it was written: one is
what this project measured and asked for, the other is what came back, what it costs to take and
what it makes possible. Three sections of the feedback document now cite it, the round's decisions
rest on §2's construction and §4's ratchets, and a round that reads only one side of a conversation
reads it wrongly. Leaving it untracked would have made the ADR cite a file that is not in the
repository.
