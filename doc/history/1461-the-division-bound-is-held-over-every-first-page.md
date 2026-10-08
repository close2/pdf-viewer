# 1461 — The division bound is held over every first page, and the pages past it by name

Slot 6 of batch seventy-two, 2026-10-08, a pixels round. ADR 1758; no row moved, no question.

**Premise.** Mostly held. The 150 pages and 7 043 pixels at sixteen strips match ADR 1742; the
one extra pixel is `issue21579.pdf`, which opens with its published password. Two parts did not
hold. The two bounds are in `crates/pdf-model/tests/strip_parallelism.rs`, not in render-cpu's
file of that name, which asserts byte equality over seven scenes. And not every moved pixel is on
a horizontal edge: `blendmode.pdf`'s are single pixels inside photographs drawn under eleven blend
modes, and it moves **3** levels at two, four and eight strips.

**Measured.** All 967 drawn first pages at one strip against 2, 4, 8 and 16 (ADR 1758 §1): 153
pages move at some division, at most 1 078 pixels (0.358%) and 3 levels. No page outside ADR
1742's eight passes a fixture bound at any of the four divisions. Looked at with the moved pixels
painted (trap 1): sample-row edges (`issue1350`, `issue7020`, `pdfjs_wikipedia`), two marks' edges
in one pixel (`issue7014` row 369, `issue12810`), glyphs under a Multiply highlight (`comments`,
`highlights`), and blended photographs (`blendmode`).

**Built.** A second ignored test in `raster_golden.rs` that runs in the golden's walk. Every first
page must stay within one level and one pixel in a thousand, or be one of the eight in
`PAST_THE_BOUND`, each with its reading and held to its own ceiling. A ninth page fails, and so does
a named page that moves more. The fixture constants stay, because they are derived for one
coverage: §3 of the ADR. The comments in render-cpu and `strip_parallelism.rs` that gave ADR
0219's three-page figure as the property's now cite the corpus figure.

**Fixtures read.** The `CpuRasterizer::new()` call sites were read by a delegated audit and two
were checked by drawing. `inline_image_abbreviations.rs`'s 900 × 900 page is drawn in **1** strip
here (24 CPUs) and in **2** on two to eight CPUs, where one label pixel at (269, 653) moves. That
pixel is outside the eight blocks the test compares, so the test holds on every machine.
pdf-transform's oracle page (ISO 32000-2 p100 at 150 dpi) gets 14 / 4 / 2 strips, and 277 to 397
pixels move between counts. It is held exact only because both sides ask the same machine.

**Gates.** `rustfmt --check` on the three Rust files: exit 0. `cargo clippy -p render-cpu -p
pdf-model --all-targets` with `-D warnings`: exit 0. `cargo nextest run -p render-cpu`: 176 passed;
`-p pdf-model`: 2 005 passed, 19 skipped. Run from an export of HEAD plus this diff only, with its
own target directory: `raster_golden` (both tests) exit 0, held 974, moved 0, 31.6 s. The division
test with ADR 0219's defect planted: exit 101, 7 pages; with ADR 0138's: exit 101, 14 pages. The
six arms exit 0, 0 digests and 0 means moved over 5 831 page lines against `/home/AI/arms-1456/`.
`turn_path --clock` twice: exit 0, 33 of 33 figures judged and inside. `cargo test -p conformance`:
exit 0, 425 passed in 55 binaries. Duration 6 000 s.
