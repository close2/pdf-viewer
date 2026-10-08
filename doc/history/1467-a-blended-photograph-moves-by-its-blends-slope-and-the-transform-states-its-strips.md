# 1467 — A blended photograph moves by its blend's slope, and the transform states its strips

Slot 6 of batch seventy-three, 2026-10-08, a pixels round. ADRs 1768 and 1769; Q362 not used. No
ledger row moved.

**Premise.** Partly held. ADR 1758's table and the `CpuRasterizer::new()` calls the brief's grep
names hold. Two parts did not. The Saturation cell of `blendmode.pdf` reaches 2 levels, not 3; the
3-level pixels are one
Hue pixel and one ColorDodge pixel, at 2, 4 and 8 strips. And the oracle page's 277 to 397 pixels
are against one strip; between the 16, 4 and 2 strips machines ask for, 94 to 190 pixels move, worst
1 level.

**Hypothesis.** Held, narrowed (ADR 1768). Each image of a cell was drawn alone on the full list, so
the planner cuts where the page cuts. Under strips every input that moved moved one level in one
channel, and in Table 135's four cells every moved pixel sits on a moved input. The worst gains:
ColorDodge 76 → 77 under a source of 177 gives 248 → 251, the slope `1 / (1 − Cs)` = 3.27; Hue on a
source nine levels from neutral gives 3. The ladder of 1 to 8 192 units of `ty` (to 0.5 px): input
flips grow with the offset and stay one level up to 1/64 px. Output gain is about 1 for Normal and
up to 12 for ColorDodge. For Saturation and Hue it is 48 to 93 from 1/1024 px, a jump at neutral
colours rather than a slope, so no closed-form output ceiling exists.

**Built.** `strip_parallelism.rs` gains a test that holds every photograph of the page, drawn alone,
to one level at the golden's four divisions, and Table 135's four cells to their inputs. The
`PAST_THE_BOUND` note says what the moved pixels are. `quorra-transform render` states
`render_cpu::MAX_STRIPS`, and so do its oracle and the seam test. Before, page 100 from the program
was three files under three CPU masks; now it is one file, the one this machine wrote before (ADR
1769).

**Gates.** All from an export of HEAD with this diff and its own target directory, because siblings'
mid-edits broke the shared build. `rustfmt --check` on the six Rust files: exit 0. `cargo clippy -D
warnings` for `-p pdf-model` and `-p pdf-transform --all-targets`: exit 0. `cargo nextest run -p
pdf-model`: 2 006 passed, 19 skipped; `-p pdf-transform`: 489 passed, 10 skipped. Calibration: a
1/16 px strip shift and a 1/64 px layer-only shift each fail the new test, naming the cell; the
unplanted tree passes. `raster_golden` (both tests, `--tree 6`): exit 0, held 974, moved 0, 153 pages
at most 1 078 pixels and 3 levels, 37.6 s. `pdf-transform --test gate` under `--clock`: exit 0,
183.8 pages/s, page 100 byte for byte. Its lock line shows `batch=-` because it ran from the export.
The six arms and `turn_path` were not run: `cargo tree -p render-raster` names no `pdf-transform`, and
`pdf-model`'s only change is a test target, so neither build graph changed. `cargo test -p
conformance` in the shared tree: exit 101 once, on `batch.rs`'s fuzz-check kind while slot 5 was
editing it, then exit 0. Duration 2 700 s.
