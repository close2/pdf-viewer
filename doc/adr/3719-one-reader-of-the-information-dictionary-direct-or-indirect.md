# 3719 — One reader of the information dictionary, direct or indirect

Session 2254, the fix round of audit 2253 (findings B02 and D01). Status: **accepted**.
Context: ADR 3581 (an agreed erratum is the standard), ISO 32000-2 §7.5.5 Table 15 with Errata
Collection 3's Issue #106, §14.3.3 Table 349, §14.5.

## Context

Issue #106 (approved) struck "; shall be an indirect reference" from Table 15's `/Info`. `optimize`,
`redact`, `split` and the archive converter's two rewriters read the entry only as a reference, so a
direct dictionary was dropped — beside a catalog `/PieceInfo` they carried whole, which left the
output without the `/ModDate` Table 349 requires there — and the converter's kept modification date
was computed and never written. `SetInformation` read both shapes but warned the operator that a
direct dictionary broke a `shall` the erratum struck.

## Decision

`pdf_transform::information` reads the trailer's `/Info` once — `Indirect` where it names a
dictionary, `Direct` where it is one, nothing otherwise — and `carry` takes each writer's own copy
of an indirect object, so a direct dictionary's references are carried the way that writer carries
any. A direct dictionary is written as a new indirect object: no table asks for that now and none
forbids it, and it is the shape every writer here already writes. `update`'s `SetInformation` reads
through the same function, writes a direct dictionary as an object of its own named by the update's
trailer, and no longer warns: the file was valid.

## Consequences

`metadata_writers_14_3_14_5_spec_audit.rs`'s B02 test passes; `stale_private_data.rs` pins `split`
and `update`. The archive's preparation and attachment readers already read through `get_key` and
are unchanged.
