# 662 — The group that argued without a clause

One contradicted group, taken apart. Parallel round, worktree `r662`, branch `round-662`.
**No pixel moves**; what changed is a group note that argued a verdict about the references with no
clause under it, a ledger row, an item in `doc/todo/11`, and three traps. ADR 0489 has the argument and
the tables.

## Which group, and why

656's criterion — how many of a group's own members its note measures — is spent. This round asked
the question one level out: **a contradicted verdict is the claim that the standard rather than the
consensus decides the page, so how many clauses does the note cite?** Counted over all fourteen
non-empty groups, taking every `§n.n.n` in the doc comment and looking each up in the ledger:
seventeen for `CONTRADICTED_DEVICE_CMYK_CONVERSION`, eighteen for `CONTRADICTED_SUBSTITUTED_FONT`,
one for `CONTRADICTED_GLYPH_EDGES` — and **zero for `CONTRADICTED_TIGHT_CONSENSUS`**, whose name is
itself a statement about the references. Every cited row the ledger has is non-`unreviewed` and the
two citations with no row are §2 and §6.3.2.2, neither a technical clause, so the count has one
answer.

## What the three pages are

All three are §10.7.4, in three different paragraphs, and the note named none of them.

`issue7891_bc1.pdf` page 1 is a black rectangle filled through a luminosity soft mask whose group
draws a 676 × 436 greyscale image reduced 2.778-fold, so the page is `255 × (1 − L)` away from the
stroke. Two closed forms were written out — §10.7.4's point sample and the exact area average — and
on the tile the gate actually fails at, ours is the arithmetic at **0.166 with a worst pixel of 1
level** while the two that vote are 4.596 and 6.723 from it; our 6.725 against `mupdf` *is* `mupdf`'s
6.723 from the form. The note had blamed the reduced word; 68.3% and 71.5% of the distance is seven
lines of pixels, all of them edges, and the biggest of them is the mask group's `/BBox`, where
§10.7.4's clipping paragraph makes a clip a set of pixels and we anti-alias it.

`colors.pdf` pages 1 and 2 were 643's. The note said ours is the closed form quantised to
`tiny-skia`'s quarter; **ADR 0476 made ours the exact form three sessions later**, and the correction
reached the trap file, the ledger row and `doc/todo/11` item 7 and not the group. Re-derived
independently here: ours differs from the exact form on **0.0000% of either page**, mean 0.0026, worst
pixel 1 level and 2.

## The eighth rung

Rows 213 and 370 of `issue7891_bc1.pdf` are covered 0.504 and 0.456 and we paint them 0.2549 and
0.2079 — those numbers squared. `crates/pdf-model/examples/coincident_edge_probe` is new and states
one rectangle twice, four ways, with and without a soft mask worth 1.0 everywhere:

```text
  restated as     no soft mask     soft mask
  fill alone            0.5059        0.5059
  W n clip              0.5059        0.5059
  form /BBox            0.5059        0.5059
  group /BBox           0.5059        0.2549
```

Seven rungs give the edge its own coverage and the eighth squares it. §11.4.4's NOTE 5 flattens a
group away unless a soft mask is in force, so `draw_group`'s blit is only *reached* with a mask
beside it — which is why `doc/todo/11` item 4's last open bullet had no small witness for nineteen
sessions. Not fixed: those two rows are 0.0197 of a distance of 0.1721, so paying it moves the page
toward the bound and past neither.

## Changed

- `oracle.rs` — `CONTRADICTED_TIGHT_CONSENSUS`'s note rewritten around the measurements, citing
  §10.7.4 by paragraph three times.
- `crates/pdf-model/examples/coincident_edge_probe.rs` — new.
- `doc/conformance/ledger.toml` §10.7.4 — the ladder and the independent confirmation of ADR 0476.
- `doc/todo/11` item 4 — the ladder, why the residual hid, and the corpus witness.
- Trap 1 — a note's third way of being wrong; trap 9 — a seventh entry, a pair agreeing with the
  clause on one line of pixels and against it on the next; trap 12 — its standing witness measured
  on the tile that fails rather than only over the page.

## Owed

- Item 4's group blit still needs a shape channel beside a group's raster.
- A criterion for the next round; this one's is spent.
- Nothing links a group's note to the code it describes. Trap 1 states the habit and no gate has it.
