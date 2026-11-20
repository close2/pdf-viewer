# Round 2155 — the sixth specification audit: §8.4 and §8.5, graphics state and paths

An audit round in the `gwg` worktree, made in the shape of review 2038 and audits 2128 to 2148,
turned on the rasteriser as the reader on the way out: §8.4.1 to §8.4.5 (Tables 51 to 57) and
§8.5.1 to §8.5.4 (Tables 58 to 60) against `pdf-model`'s line parameters, dash, `ExtGState` and
path operators, and against `render-cpu` with `pdf-render`'s shared stroke decisions. It fixes
nothing. No ledger row moves.

## What it found

- `doc/reviews/2155-specification-audit-8-4-8-5.md` and its directory. Five findings: 4 B, 0 T, 1 D.
- B01 a zero-width line is one device pixel only along the CTM's widest stretch (a quarter pixel
  across under `4 0 0 1 cm`); B02 a valid dash finer than `tiny-skia`'s million-dash limit erases
  its stroke silently; B03 a zero-width line is mitred, where §8.4.3.5 makes its mitre length zero;
  B04 a non-number in a dash array is dropped, so `[3 (x) 1]` becomes `[3 1]` through `d` and `/D`.
- D01 `line_cap`/`line_join`'s initial value for an out-of-range code is written as §8.4.1's answer.
- Four failing `*_spec_audit` tests, each with a control: three in
  `crates/render-cpu/tests/all/stroke_parameters_spec_audit.rs`, one in
  `crates/pdf-model/tests/graphics_state_paths_spec_audit.rs` (`content_suite`); listed in the
  audit's `expected-failures.json` and appended to `expected-failures-current.json`.
- About twenty further candidates were probed end to end and hold; the review lists them.

## Ledger

Twenty-five §8.4 and §8.5 rows `implemented` before and after. B findings contradict 8.4.3.2,
8.4.3.5, 8.4.3.6, 8.4.5 and 8.5.3.2.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p render-cpu` exit 101 on
the first run (an unfulfilled `cast_precision_loss` expectation in the new test), exit 0 on
the second. `pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests -p pdf-model -p render-cpu`
(`xvfb-run -a`): 3187 run, 3182 passed, 5 failed, 23 skipped — the manifest's five ("failed 5,
expected 5"). rustdoc `-D warnings --document-private-items` exit 0. `cargo test -p conformance
--no-fail-fast` exit 0. Fuzz not owed: no public type changed.

## Habit

A measure stated at device resolution keeps that origin under every transform: §8.4.3.2's zero
width is "1 device pixel", so a conversion through the path's space that is right for a similarity
is the place to ask what it does under a stretch. Probe with ink summed over a box, not one pixel,
when the mark is a pixel wide.
