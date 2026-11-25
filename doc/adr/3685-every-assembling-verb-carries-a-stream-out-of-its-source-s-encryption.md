# 3685 — Every assembling verb carries a stream out of its source's encryption, at one place

Session 2246. Status: accepted. Context: ADR 3682 (insert-pages), ISO 32000-2 §7.3.8.2, §7.4.10,
Table 14, §7.6.2; `pdf-transform/tests/encrypted_sources.rs`.

## What was tested

ADR 3682 left `merge` and `split` named as likely carrying the same two defects as insert-pages.
`tests/encrypted_sources.rs` drives every verb that writes streams it read out of another document
— `merge`, `split`, extraction (a `split` selection), `pages`' reorder and rotate (which write
through `merge`'s engine), `optimize` through the serializer and through the linearised writer, and
`archive`'s conversion — from an AES-256 (`/V` 5) and an RC4 (`/V` 4, `/CFM /V2`) source, into a
clear file and, for every verb but `archive`, into an encrypted one, reading each page back and
drawing it against the source's.

- **`/Length`**: already right on every route. The serializer's `write_body` restates each stream's
  `/Length` from the bytes it writes, so the ciphertext length 3681 met in the incremental writer
  never reaches a whole-file output.
- **A stream's own `/Crypt` stage** was carried by every verb; with it removed, page one of a merge
  read back empty in a clear output (confirmed by an environment-switched plant).
- **A stream whose decryption failed** was written empty by every verb that reached it; with the
  check removed, the merge wrote its page one blank and reported nothing.

## Decision

The verbs share one carry, `pdf_syntax::serialize`'s `Assembly`, except where `merge`, `split` and
`redact` rebuild a stream's dictionary before placing it. So the rule is stated twice, once each:

1. `serialize::carried_stream_dictionary` (moved from `update.rs`) removes a non-`Identity` `/Crypt`
   stage from a stream read out of an encrypted source. `Assembly` applies it to every copied
   stream; `merge`, `split`, `redact` and insert-pages call it where they rebuild one.
2. `Assembly::written` refuses a stream marked `decryption_failed` with
   `SerializeError::Undecrypted`, naming the output number and, where the slot stands for one, the
   source object. Both writers (`write_file`, the linearised writer) reach every object through it,
   so a synthesised stream that copied the flag is refused as a copied one is. Each verb reports
   it its own way: `Refusal::Assembly` for `merge`, `pages`, `optimize`, a declined piece for `split`.

## Cost and what is left

A refused write leaves in the caller's sink what the serializer had already handed it — no
cross-reference section and no trailer. That is true of every `SerializeError` after the first
byte, and is not fixed here: either the verbs write into memory before opening a sink or the sink
contract learns to discard.
