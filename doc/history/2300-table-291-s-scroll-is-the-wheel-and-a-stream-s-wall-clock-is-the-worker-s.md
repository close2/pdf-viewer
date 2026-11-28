# 2300 — Table 291's scroll is the wheel, and a stream's wall clock is the worker's

Lane `gwg`: one of the last two open ledger rows, §13.2.5's `/F 3`, and the flaky GTK playing test
of rounds 2295 and 2297. ADRs 3901, 3902.

- **The interface** (ADR 3901): over the rectangle of an `/F 3` picture that overflows it, the
  wheel moves the part shown, one sample a pixel, held at the picture's edges and taken there
  too; no overflow, no interface. One geometry, `viewer_host::video::placed_scrolled`
  (`video/scroll.rs`), and `VideoPlayer::scrolls` and `scroll_over`. It is raised in `quorra`
  (the sidebar's wheel, and its media window), GTK 4 (`scrolled` from the motion controller's
  last point, and its media window), Qt (`Host::pointer_at`, new), and the C ABI's new
  `quorra_window_wheel` (header, interop, and the WinUI host's wheel, which could not be built
  here).
- **The judgement**: `unscrolled` and `Honoured::fit_must` are gone. The requirement is
  conditional and every window that shows pictures meets it whatever the picture's size, so
  `/F 3` is honoured in `MH` and `BE` alike, decided before decoding.
- **The flaky test**: the 2.28 s is the product's own bound. `client::video::wall` is one second
  plus the run's 1.28 s of media, and `Budget::wall` counted from the open request to `END`. But
  the window drains the decode by its presentation clock, and credit is granted only at the
  host's next read. So most of the budget was the host's pacing, and a late window under load
  spent the rest. Reproduced without load: a caller holding forty 40 ms pictures for 80 ms each
  failed `Stalled { after: 2.28s }`. Fixed in the product, not the test (ADR 3902): the stream's
  watch stops while the host holds an item. The test bound is unchanged and nothing is retried.
- **Tests**: `video/tests.rs` (the part at four scroll positions and its affine, a turned picture,
  the wheel taken and refused), one per host over `showing::a_scrolling_video_document`,
  `media_viability.rs` (3 in the loop), `stream::tests::the_wall_clock_stops_while_the_host_holds_an_item`,
  `tests/video.rs::a_caller_that_holds_its_pictures_does_not_spend_the_worker_s_wall_clock`.
- **Ledger**: §13.2.5 `partial` → `implemented`; §13.2 `partial` → `implemented`; `doc/todo/65`
  and `doc/state-of-play.md` (where "no window draws them yet" was stale).

**Gates.** Over `-p pdf-model -p pdf-media-worker -p confined-transport -p stream-witness -p
viewer-core -p viewer-host -p viewer-confined -p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi
-p raster-gpu -p conformance`: fmt 0; clippy `-D warnings --all-targets` 0 (earlier 101:
`many_single_char_names`, `arithmetic_side_effects`, `manual_midpoint`); `pdf-sandbox` and
`pdf-media-worker --bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 6223 of 6227. The
four were this round's own, fixed and re-run 0: the two entry-point counts, the C# interop's
declaration, and the `quorra` fixture's rectangle, exactly 16 px at its magnification. Manifest:
none expected. Rustdoc (eight crates, private directory) 0; fuzz `cargo check --bins` 0;
`cargo test -p conformance` 0 (its first run 101 on this record's gates line alone). Launch
(`tools/state.sh --round 2300 launch`) 0: 36 figures banded, 0 outside.
