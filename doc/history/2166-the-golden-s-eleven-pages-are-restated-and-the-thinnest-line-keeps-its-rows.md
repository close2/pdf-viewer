# 2166 — The golden's eleven pages are restated, and the thinnest line keeps its rows

Session 2166, an oracle round in `pdf-viewer-gwg` at `bec8ae27`. ADR 3365 is the strips fix, ADR 3366 the golden's
table; ADRs 3367 and 3368 and Q987 unused.

- **Method (ADR 3051 §1).** The check walk at `bec8ae27`, no wall-clock refusal, held 963 and moved round 2163's 11.
  Round 2111's scratch dump, now also drawing each page at 2, 4, 8 and 16 strips, was built at `3b218c9f`,
  `7e990758~1`, `7e990758`, `bec8ae27~1`, `bec8ae27`, `dd12e73b~1` and `dd12e73b`: equal to the golden at `3b218c9f`
  on all 11, equal to the update walk's lines at `bec8ae27`.
- **Six raster-only pages at `7e990758`** (ADR 3333): lists and reports equal; each strokes `0 w` or a `/SA` stroke
  §10.7.5 promotes. Stroked glyphs lose their mitre tips (`r2166/side-zerowidthline.png`, `side-issue19360.png`).
- **Four reports-only pages at `bec8ae27`** (ADR 3353): `ET`, `TJ`/`Tm`, `Td`/`Tj`/`Tm` outside a text object.
- **`PDFBOX-3148-2-fuzzed` at `dd12e73b`** (ADR 3281): ASCII data without its EOD marker, reports only.
- **Restated** by the update walk: its diff is exactly the 11 lines, each the dump's value. The check after it held 974.
  No page left moved.
- **The strips failure on `bug1743245`** (ADR 3365): at `7e990758` the page's `/SA` rules under scales 0.12 % apart
  became `tiny-skia` hairlines, but `strips.rs`'s `converted_row_by_row` still said they were drawn a row at a time
  by `area`, so the planner cut through them. It now answers `false` for a stroke `strokes_in_device_space` holds of.
  Planted back, the corpus test fails with 2163's numbers exactly. A second change (hairline vertices on the page's
  grid) was built, measured to change nothing, and dropped. The bound is unchanged.
- **Ledger.** 8.4.3.2 stays `implemented`; `strips.rs` and the new `stroke_cuts.rs` test join its lists.
- **Habit.** When a branch sends a mark to another drawer, ask the strip planner's predicate the same question: the
  planner's claim about a mark is a second copy of the draw's dispatch.

**Gates.** `raster_golden` check 101 (held 963, moved 11), update 0 (diff the 11), check 0 (held 974); strips test
101 at HEAD, 0 after. Over `-p pdf-model -p render-cpu`: fmt 0; clippy `-D warnings --all-targets` 0 (101 first:
`excessive_precision` on a test literal); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`)
3205 run, 1 failed — 2148-B04's, the manifest's one ("failed 1, expected 1"); rustdoc `-D warnings
--document-private-items` 0 for each; `cargo test -p conformance --no-fail-fast` 101 on the first run (`records`: this paragraph unwritten), 0 after. No public type
changed, so no `fuzz` check is owed.
