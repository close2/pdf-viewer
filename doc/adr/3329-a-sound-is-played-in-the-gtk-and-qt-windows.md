# 3329 — A sound is played in the GTK and Qt windows, through the same helper

Session 2157. Status: accepted. Context: RFC 0015 section 5 (steps 3 and 4), ADRs 3201, 3297, 3301,
3317, 2070, 3098, `doc/questions/A946`.

## Decision

1. **The player is the shared window's.** `viewer_host::native_window::WindowState` holds
   `sounds: viewer_host::sound::Sounds` — no player, thread, worker or helper until the first
   sound — around ADR 3301's `Player` with ADR 3317's `helper::open` as its `OpenSink`. What the
   window keeps beside the player is `Sounds`' own: the sentence that holds the title (a sound
   sounding, with "Escape stops it", or one refused), whether Escape has a sound to stop, and how
   long the timer waits before it looks again. `quorra` keeps its own fields for the same three; it
   is not a `NativeWindow`.
2. **A window says whether it plays sounds.** `NativeWindow::plays_sounds` is `false` by default and
   `true` in GTK and Qt. Where it is true, `Event::Media`'s samples go to `handing_here` and a
   `Handed::Here` — at once, on a `warn`, or on an `ask`'s yes, which now carries the document's id —
   to `play_here`; where it is false the sound's file is handed over as any clip, which keeps the C
   ABI's window as it was until step 5.
3. **The wake is the toolkit's; the timer is the guarantee.** `answers_interval` holds
   `sound::LOOK` (250 ms) while a sound is decoded or sounds, so the timer that looks for a
   server's answer looks at the player too, and `take_the_answers` takes what it said: each
   sentence through `say` (status line and terminal) and the title rebuilt. GTK's
   `NativeWindow::sound_wake` invokes GTK's main context from the player's thread, which looks at
   every window that has played a sound (`viewer-gtk`'s `host/sounds.rs`, `drawing`'s `WOKEN`
   construction). Qt has no wake — the trait's default, which wakes nothing — for `device_lane`'s
   reason: a call from a thread into `QApplication::exec` would cost `viewer-qt` a second
   hand-written `unsafe` token. Its `drawing_wait` timer is re-asked after every update, so it is
   armed from the activation on.
4. **Lifecycle.** Escape stops a sound that sounds before the key table sees the key (GTK's
   `Host::key`; Qt's, which the find bar's close action hands Escape to); a second sound stops the
   first (the player's own rule); a document's close is `Sounds::closed`, which stops its sound and
   leaves another document's; the last document's close and GTK's `close-request` are
   `Sounds::end`, which drops the player, closing the helper's pipe and killing the worker before
   the window goes.
5. **Qt plays through the helper, not `Qt6::Multimedia`.** RFC 0015 section 5 named `QAudioSink`;
   ADR 3317's argument — a window maps no audio library at start and starts on a machine without
   one — holds for Qt exactly, and the helper is already the sink every window takes.

## Proof

`viewer-gtk`'s `host/played_here.rs` (under `#[gtk4::test]` and Xvfb, ADR 3098) and `viewer-qt`'s
`host/tests/played_here.rs` (the host's callbacks without a `QApplication`, the timer turned as
`pumpDrawing` arms it) each activate a sound annotation at the level `open` with a recording sink
(`tools/conformance/test-support/sounding.rs`): the device opened once, at the sound, and handed
`+½, −½`; the title naming Escape; Escape stopping it; and a missing helper said in the status line
and the title. Qt's also proves the timer stops when nothing is out and the last document's close
ends the player; GTK's that the window's close does. `tests/no_audio_library.rs` in each crate
reads `readelf -d` of `quorra-gtk` and `quorra-qt` and fails if `libasound`, `libpulse` or
`libjack` is `NEEDED`. **No person has heard a sound through either window**: this machine has no
sound card.

## Cost

A module of about 180 lines in `viewer-host`, a field and three provided methods on the shared
window, about 70 lines in `viewer-gtk` and 20 in `viewer-qt`. Nothing at launch; a GTK timer at
`submit::LOOK` while a sound is out, one channel drain per look.

## What step 5 (the C ABI and WinUI) needs

One decision and a line. `viewer-ffi`'s `Window::interval` already folds in `answers_interval`, so a
host program's own timer looks at the player with no wake; `say` is already owed as a record. The
decision is whose sink a sound reaches: `plays_sounds` returning `true` plays it through
`quorra-sound` found beside the host program's executable (`QUORRA_SOUND` otherwise), which on
Windows opens WASAPI through `cpal` and has to be installed beside the WinUI shell; or the window
owes the converted samples to the host as a record, as it owes a clip today (ADR 3217), and WinUI
plays them through its own audio graph. The first keeps one sink for every window and needs no
new record; the second keeps the host program the platform's player, as ADR 3217 chose for clips.
