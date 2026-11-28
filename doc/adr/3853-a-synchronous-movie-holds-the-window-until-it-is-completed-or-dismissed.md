# 3853 — A synchronous movie holds the window until it is completed or dismissed

Session 2288. Status: **accepted and built**. Code: `pdf-model`'s `media/deprecated.rs` (`told`)
and `media/playing.rs` (`Playing::synchronous`); `viewer-host`'s `video.rs`
(`Shown::synchronous`, `VideoPlayer::holds_the_reader`, `HELD`, `HOLDING`), `video/window.rs`
(`Pictures::holds_the_reader`) and `native_window.rs` (`WindowState::held`); the key and pointer
paths of `viewer-qt`, `viewer-gtk` and `viewer-ui`. Clauses: ISO 32000-2 §13.4, Table 307. Plan:
`doc/todo/80` section 2 step 3.

## Context

Table 307's `/Synchronous`: "If true, the movie player shall retain control until the movie is
completed or dismissed by the user." It was said as set aside and not honoured. The sound player
already holds the window for Table 212's synchronous sound (`Heard::Holding`, ADR 3343); a movie
without sound has no sound to hold it.

## Decision

1. **The flag is `Playing::synchronous`**, the field Table 212's flag already has, so it crosses
   the confined protocol with no new wire. A value that is no boolean is said.
2. **The picture player holds while the movie's clock runs**: `VideoPlayer::holds_the_reader` is
   true for a synchronous clip in hand and running. It lets go when the last pass ends
   ("completed") or Escape stops it ("dismissed by the user"). A clip paused by a rendition action
   or held at its start holds nothing, as a paused synchronous sound does not: the hold is the
   player's "control", and a paused player is exercising none.
3. **The window asks one question.** `WindowState::held` (Qt and GTK) and `App::held` (the winit
   window) answer the sentence to say for a refused key or click — the movie's (`video::HELD`)
   where the pictures hold, the sound's otherwise — and every place that asked
   `sounds.holds_the_reader()` asks it instead. Escape is taken before the question, so it always
   dismisses. A movie with sound is held by both players, and either answer refuses the same keys.
4. **The title says it** (`video::HOLDING`), as it says a synchronous sound.

## Tests

`video/tests.rs`: `a_synchronous_movie_holds_the_reader_until_it_is_completed` (holds while
showing and while its last picture is up, lets go at the end; an asynchronous clip never holds) and
`a_synchronous_movie_dismissed_or_paused_lets_the_reader_go`;
`deprecated.rs::a_synchronous_movie_tells_the_player_to_hold` (the action's `false` overrides, a
non-boolean is said).
