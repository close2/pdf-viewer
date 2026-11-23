# 3433 — A rendition is stopped, paused and resumed where it plays

Session 2183. Status: accepted. Supersedes ADR 3418's decision. Context: ADRs 2988, 3342, 3349,
3350, 3407, 3417; ISO 32000-2 §12.6.4.14, Table 218.

## Decision

1. **The association lives in the player.** Table 218's `/OP` 1, 2 and 3 act on "any rendition
   being played in association with the annotation specified by AN", and only the window knows
   whether a sound still plays, so the core does not keep it. Every sound a rendition action plays
   carries `viewer_core::Associated` (`/AN`, and whether `/OP` was 4) in `SoundData::rendition`,
   whatever its window type, across the confined wire too; the player's thread keeps it on the
   sound in hand.
2. **Stop, pause and resume become `Event::OperateMedia`**, after §12.6.4.14's precondition, which
   binds "a rendition action" whatever its operation, and in the chain's order with the plays
   (`interact::MediaRequest`). The window hands it to `Player::operate`; the thread acts where the
   sound in hand is the document's and the annotation's, and answers `Heard::Unchanged` — the
   clause's "there is no effect" — where none is, or where a pause finds it paused or a resume
   finds it sounding. A stop says the association is removed.
3. **A pause keeps the device and gives up the stream.** The sink is stopped (the helper drops its
   `cpal` stream and keeps the device it opened at the first sound, as between sounds); the frames
   the device counted of the pass, or the part of a freeze that lasted, are folded into the media
   clock (`MediaClock::pause`), the pass's remainder kept, and a resume starts the rest of the
   samples from there on a new stream. Keeping the device costs a resume no probe of the sound
   server; dropping the stream leaves a paused sound holding nothing another program shares.
   Table 294's `/W` 3 painting stays while paused: the rendition is still associated.
4. **`/OP` 4 is decided by the player.** `RenditionOperation::PlayOrResume` is parted from `Play`;
   the core judges and hands over its clip as for 0, and the thread, where the annotation's
   rendition is in hand, resumes it if paused (held at its start counts) and otherwise does
   nothing, and plays the clip only where none is associated. `/OP` 0 stops the one in hand first,
   as every play does.
5. **The painting is told, not inferred.** The thread sends the screen it paints with each change,
   in place of the window following the order of first answers, which a resume would have broken.
6. **Where the clip was handed on** (no helper, ADR 3417; a clip judged for this machine's player;
   the C ABI's host), nothing here plays it: the GTK, Qt and `quorra` windows and `quorra-confined`
   say the operation had no effect and that a handed-on clip is beyond this program's reach. The
   C ABI gains `QUORRA_EVENT_OPERATE_MEDIA` (32): its host handed the clip to the platform's
   player, and may do the operation there.
7. **`/JS` beside `/OP` is unchanged** (ADR 2988): a runner supplied runs the script and no
   operation is sent; with none, `/OP` is the fallback.

## Cost

One event kind (core, confined wire, C ABI), one field on `SoundData`, one `Heard` variant, a
pause state and offset in the player's thread.
