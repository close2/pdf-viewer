# 1377 — A corpus's published password is supplied from one table, to us and to the references

Status: accepted. Session 1270.
Context: `crates/pdf-model/tests/support/corpus_passwords.rs`, `crates/pdf-model/tests/{corpus,oracle,raster_golden}.rs`,
`crates/render-raster/tests/corpus.rs`, `tools/pdfref/src/{reference,cache}.rs`.
Clauses: ISO 32000-2 §7.6.4.1, §7.6.4.3.3, §7.6.6 Table 25.

## Decision

§7.6.4.1 has a reader try the default user password and then says "the interactive PDF processor
should prompt for a password". A gate has nobody to prompt, so nine encrypted pdf.js corpus
documents were pages no gate compared — held by name in `LOCKED`, `NO_RENDER_NEEDS_A_PASSWORD` and
`NOT_COMPARABLE`. Their passwords are published beside them (seven in pdf.js's
`test/test_manifest.json`, `pr6531_1.pdf`'s in pull request 6531, `print_protection.pdf`'s — its
owner password — in pdf.js's browser test). A published password is a fact about the corpus, not a
secret, so the gates answer the prompt with it.

1. **One table.** `corpus_passwords.rs` holds the nine, keyed by pdf.js file name, each with its
   source. The four corpus gates read it and nothing else; `render-raster` through `#[path]`. The
   older private tables in five of `pdf-transform`'s and two of `pdf-vfs`'s corpus tests, `save_round_trip.rs` and
   `pdf-syntax`'s `encryption.rs` are not this round's files and still hold copies.
2. **The references are asked the same question.** `pdfref` gains `Reference::render_with_password`
   and `Cache::render_with_password` (`pdftoppm -opw/-upw`, `mutool draw -p`, `gs -sPDFPassword=`);
   the password is on the command line and so in the cache key, and `None` is byte-for-byte the old
   invocation, so no existing cache entry moves. `hayro`'s command line takes no password and is
   asked what it was asked before.
3. **`SASLprep` is the one place the references are handed a different spelling.** §7.6.4.3.3
   step (a) runs the password through SASLprep; the three references do not, and each refuses
   `saslprep-r6.pdf`'s published password and opens the file on its prepared form `SaSLprep`
   (measured). The table's `for_the_references` carries that form; it reaches the same hash.
4. **What stays out** is `encrypted-attachment.pdf` (no password published anywhere; ADR 1040) and
   `PDFBOX-4352-0.pdf` (its `/Encrypt` resolves to null) — each still named with its reason.

## Consequences

All nine open and draw complete in `pdf-model --test corpus`; all nine agree between `render-raster`
and the CPU oracle; `raster_golden` gains `issue21579.pdf` (its private table had lacked
`pässwört`). In the oracle eight agree with the consensus and `issue3371.pdf` — a 4.17-to-1 reduced
one-bit JBIG2 scan whose text is in the image — is `AMBIGUOUS_IMAGE_REDUCTION`'s, with its ink
ladder beside its name.
