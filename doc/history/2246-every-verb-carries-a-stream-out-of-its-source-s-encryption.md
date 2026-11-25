# Round 2246 — every verb carries a stream out of its source's encryption, and a computed number refuses inf

Round 2245's leftovers, in the `audit` worktree. 2026-11-25. ADRs 3685 and 3686.

## What changed

- **Every assembling verb, tested end to end** (`pdf-transform/tests/encrypted_sources.rs`, five
  tests): `merge`, `split`, extraction, `pages`' reorder and rotate, `optimize` (plain and
  linearised) and `archive`, from an AES-256 source and from an RC4 (`/V` 4, `/CFM /V2`) source the
  test writes by §7.6.4's algorithms; into a clear file, and into an encrypted one where the verb
  can write one. Each page reads back with its source's text and, but for rotate and archive, draws
  the same.
- **`/Length`** was already right: the serializer restates it from the bytes written.
- **The spent `/Crypt` stage and the undecrypted stream** were carried by every verb; a plant
  (an environment switch, removed) turned page one of a merge blank on each. One rule for each,
  in `pdf_syntax::serialize`: `carried_stream_dictionary` (moved from `update.rs`) applied to every
  copied stream and called where `merge`, `split`, `redact` and insert-pages rebuild one; and
  `Assembly::written`, which both writers go through, refusing a `decryption_failed` stream as
  `SerializeError::Undecrypted`, named by output and source object (ADR 3685).
- **One real formatter** (§7.3.3): `pdf_syntax::write::real_number` and `real_number_f32` answer
  inf and NaN with `write::NonFinite`. Through it: `appearance.rs`'s `Stream` (appearance not
  drawn, `Refusal::NonFinite`), `colour_operator`, `variable_text.rs` and `rich_text/layout.rs`
  (`Owed::NonFinite`), `redact/overlay.rs`'s `/RO` origin. A script's NaN colour component wrote
  `NaN 0 0 rg` into a `/DA`: `pdf-script`'s `colour_of` now answers no colour (ADR 3686).
- `media.rs`'s module comment restated: the players a host has, the hand-off and this program's
  own (A946, A1006).

## Left

1. `pdf-transform/src/archive/`'s three real writers (another lane's files this session).
2. A refused write leaves a truncated file in the caller's sink (ADR 3685, "Cost").

**Gates.** fmt 0; clippy `-D warnings --all-targets` (pdf-syntax, pdf-model, pdf-transform,
pdf-archive, viewer-qt, pdf-script) 0; nextest of the same 5330 run, 3 failures, exactly the
manifest's B02 to B04; rustdoc 0; `fuzz` check 0; conformance 0.
