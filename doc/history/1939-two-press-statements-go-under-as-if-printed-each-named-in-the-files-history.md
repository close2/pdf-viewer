# 1939 — Two press statements go under `as-if-printed`, each named in the file's history

Session 1939, a single archive round. ADR 2693; no question; ADR 2694 not used.

**The premise.** Half held. Round 1938's census refuses the four under `as-if-printed` at 2b and 4
by R17 alone: `endchar`, `reserve3` and `ESUjulyaugust2002` by
`graphics/no-halftone-name` (Distiller's `/HalftoneName (Default)`), `102952` by
`graphics/no-overprint-mode-one-under-icc-cmyk` (`/OP true /OPM 1` over DeviceCMYK marks on pages 8
and 9). `no-data-loss` does not refuse the name (it states it on an appended page and discards it);
it refuses the overprint mode, and still does.

**What this reader draws.** Neither moves a pixel. With the halftone key renamed, `reserve3`'s 18
pages and `ESUjulyaugust2002`'s 140 draw identically at 72 dpi. The mode is composited only in a
group blending in four components (§11.7.4.3, still refused); `102952` states no `/Group`, and its
77 pages draw identically with the mode at 0 and `/OutputIntents` renamed.

**The construction (ADR 2693).** `as-if-printed` answers both sites `discard` as `as-shown` does
(losses `halftone-name`, `overprint-mode`): a print scanned back in (A694) was screened and mixed by
its own printer. Every profile taking either writes it into `xmpMM:History` as image smoothing is
(ADR 2498), the halftone and its name or the graphics states and their marks' pages, a condition of
the rewrite (`NO_PLACE_TO_RECORD_A_PRESS`); a restatement that costs nothing records nothing.

**Kept.** `no-data-loss` and `state-every-loss` refuse `102952` by `overprint-mode`; a mode the
window composites by is refused under every profile. **Tests**: `tests/archive/press_statements.rs`
(6), `outside_the_packet` in three older ones; planted `stop` entries and dropped rows fail 7 of 25.

**Census**, against round 1938's runs. Shipped: 8 NEW (`as-if-printed`, the four at 2b and 4), 8
CHANGED (`as-shown`'s same four), 5077 identical. Stating: 12 CHANGED (`no-data-loss` and
`state-every-loss`, the three halftone files at both parts), 4549 identical. Every CHANGED file
differs only in its packet's new history entry and the offsets after it; the new outputs are
`as-shown`'s, byte for byte once dates are masked. Drawn beside the source, `102952` differs
on pages 8 and 9 (at most 8 levels) and `endchar` by at most 7, and both are the output intent's
(0 with it renamed). `reserve3` and `ESUjulyaugust2002` differ by at most 1 level, which is the font
rewrites their report lists. R17 frees 0 now, and the table was regenerated.

**Gates.** fmt and clippy `-D warnings --all-targets`, `-p pdf-transform`: exit 0 each. nextest
`-p pdf-transform -p pdf-archive`: 1394 passed, exit 0. Archive tier (`corpus`, `archive_corpus`,
`witnesses`): exit 0 each. Both censuses: exit 0. veraPDF 1.30.3: all 28 new and changed outputs
pass. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 101 on a rustdoc
panic in `viewer-host`; rerun with `CARGO_INCREMENTAL=0`, exit 0.
