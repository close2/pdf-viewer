# 3682 — A carried stream leaves its source's crypt filter behind, and an undecrypted one is refused

Session 2245. Status: accepted. Context: review 2243 (B01's second half), ISO 32000-2 §7.4.10,
Table 14, §7.6.2, §7.6.6; ADR 3681; `pdf-syntax`'s
`a_stream_this_reader_holds_no_key_for_is_refused_rather_than_emptied`.

## What was tested

Review 2243 reasoned that `pdf-transform`'s insert-pages carry (`update.rs`, `Carry::value`) hands
an AES source's ciphertext `/Length` to the writer over decrypted data. `tests/update.rs` now carries
a page out of an AES-256 document into a clear one and into one encrypted under another key. With
ADR 3681 removed, the clear destination got `/Length 64` over 39 bytes, so the route was real. The
encrypted destination was already right, because `encrypt_for_update` re-derives the entry when it
encrypts.

Two further defects were found on the same route, and each has a test:

1. **A stream naming its own crypt filter** (§7.4.10's `/Crypt` first in `/Filter`, Table 14's
   `/Name` the source's `/StdCF`) kept that stage. The data had already been decrypted through the
   *source's* `/CF`. In a clear destination the name is undefined, and the page read back with no
   content. The carry now removes a `/Crypt` stage whose `/Name` is not `Identity`, with its
   `/DecodeParms` entry, when the source is encrypted. The destination's own §7.6.2 then decides
   how the bytes are written. An `Identity` stage stays, because it is the producer's statement
   that the stream is in the clear, and it means the same in any document.
2. **A stream whose decryption failed** is held empty by the reader. Carried, it would have been
   written empty, with the page's content replaced by nothing. The carry now refuses by name
   (`Refusal::Assembly`, naming the object), the same as `write::incremental_update` does for its
   own document.

## Left

`merge` and `split` copy `decryption_failed` streams and `/Crypt` stages the same way, through
`serialize`. Each is its own change, with its own tests, and is listed in the record.
