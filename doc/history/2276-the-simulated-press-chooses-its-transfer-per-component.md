# Round 2276 — the simulated press chooses its transfer per component, by name and for images

An audit-lane round in the `pdf-viewer-audit` worktree, taking the four readings round 2267 left
on §10.8.3's simulated press, and the llvmpipe and OpenGL band rows round 2250 left stale.

## The readings

- **A group of one or three components, ADR 3805.** §10.5 puts the functions in "the native colour
  space of the output device", and §11.7.4.2 converts a group's result "to the device colour space"
  first, so the press's functions apply to such a page too. ADR 1329 gives it no stop in the
  press's colourants, so it keeps the screen's application, recorded as a departure; every lane
  agrees (CPU and `render_raster::finish` map the finished pixel, `render-gpu` refuses by name).
- **Overprint per component, ADR 3806 (supersedes 3771).** §11.7.5.2: a component the mark keeps
  takes the default. `pdf_render::OpaqueFor` rides on a run; `pdf_model` reads it off
  `DeviceSpots::overprint` for every plane, one slot per stroking and non-stroking decision.
- **A `DeviceGray` image, ADR 3807.** An image's or stencil's samples in the black-ink space take
  `Transfer::grey_alone` (`Interpreter::image_transfer`).
- **A Type 5 entry by spot name, ADR 3808.** Every Type 5 key is read; a spot colourant takes its
  own entry, else `/Default`'s. Erratum #311 deletes the "gray … transfer function" sentence for
  nonprimary colourants, so `/TR`'s array of four still sets none for a spot.

Tests: four in `render-raster`'s `press_transfers`, on CPU, `rasterize`, the window (RX 580), `render-gpu` refusing.

## Ledger

10.5, 10.6.5.6 and 11.7.5.2 `implemented` before and after; their notes say what is now built and
name ADR 3805's departure, and the four tests are added.

**Gates.** Over `-p pdf-colour -p pdf-model -p pdf-render -p render-cpu -p render-raster -p raster-gpu
-p render-gpu -p viewer-core -p test-scenes`, rebased on 24337bfe6: fmt, clippy, `pdf-sandbox`, nextest,
rustdoc, fuzz and conformance 0; `raster_golden` re-held `issue6931_reduced.pdf` p1 (list only),
strips 0. `draw_mark` now returns before per-component work on a page stating no transfer (opt_demo
`frame_kinstructions` read 3212.6 against 3118..3183 before it): launch 0 outside (3177.4), turn 0
outside with 30 judged (`personwithdog.pdf` p1 68.99 ms against main's 69.08 ms the same hour).

**Bands.** The llvmpipe and OpenGL rows for `personwithdog.pdf` and the Type 3-in-Type 3 page are
not re-taken (queued two hours without a reading); owed from this tree's release build.

**Habit.** A slot written at a decision and read at the mark is per kind: a `B` decides fill and
stroke before painting either, and one slot gave the fill the stroke's answer.
