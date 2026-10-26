# 1774 — A movie's file and a sound's file cross through the rendition's hand-off

Session 1774, a host round in batch 117; 2026-10-26. ADR 2382; ADR 2383 unused; no Q.

**Contract.** Slot 4: A631's hand-off — a movie's Table 306 `/F` handed over on a movie annotation's
activation and a movie action's Play through the hand-off a rendition's clip uses (ADR 1961),
Stop/Pause/Resume acting on nothing playing, then a sound with `/F`; nothing plays in-process.

**Premise.** Held: `Event::Media` and `viewer_host::handed_media` already carry any file, so the
hypothesis "one more source for the rendition's hand-off" held and no window, wire or C ABI changed
shape. Did not hold: the brief's §12.5.6.25 is the 3D/RichMedia pointer row; the movie annotation is
§12.5.6.17, which moved instead (and §12.5.6.16's sound annotation, whose activation now hands over).

**Built.** `pdf_model::media::{movie, sound}` (new `media/deprecated.rs`) read Table 189/306 and
Table 212/188 + §13.3's Table 5 `/F` into the `Clip` a viable rendition yields, either file-spec form;
`Clip::temporary_file` is an `Option`, `None` where no Table 286 stands, so the document asserts
nothing against `HandOverMedia`. `Action::Movie`/`Action::Sound` and two `Request` variants replace
the two refusals; `viewer_core::interact::deprecated_media` performs them (a `/T` looked for on the
chain's jump page or the page shown; Table 189 `/A false` withholds the click only; Table 307,
`/Rotate` and Table 212's parameters said as what the player is not told) and `exhibited` answers a
click (after `/U`) and `Command::Activate`. The windows' sentence cites §13.3 and §13.4 too. The drive
gains `89-movie-click` in all four lists.

**Rows.** §12.5.6.17, §12.6.4.9, §12.6.4.10 `out-of-scope` → `implemented`; §13.3 `out-of-scope` →
`partial` (embedded samples said, the RIFF wrapper owed — bucket 6 of `doc/todo/65`); §13.4 and
§12.5.6.16 stay `implemented`, notes and code widened.

**Owed.** The RIFF wrapper for embedded samples and the rich-media `/Assets` walk — ADR 2382 says what
each needs. Round 1771's two `media.rs` defects (an image clip's `/B`/`/E`, a non-viable `/MU` entry)
are a §13.2.7.2.1 reading of their own and were not taken: batch 118's.

**Gates.** `cargo nextest run -p pdf-model -p viewer-core`: exit 0, 2636 passed (the new
`movies_and_sounds.rs` 7, `deprecated.rs` 4); `-p viewer-host`: exit 0, 302 passed. `RUSTFLAGS="-D
warnings" cargo clippy --all-targets` on `pdf-model` and `viewer-core`: exit 0; on `viewer-host`:
exit 0. `cargo fmt --check` on the three: exit 0. `cargo test --profile gates -p pdf-model --test
actions -- --ignored --nocapture`: exit 0, 2 passed. A planted `/A false` ignored fails
`a_movie_annotation_whose_activation_is_false_hands_nothing_over` (trap 13). The drive under Xvfb on
my export's release binaries, `rendition_click` and `movie_click` in all four windows: 8 works, 0
wrong. `cargo test -p conformance --no-fail-fast`: exit 101, 465 passed, 1 red, 1775's record. `t2-launch_path` not run: no
file the launch path executes changed — the new arms run on a click or a performed action only.
