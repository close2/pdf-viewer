# 1868 — A zero-length dash at an open subpath's end is painted, in every backend

Slot 8 of batch 127, 2026-10-28, Opus 5.5, one session; ADR 2570 used, ADR 2571 and Q734 unused. Ledger: §8.4.3.6
stays `implemented`, its code, tests and note extended; §8.5.3.2 stays `implemented`.

**Premise.** Round 1859's finding held exactly: `[0 2] 0 d`, `1 J`, `0.6 w` on a straight rule whose length is a whole
number of periods paints 51 dots in `render-raster` and 50 in `render-cpu`, the last dot missing, 18 levels apart at
its pixel. The hypothesis held in its first half: one lane drops the final zero-length dash. `tiny-skia`'s dasher
dispenses only `while distance < length`; `kurbo`'s dispenses the dash that begins at the end.

**Decision.** §8.4.3.6 dispenses the pattern "along the path", both ends included, and §8.5.3.2 paints the caps of a
zero-length dash "always"; a dot at position `L` is on the path as the one at 0 is, and a rule drawn from its other end
places the same dots. Raster was right, the processor wrong (ADR 2570). The references split two against two: Ghostscript
paints the end dot in both directions, MuPDF and Poppler drop it and so draw the backward rule without its own last dot.

**Built.** `pdf-render`'s `dash_end.rs`, `extended_to_a_zero_dash_at_the_end`: each open subpath of straight segments
whose pattern position at its end is exactly a zero-length dash's start is extended by twice `ZERO_DASH` along its last
segment, so every dasher meets that dash inside the path and dispenses it once; a cubic claims no coincidence, as
`closing.rs` does. Called in the zero-length-dash route of all three lanes (`render-cpu` and `render-raster`'s `stroke.rs`,
`render-gpu`'s `scene.rs`), after §8.4.3.4's opening. `closing::period` is `pub(crate)`. Ten unit tests in
`dash_end.rs`; `render-raster/tests/dotted_rule_end.rs` counts the dots on all three backends at eight placements, three
lays and lengths 100, 102 and 101 (51, 52, 51 dots), and holds the processor and raster within 2 levels per pixel
(measured 1).

**Plant.** The processor's call reversed by a patch of this round's own: 50.000 dots of 51 and "18 levels apart", both
tests red; restored, both green.

**Bound.** One walk of the path's commands and one copy where an end is extended. Hostile probe (ignored test, 200 000
subpaths each ending on a dot, `getrusage` children's high-water): 1 272 152 KiB here, 645 060 KiB with HEAD's processor
route — the second dot the clause asks of each subpath; with the same two dots and no extension (length 2.5) 1 272 504 KiB.

**Gates.** `rustfmt --check` on the five files exit 0; `cargo fmt --check` and `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `pdf-render`, `render-cpu`, `render-raster`, `render-gpu` exit 0; `cargo nextest run` on the four exit 0,
731 passed, 3 skipped. In the export `dots/` I worked in: `raster_golden` exit 0, held 974, moved 0; the six arms against
HEAD's six arms, by page: each exit 0, 968, 968, 968, 965, 965, 965 pages, 0 moved (the corpus gate's cpu arm 968 agree, 0 differ), so no corpus page reaches the construction at a pixel. `--bin cited` exit 0, `--bin quotations` exit 0; `cargo test -p conformance
--no-fail-fast` exit 0 (an earlier run's one red was round 1866's record, not this round's).

**Found, not fixed.** A dotted rule containing a cubic that ends exactly on a dash still gets each dasher's own answer
(`crates/pdf-render/src/dash_end.rs`, the `None` length). Vello's ink for one dot moves with its place on the page, by up
to 0.22 of a dot on these rules (`render-raster/tests/dotted_rule_end.rs`, `HALF_A_DOT`).
