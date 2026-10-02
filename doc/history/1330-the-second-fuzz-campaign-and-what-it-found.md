# 1330 — The second fuzz campaign, and what it found

Robustness slot of batch fifty-one. ADRs 1495, 1496, question Q227. No ledger row moved.

**Targets.** Five new, each with its `verify.md` line and `seeds.sh` arm: `embed`, `jpeg_bands`
(differential, through `pdf_model::image::banded_decodes`), `meet` (differential against an exact
integration, through `raster_gpu::intersection`), `vfs_write`, `find` (through
`viewer_core::find_in_text`, the one caller of `fold`/`decompose`/`mark_class` with a page's
text; `shaping` reaches `Label` only). `seed_confined_wire.py` had stopped: three protocol
questions unasked; it asks them.

**Stock-take, INITED edges (disk corpus / fresh seeds).** Stale on the disk: `display_list`
105/957, `forms_data` 270/1241. Otherwise the disk's corpus leads: `document` 3983/2664,
`serialize` 4143/3077, `page` 31553/22608, `variable_text` 6341/5352, `xmp` 1584/689,
`confined_wire` 3689/390, `crypt` 837/1064, `cmap` 368/360, `ccitt` 223/122 (and 223 at DONE after
141 M runs). New targets, fresh only: `vfs_write` 5929, `linearize` 5743, `jbig2` 2480, `jpx` 2097,
`xfdf` 1895, `jpeg_bands` 1619, `embed` 1240, `shaping` 830, `find` 627, `meet` 526.

**Runs.** All 28 targets, 600 s each behind the lock (`page` at its own 4096 MB, `jbig2` also with
`-fork=1 -ignore_timeouts=1`); executions and DONE edges per target are ADR 1495's table. Clean on
the first run: 23; `embed` and `vfs_write` clean on their re-runs, `jpeg_bands` stopped at Q227. Findings, each fixed with a test that fails without the fix:
- `embed`: a face written whole (`glyf` or `CFF `) wrote glyphs it does not hold and no `.notdef` —
  refused now as the subsets were (`a_face_written_whole_is_refused_a_glyph_it_does_not_hold`,
  `a_whole_face_is_refused_a_glyph_it_does_not_hold`).
- `jpeg_bands`: a damaged restart interval banded differently from the whole decoder — bands are
  decoded strict; a scan with no `EOI` was cut where the whole decoder stops short — both plans
  decline it (`tests/banded_decodes.rs`, fixture `tests/restart/damaged_interval.jpg`).
- `vfs_write`: a page tree naming its own ancestors counted 449 367 pages and hung a mount — a node
  met beneath itself stands for no pages (ADR 1496,
  `nodes_that_name_their_own_ancestors_stand_for_no_pages`).
- `jpeg_bands`, third run: `zune-jpeg`'s DC dequantise overflows (panics under overflow checks,
  wraps in release); one-line patch and reproduction in `doc/patches/`, the fork is Q227.
Known, not fixed here: `jbig2` timeouts in `hayro-jbig2`'s symbol dictionary (ADR 1447's patch);
`page` at 3.16 GiB / 26 s on a mutation of `ContentStreamCycleType3insideType3.pdf` (`todo/49`).

**Gates.** clippy `-D warnings` on pdf-font, pdf-model, viewer-core, pdf-vfs, raster-gpu and the
fuzz workspace: 0. nextest pdf-model, pdf-font, viewer-core, pdf-vfs 2377 passed, exit 0;
raster-gpu 668 passed, exit 0. `cargo test -p conformance` 370 passed, 0 failed.
`cargo +nightly fuzz build -O -s none`: exit 0. Tier 2 behind the lock: `pdf-model --test corpus`
exit 0, `raster_golden` exit 0 (974 held, 0 moved). rustfmt `--check` on every file of mine: 0.
