# 2009 — A frame's cost carries the stream each byte crossed in, and the window's trace says what each frame sent

Session 1586, a "pixels" round of batch ninety-three, for the project owner's ask of 2026-10-19
that a zoom "send only a matrix": the instrument that ask is judged by, carried from `zoom_frame`
to the window. Status: **accepted** and **built** — `raster_gpu::UploadSplit::saturating_add`
(`raster/crates/raster-gpu/src/frame.rs`), `render_raster::FrameCost::streams`
(`crates/render-raster/src/present.rs`), the frame line's `sent` column
(`crates/viewer-ui/src/bin/quorra/timing.rs`), `zoom_frame` reading the same field. Context: ADRs
0227, 0387, 1992, 1993, 2008. Cites no clause and moves no ledger row; no pixel moves.

## 1. What was missing

ADR 1992 divided raster's `bytes_uploaded` by stream (`Counters::uploads`) and printed it from
`zoom_frame` through `QuorraRasterizer::last_counters`; the window, which draws through
`QuorraWindowRenderer` and reads only `FrameCost`, still had a total with no stream. Round 1578
stopped there because the eighty bytes the split adds took `FrameCost` from 192 to 272 bytes, past
clippy's `large_types_passed_by_value` bound of 256 at the sites that pass it by value.

## 2. Decision

**`FrameCost` carries the split by value, as `streams`, and stays `Copy`; the sites that passed it
by value take a reference.** The lint fires only on a `Copy` type passed by value, and its remedy
is the reference: of the whole workspace, two sites tripped it — `zoom_frame`'s `Sample::of` and
the window's `said_idle_work`, each now `&FrameCost` — and every other reader already borrowed it
or held it in a field. A `Box<UploadSplit>` was the other answer and is refused: it would take
`Copy` from a type the window keeps beside its next frame (`renderer::window`'s `scene_news`,
`Stages::gpu`) and copies across threads, and it would allocate on every
`FrameCost::default()`, which the render thread makes per frame and per lane, to silence a lint
about a 272-byte copy.

`FrameCost::read` sets `streams` from `Counters::uploads`, and `FrameCost::add` adds it share by
share (`UploadSplit::saturating_add`), so a call that drew the page and the chrome reports their
streams' sum, as it reports their `bytes_uploaded`.

**The window's frame line ends in `| sent …`**: each stream that sent bytes, under the names
`zoom_frame`'s `crossed:` line uses (`line`, `quad`, `rect`, `sheet`, `atlas`, `records`, `arena`,
`winding`, `paint`), and the compute lane's device-only allocations after a semicolon, apart,
because the budget counts them and no host write fills them. A frame that sent nothing has no
column, so a replayed or reprojected frame's line is as short as it was; the legend names the
column once.

## 3. What it shows

Driven under Xvfb on lavapipe, the export's own binary, two documents: the EC3 specification's first
spread `| sent quad 76096, rect 1216, atlas 223232, paint 226944` (encoded) and the spread after a
turn `| sent line 180, quad 144640, rect 1952, atlas 126976, paint 176400`; `AdobeBPC-2006.pdf`'s
first page `| sent quad 19840, rect 544, atlas 251904`; every frame that adopted nothing printed no
column. The witness's step in the shipped window is the figure the owner's display will show, and
`zoom_frame`'s window mode prints the same shares headless (ADR 2008 §1).

Tests: `raster-gpu`'s `frame::tests::two_frames_streams_add_share_by_share`, `viewer-ui`'s
`timing::tests::a_frame_that_sent_nothing_has_no_sent_column` and
`the_sent_column_names_the_streams_that_sent_and_the_device_only_share_apart`.
