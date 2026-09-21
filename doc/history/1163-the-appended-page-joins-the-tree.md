# 1163 — The appended page joins the structure tree

The corpus-named round of batch twenty-four: the PDF/A converter's `preserve`/`append` remedy,
refused since session 1006 for every document that describes its content.

## What was found

Against `doc/adr_revisit/1025-a-page-of-the-packet-the-producer-wrote.md`, whose claims were to be
verified before anything was built on them:

- The permission claim and the measurement claim held.
- The machinery claim did not, as stated. `pdf-fuse` is RFC 0003's FUSE face and carries no
  structure code; the ADRs named are `pdf-transform`'s `structure.rs` and `pdf-model`'s reader.
  Neither is reusable here — `Carry` plans a tree for a **derived** document — but the *reading*
  in them is what made the clause quick to get right.
- Annex L's Table L.2 would have forbidden the shape this builds. §14.8.6.1 is why it does not
  bind: an element with no `/NS` is in the default standard structure namespace, which that clause
  makes the PDF 1.7 one, and Annex L governs the PDF 2.0 one. The corpus agrees from the other
  side — the one document at issue states two `Slide` elements at its root and no `Document`.
- A second refusal fell out on the way. The corpus's other refused document stated no character
  outside `/WinAnsiEncoding` at all: its packet is CRLF, and the face search demanded a glyph for
  the carriage return that `wrapped` had already trimmed. §7.2.3 makes CR and LF one end-of-line
  marker, so `is_set` excludes both and `wrapped` reduces the three spellings to one.

## What was built

`crates/pdf-transform/src/archive/tagged.rs`, new: §14.7.2's `Part` holding a `P` per line,
§14.7.5.4's `/StructParents`, `/Nums` entry and `/ParentTreeNextKey`, with four named refusals for
the shapes it will not rewrite. `preserve.rs` writes §14.7.5.2's marked-content sequence per line;
`rewrite.rs` applies the two dictionary edits. ADR 1163 is the argument.

## Measured

`quorra-transform archive <doc> --to 2b --config doc/profiles/only-metadata-loss.toml -o /dev/null`
over each of `doc/pdf.js/test/pdfs`, counting the nine that reach
`metadata/properties-use-known-schemas` by whether the report says the packet *is on page(s)* or
gives a refusal: **7 preserved / 2 refused -> 9 preserved / 0 refused**, the first count from a
build of `HEAD`.
