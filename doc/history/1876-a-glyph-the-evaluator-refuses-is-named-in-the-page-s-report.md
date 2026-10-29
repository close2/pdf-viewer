# 1876 — A glyph the evaluator refuses is named in the page's report

Slot 6 of batch 128, 2026-10-29, a "parsers" round (robustness, Opus 5.5), one session; ADR 2586, ADR 2587
and Q752 unused. I worked in `scratchpad/r1876/` and one export of this tree built there. No ledger row
moved: the slot is principle 1's.

**The change.** `LoadedFont::build_outline` turned every evaluator error into `None` with `.ok()?`; the
outline cache now keeps `Result<Option<Arc<Path>>, OutlineRefused>`, `outline` still answers `None` for
a refusal, and `outline_refusal(code)` reads the same entry. `account_for_a_missing_mark` asks it after
`glyph_lost_to_repair` and before the program's empty answer, `Coverage` keeps the count and the first
refusal, and `complete` writes one `Unsupported::Font` per font naming the glyph and the evaluator's
`Display` text. The corpus gate classes the line as `Whose::NeitherOne` (ADR 2586).

**The premise that did not hold.** The brief put `show.rs` in `pdf-font/src/loading/`; it is
`pdf-model/src/content/text/show.rs`, and the report site is `content.rs`'s `complete`. And the
refusal was not reported as nothing on every page: where the font drew nothing else, HEAD said
"program has no outline for any of the 1 code(s)", which is untrue of a program the evaluator stopped.

**Plant.** `refused_glyphs.rs` draws round 1869's fan-out (ten levels, *k* = 5) through a `/Type1C`
glyph: the report reads "font /F1: the evaluator refuses the outline of glyph 2 (the CFF font program
could not be read: CharstringNestingDepthLimitExceeded), and 1 of the code(s) …"; at *k* = 3 the same
glyph draws and the page reports nothing. With the refusal arm replaced by `None` the first test went
red on HEAD's no-outline sentence, and green when restored. `getrusage`: 11.9 MB refused, 12.5 MB drawn.

**Unfinished.** A refused glyph still costs up to `cff::CHARSTRING_OPERATIONS` operations once per
distinct glyph (not this slot). `character_glyph` (`loading/glyphs.rs:192`) still drops a refusal of
an interface face into `None`, and a code inside a Type 3 glyph description has no readback and does
not reach `account_for_a_missing_mark`.

**Gates.** In the worktree: `cargo test -p pdf-font` 313 lib passed, exit 0; `cargo test -p pdf-model`
2361 passed, exit 0; `refused_glyphs` 2 passed, exit 0; `--bin cited` exit 0; `--bin quotations` exit 0,
no divergence in a file of mine; `cargo test -p conformance --no-fail-fast` 469 passed and `records` red
on this record's unfilled paragraph, mine, then `--test records` exit 0. In the export: clippy `-D
warnings` on `pdf-font` and `pdf-model --all-targets` exit 0; `cargo fmt --check` on both exit 0;
rustdoc `-D warnings` on both exit 0; the six arms exit 0 each, 0 pages moved against HEAD's six arms
(968, 968, 968, 965, 965, 965 pages; the comparison counted a planted digest as 1); `raster_golden
--ignored` 2 passed, exit 0; `pdf-model --test corpus --ignored` exit 0, 60 incomplete, the ratchet held,
and no corpus document reports a refused glyph.
