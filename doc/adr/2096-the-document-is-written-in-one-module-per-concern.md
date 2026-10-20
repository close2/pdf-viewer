# 2096 — `Document` is written in one module per concern

Session 1630, slot 8 of batch ninety-eight, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 8, first half. Status: **accepted** and **built** in `crates/pdf-syntax/src/document.rs` and
`document/`. Context: the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not fix any
bugs."; ADRs 2004, 2018, 2032, 2046, 2064, 2080 and 2081 (the same split of other types). Clauses:
none; no behaviour moves. ADR 2097 is the colour space's half of the entry.

## 1. What the census found

`document.rs` was 3 364 lines: the state and eleven small types, one `impl Document` of 66 methods over
1 880 lines, the decoded-stream cache (`DecodedStreams` and its entry, 450 lines), and about 580 lines of
tests. The methods already clustered: opening, the reports of a recovery, object resolution, the repair
of a disproved cross-reference entry, the cipher, the object streams, the filter chain and the readers
of its output.

## 2. The decision

`document.rs` stays the root: `Document` and its fields, the entry points `open`, `open_with_limits`,
`open_with_password` and `empty` with the two constructors they share (`around`, `as_of`), the bounds
(`DECODED_BUDGET`, `RECOVERY_DECODE_BUDGET`, `MAX_REFERENCE_DEPTH`), the lock helpers, the public result
types (`StreamRefusal`, `StreamSource`, `Decoding`, `ImageStream`) and the tests. Nine modules each hold
an `impl Document` for one concern, and one holds a type:

| module | concern |
|---|---|
| `objects` | §7.3.10: the trailer, the catalogue, one object by number, one entry of a dictionary |
| `parsing` | one indirect object at an offset: its stated length, and the extent when it is wrong |
| `repair` | an entry the file disproves, found by its own header (`load_by_header`, `scan_refused`) |
| `encryption` | §7.6: authentication, the permissions, the cipher both ways, and §7.6.2's signature exception |
| `object_streams` | §7.5.7: the members of an object stream, and their entry into a rebuilt table |
| `reports` | what a recovery found: misfiled objects, padded keywords, `ObjectsLost`, `CompressedRecovery` |
| `decoding` | §7.4: the chain run to the end, and the cache's read-out |
| `images` | a stream that carries an image: its data with the codec still to be applied |
| `sources` | how a caller reads the decoded bytes, and where an inline image's data ends |
| `cache` | the decoded-stream cache under its byte budget (ADRs 0260, 0317, 0437) |

**Visibility is what a sibling reads.** A private method or type a sibling module calls is
`pub(super)`; the compiler decided each by error, and each `pub(super)` was then removed one at a time
and kept only where the build failed. The public types `ObjectsLost`, `CompressedRecovery` and
`DecodedStreamCache` are re-exported from the root, so `pdf_syntax::` and `crate::document::` paths
did not change; `is_signature_dictionary` is re-exported `pub(crate)` for `serialize`.

## 3. The proof that nothing moved

The multiset of the old file's non-blank lines (visibility words removed, whitespace collapsed) against
the root and the ten modules: removed-only is 16 lines, all of them import lists that narrowed to what
each file uses, 12 doc-comment lines whose link target is now re-qualified (`SyntaxError` as
`crate::SyntaxError`, `DECODED_BUDGET` as `super::DECODED_BUDGET`, and so on; habit 126), and one
signature `rustfmt` wrapped. Added-only is the module docs, the `use` and `mod` lines, the nine `impl
Document {` wrappers, and the re-qualified links. `cargo test -p pdf-syntax` passes unchanged and the
six corpus arms agree with HEAD's page by page.

## 4. What this costs

A reader looking for `get_key_of` opens `objects.rs`; one looking for `stream_source`, `sources.rs`.
`document.rs` is still 1 050 lines, about 600 of them tests. Twenty-two ledger rows name the modules
that cite their clause (habit 96), and the two teaching diagrams that named `document.rs` name
`document/objects.rs`.
