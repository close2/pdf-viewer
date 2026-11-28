# 2288 — A synchronous movie holds the window, and rich media plays on its conditions

Lane `gwg`: `doc/todo/80` section 2 step 3, then section 1 step 1. ADRs 3853, 3854, 3855; no
question.

- **`/Synchronous`** (ADR 3853): Table 307's flag is `Playing::synchronous` (the field Table 212's
  already crossed the protocol in). `VideoPlayer::holds_the_reader` is true while a synchronous
  clip's clock runs, false once it ends, is stopped (Escape, the dismissal) or is paused.
  `WindowState::held` (Qt, GTK), `App::held` (winit) and `viewer-ffi`'s window refuse every key,
  click and wheel but Escape, saying `video::HELD` or the sound's `HELD`; the titles say it. 13.4's last `shall`.
- **Rich media activation** (ADR 3854): `pdf_model::media::rich_media::settings` reads Table
  335's `XA`/`PO`/`PV` and Table 336's `XD`/`PC`/`PI` (an unlisted name said, default kept).
  `interact::trigger`, which Table 197's four page moments reach for every annotation, asks
  `interact::rich_media::raised`: `PO`/`PV` play as `/OP` 4 associates (nothing twice), `PC`/`PI`
  ask `/OP` 1's stop. Not before `Command::Presented`: page one pays nothing, and `presented`
  raises the shown pages' `PO`/`PV` then (the launch miss below).
- **The configuration** (ADR 3855): `/Configuration` or the first; its kind from Table 342's
  `/Subtype` or the first instance's; the first instance of that kind; the asset's Table 44
  `/Subtype` decides (a documented choice). `Sound` (WAVE, MP4 sound) and `Video` (MP4 `video/`,
  in the rectangle) play; `3D`, unknown kinds, no instance, non-embedded or untyped assets are
  refused by name and the press keeps ADR 3819's sentence and ADR 2399's files. `/Scripts`
  (ISO 21757-1), `/Animation`, `/View`, `/Presentation` said as set aside. A press that plays
  hands no file out.
- **Tests**: the hold (two, `video/tests.rs`) and the flag (`deprecated.rs`); three
  `rich_media.rs` units; four in `viewer-core/tests/rich_media_activation.rs`. Ledger: 13.4
  `partial` → `implemented`; 13.7.2.1 and six `out-of-scope` rich media rows restated.
- Left, in order: `doc/todo/80` section 1 steps 2 to 5 (presentation, instance-to-asset
  matching, views and snapshots, animation); then the rich media rows' status.

**Gates.** Rebased onto main c8770fe7c; over `-p pdf-model -p pdf-media-worker -p viewer-core -p
viewer-host -p viewer-confined -p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi -p
conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox`/`pdf-media-worker
--bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5334 of 5335, the one viewer-gtk
`a_clip_plays_to_its_end_in_the_gtk_window` on a 2.28 s worker timeout under load, passing alone
(manifest: none expected); rustdoc (seven crates, private directory) 0, fuzz 0, conformance
0. Launch: page moments read at the open put opt_demo's `frame_kinstructions` at
3187.942 / 3187.917 against HEAD's 3181.3 (band ..3183); deferred to `Presented`, 3178.1 on two runs, inside;
the first run's 101 was xfa_filled's `peak_anon_mib` 3.047 of ..3.0, the second 0 with none outside.
