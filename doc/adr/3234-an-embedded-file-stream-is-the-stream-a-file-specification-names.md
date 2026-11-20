# 3234 — An embedded file stream is the stream a file specification names, and a rule about one stream asks which stream it is

Session 2133. Status: accepted and built. Context: review 2131 findings B01, B05, B06; ISO 32000-2
§7.5.8.2, §7.6.2, §7.6.6, Table 20 (`/EFF`), Table 21 and Algorithm 2 NOTE 1 (`/EncryptMetadata`),
Table 43 (`/EF`, `/RF`), Table 44, §7.11.4.2; CLAUDE.md principle 2 ("Nothing eager").

## Decision

Three rules of §7.6 are about a particular stream, and `Document::stream_filter` now asks which
stream it is rather than what its dictionary says:

1. **The document-level metadata stream (B01).** Table 21's `/EncryptMetadata` "[i]ndicates whether
   the document-level metadata stream ... shall be encrypted", and Algorithm 2's NOTE 1: "This
   provision pertains only to document-level XMP metadata." A `/Type /Metadata` stream is left in
   the clear only where its number is the one the catalog's `/Metadata` names
   (`Document::is_document_metadata`); the writer's mirror asks the output's catalog the same.
2. **A cross-reference stream's dictionary (B05).** §7.5.8.2: "strings appearing in the
   cross-reference stream dictionary shall not be encrypted". `decrypt_value` and `encrypt_value`
   carry a `/Type /XRef` stream's dictionary unchanged, as they already carried its data.
3. **An embedded file stream (B06).** Table 44 makes `/Type` "(Optional)", so what makes a stream
   one is a file specification's `/EF` (or `/RF`'s related files array) naming it. An untyped
   stream with no `/Crypt` filter, in a document whose `/EFF` method differs from `/StmF`'s, is
   looked up in a set of object numbers built once by `scan_embedded_file_streams` and kept in
   `Document::embedded_files`. A stream stating another `/Type` is taken at its word.

## Why this construction

Decryption runs one object at a time, when the object is loaded, and an embedded file stream is
defined by the reference to it. Three constructions were weighed:

- **Decide at the referrer** (decrypt under `/EFF` when a caller reaches the stream through
  `/EF`): the object cache would hold whichever answer was loaded first, and a caller that reaches
  the stream by number — the audit test does — gets the other. Rejected.
- **Try `/StmF`, then `/EFF`**: a guess that AES padding decides, and RC4 decides nothing. Rejected
  under trap 5.
- **Collect the references once** (built). Table 43 helps: where `/EF` is present "the file
  specification dictionary shall be indirectly referenced", so the scan reads the file's indirect
  objects and no deeper, each *without decryption* (`Document::parse_plain_at`) — names and
  references are never encrypted, and decrypting would ask the question being answered.
  Members of object streams are read from the expanded stream, which §7.6.2 already leaves alone.

The scan never decrypts an untyped stream, so nothing it reads is cached under the wrong filter. A
stream the scan itself must decrypt (an object stream with no `/Type`, a `/Length` naming a
stream) asks the question again on the same thread; that thread is recorded in `scanning`, and
the answer there is "not an embedded file stream", which is what the reader answered before.

## Cost

Nothing for a document whose `/EFF` is absent or names the same method as `/StmF` — every
document the corpus is known to hold but the attachment-only ones (`/StmF /Identity`, `/EFF
/StdCF`). For those, the first untyped stream — usually page one's content stream — pays one parse
of every indirect object in the file, uncached; it is on the launch path of exactly those files.
An incremental update that adds an *untyped* embedded file stream to such a document is encrypted
under `/StmF` by `encrypt_for_update`, whose set is the file's as opened; this reader writes
`/Type /EmbeddedFile` on what it embeds, so no update of its own reaches that case.
