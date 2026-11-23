# Round 2179 — a sound handed on without its helper, and four leftovers

Clause 13 sound's last item and four leftovers of rounds 2167, 2169 and 2176: ADRs 3417–3420.

## What it did

- **No helper (ADR 3417).** GTK, Qt and `quorra` ask `Hearing::of_a_playing_window` at the launch
  (`helper::missing`, one look for `quorra-sound`, nothing started) and tell the core
  `PlaysHere` from it, so a window without its helper has every clip judged for this machine's
  player. A sound is then handed on at the reader's level (`handing_without_helper`), the
  missing helper named on the terminal and in the status line or question, and `HANDED_ON` in the
  title until the next sound. `Launch::start` takes the answer; `viewer-ffi` now says
  `Hearing::HandsOn` and is no longer told `PlaysHere(true)`, which ADR 3408 said it was not.
  `PortalCalls::calling` lets a whole window's portal call go to a double.
- **Stale sentence (ADR 3418).** `rendered` and its notes say what is: stop, pause and resume are
  not performed, since a played sound keeps no association with `/AN`. §12.6.4.14's row said none
  ever plays, which ADR 3350 made false; it is `partial`, naming the three operations.
- **Outline test (ADR 3420).** `Pages::searches` counts walks (one relaxed increment per
  `index_of` or `indices`); the test asserts one search for 988 items. A planted walk per page
  counted 1 033 and failed; reversed.
- **String operands (ADR 3419).** `Tj`, `'`, `"` with a non-string `string` operand report
  `StreamSyntaxFault::TextOperandNotAString`; `'`'s line move is still made.
- **Fuzz crate.** `clippy --bins -D warnings` failed in `cms`, `embed`, `fragment`, `saml`,
  `vfs_write` (five lint forms of clippy 1.100); fixed. `fmt --check` was already clean on
  `media_sound.rs` (round 2176 formatted it).

**Left, in order.** Performing §12.6.4.14's stop, pause and resume on a sound a window plays (the
row's owed half); a macOS check of the sound path; a `quorra` window test of the hand-on (its
portal has no seam).

**Ledger.** §12.6.4.14 `implemented` → `partial`, and §12.6 and §12.6.4 above it aggregates;
`doc/todo/65` places all three. §9.4.3 stays `implemented`, its note and a test added.

**Gates.** One `-p` set, `viewer-core viewer-host viewer-qt viewer-gtk viewer-ui pdf-model
viewer-ffi`: fmt exit 0; clippy `-D warnings --all-targets` exit 0 after two fixes of this round's
own code; `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a`: 4 369
run, 4 360 passed, 9 failed, the manifest's 9 exactly; rustdoc `-D warnings
--document-private-items` of seven crates exit 0; `fuzz` clippy `--bins -D warnings` exit 0, fmt
`--check` exit 0, `check --bins` exit 0; `cargo test -p conformance` exit 0 after the frontier map
placed the three open rows.
