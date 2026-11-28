# 2297 — A rich media snapshot stands while a player holds the clip

Lane `gwg`: `doc/todo/80` section 1 step 4's display, the last piece of section 1. ADR 3889.

- **The command** (ADR 3889): `Command::InHand { document, annotation, in_hand }`, sent by a player
  when it takes a clip whose `SoundData::rendition` names an annotation and when the clip leaves
  it (its end, Escape or the bar, a later clip, a rendition stop, a `PC`/`PI` stop, a refusal).
  `viewer_host::in_hand::Holding` owes it on each change: the sound player's thread says it beside
  each `Painted` (`repainted`), the picture player derives it at each look (`VideoPlayer::told`,
  `Looked::told`). Raised by GTK 4, Qt, the C ABI window (through `Looked::told` and
  `native_window`'s sound look) and winit (`show_the_pictures`, `take_the_sounds`); it crosses the
  confined protocol as `IN_HAND` 58. No host that plays is left out; a window that hands every clip
  on takes no rich media clip, so it invokes no view.
- **The page**: the core counts the players holding each annotation's clip (`Open::in_hand`) and
  marks `ViewState::show_snapshot`; `annotation::decide`'s `stored_set_aside` sets the normal
  appearance aside where the invoked view names an image, and `appearance::rich_media_snapshot`
  fits it as the poster is (`fitted_image`, shared with `movie_poster`), reported as a placement
  this program chose. The views.rs "not displayed" note is gone; the activation says the image is
  displayed while held, and a refused activation says it is not.
- **Tests**: `viewer-core/tests/rich_media_snapshot.rs` (before, during, after; two holders; a
  refused activation), `in_hand.rs` unit, `video/tests.rs` (end, Escape, unassociated),
  `rendition_operations.rs` (the sound player's held and let go), the protocol round trip, and
  one per host — GTK, Qt, C ABI, winit — over `showing::a_rich_media_document`, the page's report
  of the snapshot appearing while the clip plays and gone after its end (winit: and after Escape).
- **Ledger**: 13.7.2.3.5 `reported` → `implemented`; 13.7, 13.7.2, 13.7.2.3 `partial` →
  `implemented` (every leaf settled); `doc/todo/80`, `doc/todo/65`, `doc/state-of-play.md`.
- Left: nothing in section 1; Q1116 (animation onto a clip) stays open.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-confined
-p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi -p conformance`: fmt 0, clippy `-D warnings
--all-targets` 0 (earlier runs 101: two `doc_markdown` in quotes, a test `panic`,
`unnecessary_wraps`, an empty-assert message); `pdf-sandbox` and `pdf-media-worker --bins` 0;
nextest `--lib --bins --tests` (`xvfb-run -a`) 5375 of 5376, the one `viewer-gtk`
`a_clip_plays_to_its_end_in_the_gtk_window` (the media worker's 2.28 s answer bound run out under
load, as in 2295), passing alone with its five neighbours (manifest: none expected); rustdoc (four
crates, private directory) 0; fuzz 0; conformance 0. Launch (`tools/state.sh --round 2297 launch`)
0: 36 figures banded, 0 outside. Nothing this change adds runs before a clip is taken.
