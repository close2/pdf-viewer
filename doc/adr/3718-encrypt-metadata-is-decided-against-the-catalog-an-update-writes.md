# 3718 — `/EncryptMetadata` is decided against the catalog an update writes

Session 2254, the fix round of audit 2253 (finding B04). Status: **accepted**.
Context: ADR 3234 (the stream filter's order), ISO 32000-2 §7.5.6, §7.6.4.2 Table 21, §14.3.1 and
§14.3.2.

## Context

Table 21's `/EncryptMetadata` "Indicates whether the document-level metadata stream (see 14.3.2,
"Metadata streams") shall be encrypted." `Document::stream_filter` decided which stream that is by
asking the *source's* catalog. `SetInformation` in an encrypted PDF 2.0 file creates the packet
§14.3.1 sends the title to and names it from a rewritten catalog, so under `/EncryptMetadata false`
the new packet was encrypted like any stream: the file said its metadata was in the clear and held
ciphertext there, and every reader, this one included, lost the title.

## Decision

The incremental writer works out the document-level metadata stream as the *updated* file names it
— the `/Metadata` of the catalog among the replacements where the update rewrites it (or the root
its trailer additions name), the source's otherwise (`write::written_metadata`) — and hands it to
`encrypt_for_update`. The reader keeps asking the file's own catalog, and only when a stream's
`/Type /Metadata` makes it a candidate (`Metadata::AsRead`), so a read costs what it did. One
`stream_filter` still states the rule for both directions.

## Consequences

`metadata_writers_14_3_14_5_spec_audit.rs`'s B04 test passes, and its control (`/EncryptMetadata
true`) still reads back. The whole-file serializer already decided against the assembly's own
catalog and is unchanged.
