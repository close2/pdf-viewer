# 2295 — A rich media view and animation are read, and the rows take their states

Lane `gwg`: `doc/todo/80` section 1 steps 4 and 5, then the rich media rows. ADRs 3881 to 3883;
Q1116.

- **Views** (ADR 3881): `pdf_model::media::rich_media::invoked_view` reads Table 335's `/View`, else
  the first of Table 341's `/Views` (a direct view or one not in `/Views` said and used). For a
  `Sound` or `Video` clip, Table 315's scene entries are said as setting nothing; Table 344's
  `/Params` are read past by name (§13.7.2.3.6's state data is the run time's, stated only as ISO
  21757-1's `StateEvent`); `/Snapshot` is checked to be an image XObject and said as **not
  displayed**. A refused (`3D`) activation says its view too (`viewer-core` `pressed`).
- **Why the snapshot is not drawn**: the page would show it from activation to deactivation, and
  Table 336's `XD` is the reader's stop of a player, which no host tells the core of. Drawing it
  without that leaves it on the page after the clip stops. The construction is in ADR 3881 and
  `doc/todo/80` step 4: a players-to-core command (as `Command::PlayingIn`), then a `ViewState`
  set read by `annotation::decide`.
- **Animation** (ADR 3882): Table 337 drives "keyframe animations present in this artwork", and
  Table 313's time line is the 3D artwork's keyframes; a clip has none. `animation` reads it whole
  (defaults, the `None` rule, an unlisted style as `None`) and a `Linear`/`Oscillating` one is set
  aside by name in `Clip::unhonoured`. Q1116 asks whether to map it onto a clip as a choice.
- **Tests**: two `views.rs` units, one `rich_media.rs` unit, one in
  `viewer-core/tests/rich_media_activation.rs`.
- **Ledger** (ADR 3883): `implemented` 13.7.1, 13.7.2.2, 13.7.2.2.1 to .3, .5, .6, 13.7.2.3.3, .3.4
  (from `out-of-scope`); 13.7.2.3.5 `reported`; 13.7, 13.7.2, 13.7.2.3 `partial` (aggregates,
  placed in `doc/todo/65`); 13.7.2.2.4 and 13.7.2.3.6 to .6.3 `out-of-scope`, restated.
- Left, in order: the snapshot's display (players-to-core command, then the page set-aside); then
  13.7.2.3.5 `implemented` and the three aggregates with it.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-confined
-p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi -p conformance`: fmt 0, clippy `-D warnings
--all-targets` 0 (the first run 101 on one `doc_markdown` in a quote, fixed), `pdf-sandbox` and
`pdf-media-worker --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5356 of 5357 — the one
`viewer-gtk` `a_clip_plays_to_its_end_in_the_gtk_window`, its two-minute bound run out under four
lanes' load, passing alone in 2 s with the rich media tests, 18 of 18 (manifest: none expected);
rustdoc (two crates, private directory) 0, fuzz 0, conformance 0. Launch (`tools/state.sh --round 2295 launch`) 0:
36 figures banded, 0 outside, after an hour queued for the clock lanes behind two rounds' clock runs
and the train's long walk. Nothing this change adds runs before a press.
