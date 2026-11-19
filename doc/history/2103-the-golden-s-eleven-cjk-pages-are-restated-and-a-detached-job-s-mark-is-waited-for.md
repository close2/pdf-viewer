# 2103 — The golden's eleven CJK pages are restated, and a detached job's mark is waited for

Session 2103, an oracle round in `pdf-viewer-audit` at `9095a81e`. ADR 3113 is the golden's table, ADR 3114 the
timing test's; ADR 3115 and Q925 unused.

- **Method (ADR 3051 §1).** `b60382af` (round 2098) was built in a scratch copy and its golden written by the walk's
  update mode: equal to the file on all 974 pages, so the washed and mesh strokes moved no first page. `720e1e1b`
  (test targets) and `59c77c0b` (`pdf-transform`) are outside `pdf-model`'s tree. The check walk at `9095a81e`
  held 963 and moved 11. Round 2095's scratch dump, built at `59c77c0b` and `9095a81e`, equals the golden at
  `59c77c0b` on all 11.
- **Ten pages, glyph outlines only:** with points and path verbs set aside the lists are equal and the reports
  equal. Mincho to Noto Serif CJK (ADR 3102), and `issue19182`, `issue2128r`, `issue3521` in their region's face
  (ADR 3101). `issue3521`'s `黑体` is drawn serif because its descriptor states `/Flags 6`.
- **`issue14438`, not in round 2100's list:** five glyphs of an unembedded `STSong` ("title") take Noto Serif CJK
  SC's outlines and lie inside their box now, so ADR 0155's rule takes the box clip off them (`ClipId(2)` to
  `None`); every other item equal, 135 pixels differ.
- **`issue19517`, left moved by load.** The first update, at a load near 200, drew its other value (list and
  reports, the one round 2095 saw once at `fa29fe4c`); the check after drew the file's. Not explained by any
  commit, so not restated: a second update drew the file's value. Likely `pdf-sandbox`'s 30 s wall-clock decode
  deadline on its JPEG 2000 image; the same nextest failed `thumbnails` on it and `matte_reduced`, both passing
  alone. A finding for a later round.
- **Restated:** the second update run in the worktree, whose diff is exactly the 11 lines; none written by hand.
  The check after it held 974 and moved 0.
- **The timing test.** `open_returns_while_the_warm_build_and_the_arms_export_run_detached` waited twenty seconds
  for each of the export's marks. With the slice at 65 of 66 GiB and processor pressure at 89 %, it failed once in
  three with the merge's message; a sampler saw the export's wrapper asleep 20 s before `arms-held` (at `nice 19`)
  began. Each wait now ends at the mark or at the export's end (`marked_before_ending`). A 30 s `SIGSTOP` of the
  export fails the old test at 20.6 s and passes the new at 30.6 s; a misspelt lock line fails the new at 124 s.
- **Trap:** `pkill -f "<loop text>"` from the shell that started the loops kills that shell (round 2095's trap,
  met again).

**Gates.** `raster_golden` update 0 (moved 12), check 101 (`issue19517`), update 0 (moved `issue19517` back; diff
the 11), check 0 (held 974). Over `-p pdf-model -p conformance`: fmt 0; clippy `-D warnings --all-targets` 0;
`pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 100 (3345 passed, 4 failed: `records`, this record's gates
paragraph; `wait`'s self-test, `thumbnails` and `matte_reduced` under load, all three passing alone in 12 s);
rustdoc `-D warnings --document-private-items` 0; `cargo test -p conformance` 101 (`records`), `--test records` 0,
then the whole suite 101 at a load of 189: only `wait`'s self-test, "a start and a wait in one call took one wall".
