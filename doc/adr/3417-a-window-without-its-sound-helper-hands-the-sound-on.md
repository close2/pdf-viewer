# 3417 — A window without its sound helper hands the sound to this machine's player

Session 2179. Status: accepted. Context: ADRs 3201, 3217, 3317, 3350, 3408; ISO 32000-2 §13.2.2,
§13.2.4.2.

## Decision

1. **The question is asked once, at the launch.** GTK's, Qt's and `quorra`'s windows ask
   `viewer_host::sound::Hearing::of_a_playing_window` — `helper::missing`, the same look for
   `quorra-sound` that the first sound makes (`QUORRA_SOUND`, then beside the executable) — and
   tell the core `Command::PlaysHere` from the answer. A window whose helper is missing is told
   it plays nothing here, so §13.2.2's viability is judged for this machine's player, as ADR
   3350 has it for every clip a window does not play: a clip judged for the helper's player and
   then given to another would have been judged for a player it never reached.
2. **The C ABI's window says `Hearing::HandsOn`.** `Launch::start` now takes the answer, and
   `viewer-ffi` was being told `PlaysHere(true)` by the launch it shares with GTK and Qt, which
   ADR 3408 decision 2 says it is not; it now judges every clip for the hand-off.
3. **A sound is handed on at the reader's level and said.** `viewer_host::handing_without_helper`
   is `handing`'s answer with the sentence naming the missing helper beside a hand-over and in
   the question before one; a refusal stays the refusal. The window says it on the terminal and
   in the status line, and the title carries `HANDED_ON` until the next sound. Only a clip that
   is a sound (`is_a_sound`: samples, an `audio/` type or a WAVE type) carries the sentence: a
   movie is handed on in any window and saying a helper was wanted for it would be false.

## Cost

One look for a file on the launch path — `current_exe` and at most a handful of `stat`s, no
process started; on GTK and Qt it runs on the launch's own thread, in `quorra` before the
document is opened. The helper itself is still started only by the first sound, as `CLAUDE.md`'s
"nothing eager" asks. A helper that appears after the launch is not used until the next launch,
and one that vanishes after it is the refusal ADR 3317 already says. `PortalCalls::calling` is a
test seam: a whole window's portal call made through a double.
