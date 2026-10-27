# 1817 — Table 13's default of 1 governs a three-component frame whatever its identifiers spell

Slot 2 of batch one hundred and twenty-two, 2026-10-27, a "clause" round on Sonnet at high effort; ADR 2468;
ADR 2469 and Q683 unused. I worked in `scratchpad/r1817/` (exports `dct` and `head`).
**Row moved:** §7.4.8 `departed` to `implemented`, on "the default value of ColorTransform shall be 1 if the image
has three components and 0 otherwise" applied to a frame with neither the Adobe marker nor the entry, `R`, `G`, `B`
identifiers included — A668 item 1 ("Q668 7.4.8 agree"). **Premise** held: one decision site
(`image::colour_transform`'s last arm returned `None`), and `decode_jpeg` already transformed a frame the decoder read
as `RGB` where the entry said 1. **Hypothesis** (a handful of corpus images move, each toward the clause) held at
one: `colour_transform_census` now lists the images in this case with `RGB` identifiers — 1 image over 1 document
(`issue11931.pdf` object 5) among 974 documents, 0 among the other 3415.
**Moved:** against HEAD's six arms exactly one page moves in each of six, `issue11931.pdf`; no other. Drawn before and
after by `render_at`, 83 602 of 921 600 pixels differ (the grey dome becomes a magenta bar with a green arc, what
`ghostscript` paints). `raster_golden.tsv` takes that page's line, the gate's own update naming it the one move.
**Built.** The arm in `crates/pdf-model/src/image/dct.rs` and its comments; `tests/colour_transform.rs` replaces the
departure fixture with the ending (the transformed pixel to within one count, identifiers changing nothing, and an
out-of-range entry leaving the default); the census example prints the images; `oracle.rs`'s doc on
`AMBIGUOUS_JPEG_COMPONENT_IDS` says the page stays ambiguous because three references read a convention no clause
states; the row's note and `doc/state-of-play.md` say the clause is executed throughout.
**Found, not fixed.** Oracle-versus-raster mean error on that page is 0.0003 of 255 at 1× where HEAD's was 0.0000
(the gate passes); not chased. The banded direct path still declines an `RGB`-read frame, as before.
**Gates.** `cargo fmt --check -p pdf-model` exit 0 and `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
--all-targets` exit 0 (export); `cargo nextest run -p pdf-model` 2302 passed (export, debug worker built); `--test
colour_transform` 17 passed; six arms against HEAD's: exit 0 each, one page moved each; `raster_golden` exit 0, 1
moved; `cargo run -p conformance --bin cited`, `--bin quotations`, `--bin departures` exit 0 (§7.4.8 no longer listed).
`tools/state.sh --round 1817 docs` (rustdoc `-D warnings`, whole workspace) exit 0; `cargo test -p conformance
--no-fail-fast` exit 101 with one red, `every_quotation_is_the_standards_own_words` naming
`crates/pdf-transform/src/archive/fonts/bystanders.rs:6`, a sibling's (my own `colour_transform.rs:499` was the first run's
second name and is fixed); every other target passed.
The validator's three comment findings (a count, two history clauses, one claim the test did not make) were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on three comment defects and PASS on the code, the tests, the plants and the clause reading; the round fixed the finding.
