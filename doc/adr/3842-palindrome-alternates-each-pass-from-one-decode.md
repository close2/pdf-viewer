# 3842 — `Palindrome` alternates each pass's direction, from one decode of the file

Session 2285. Status: **accepted**. Code: `pdf-model`'s `media/playing.rs` (`Playing::palindrome`)
and `media/deprecated.rs` (`told`); `viewer-confined`'s `protocol/event.rs`; `viewer-host`'s
`video.rs` (`Shown::palindrome`, `InHand::backward`) and `sound/reversed.rs` (`Direction::pass_from`).
Clauses: ISO 32000-2 §13.4, Table 307. Plan: ADR 3818.

## Context

Table 307's `/Mode`: "Palindrome Play continuously forward and backward until stopped." Until now
the movie repeated forward and the alternation was said as not built.

## Decision

1. **A repeat count of zero, and every second pass turned.** `told` sets `Playing::repeat` to zero
   and `Playing::palindrome`, which crosses the confined protocol beside `backward`. Pass `n`
   (from one) runs backward where `backward` differs from "`palindrome` and `n` even", so the
   first pass runs in `/Rate`'s direction and each later one against the one before it. **That a
   negative `/Rate` makes the first pass the backward one is this program's reading**: the table
   says "forward and backward" and, of `/Rate`, "played backward with respect to Start and
   Duration"; nothing in either orders the two when both are stated, and the reading keeps
   `/Rate` meaning what it means under every other mode.
2. **The pictures** decode each pass from the bytes already in hand (`Arc<[u8]>`), forward or by
   groups backward (ADR 3837), as a repeated pass already did; the file is not opened again.
3. **The sound is decoded once.** The sink holds the section in the first pass's direction and
   then in the other (ADR 3841's `Turning`), and every second pass starts at the second copy, the
   pass's own frames and clock counted from its own start as before.

## Tests

`played_video.rs::a_palindrome_alternates_its_direction_pass_by_pass` (three passes of six
pictures: rising, falling, rising); `played_sounds.rs::a_palindrome_s_sound_alternates_pass_by_pass`
(three passes forward, backward, forward, and with a negative `/Rate` the reverse);
`deprecated.rs::open_holds_and_palindrome_alternates`; the protocol round trip. Planting the turn
away fails the picture and sound tests.
