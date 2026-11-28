# 3889 — A rich media snapshot stands while a player holds the clip

Session 2297. Status: **accepted and built**. Code: `viewer-core`'s `Command::InHand`,
`interact/rich_media.rs` (`in_hand`, `played`), `open.rs` (`Open::in_hand`); `pdf-model`'s
`view.rs` (`ViewState::show_snapshot`, `AnnotationView::snapshot`), `annotation.rs`
(`stored_set_aside`), `appearance.rs` (`rich_media_snapshot`, `fitted_image`); `viewer-host`'s
`in_hand.rs` (`Holding`), `sound.rs` (`Player::repainted`), `video.rs` (`VideoPlayer::told`),
`video/window.rs` (`Looked::told`); the four windows' picture looks; `viewer-confined`'s
`IN_HAND`. Clauses: ISO 32000-2 §13.7.2.3.5, Tables 333, 336, 344. Builds ADR 3881 decision 5.

## Context

Table 344's `/Snapshot` is "an image XObject ... to be displayed when the view is invoked", and
"If the Snapshot entry exists, the image it describes shall be displayed in place of the current
artwork". The view is invoked at activation, and the image stands until deactivation. The core
sees activation (a press, `PO`, `PV`) and asks the stops `PC` and `PI` call for, but Table 336's
`XD` — the reader's stop — and a clip's end happen inside the window's players. ADR 3881 kept the
image undrawn until the players could say so.

## Decision

1. **The players say both ends.** `Command::InHand { document, annotation, in_hand }` is sent when
   a player takes a clip whose `SoundData::rendition` names an annotation, and again when the clip
   leaves it: its end, Escape or the controller bar, a later clip in its place, a rendition
   action's stop, the stop a `PC` or `PI` asks for, a refusal, the document's close. Saying the
   start as well as the end, as `PlayingIn` does (ADR 3407), means the image is drawn only once a
   player truly holds the clip: a clip refused before it is held never leaves an image behind.
2. **One tracker for both players.** `viewer_host::in_hand::Holding` owes a command at each change
   of the annotation in hand. The sound player's thread says it with each `Painted` (taken in by
   `repainted`, which every playing window already dispatches); the picture player derives it from
   what is in hand at each look (`VideoPlayer::told`, carried in `Looked::told` to GTK 4, Qt and
   the C ABI window, and dispatched by winit's `show_the_pictures`). Every stop path of every
   window already ends in a look, so no stop goes unsaid.
3. **The core counts holders.** A clip with sound and pictures is in two players' hands, and the
   sound may end before the pictures; `Open::in_hand` counts per annotation, and the image goes
   only when no player holds the clip.
4. **The page draws it.** `ViewState::show_snapshot` marks the annotation, read as
   `AnnotationView::snapshot`; `annotation::decide` sets the stored normal appearance aside (the
   third case of `stored_set_aside`, beside retyped text and scripted widgets) where the invoked
   view names an image, and `appearance::rich_media_snapshot` fits it into `/Rect` as Table 306's
   poster is (`fitted_image`, ADR 1561), reported as a placement this program chose.
5. **The colour-space permission is not taken**: "the colour space of the image may be modified"
   — the image is painted as its own colour space states.
6. **A refused activation shows no image and says so**: no view is invoked while the annotation
   stays inactive. A window that hands every clip to this machine's player takes no rich media
   clip (`activated` refuses it), so no window that cannot say `InHand` ever invokes a view.

## Consequences

The 13.7.2.3.5 row is `implemented`; ADR 3881's "said as not displayed" note is gone. A host that
never sends `InHand` draws no snapshot, which is the safe direction: the appearance Table 333
gives "for the initial display of the annotation" stays.
