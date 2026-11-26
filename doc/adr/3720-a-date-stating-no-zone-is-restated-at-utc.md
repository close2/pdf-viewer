# 3720 — A date stating no zone is restated at UTC

Session 2254, the fix round of audit 2253 (findings B01 and D02). Status: **accepted**; supersedes
the zone in ADR 1013 section 5's reading of what the packet writer emits.
Context: ADR 1013 (a date is what the text defines), ADR 1473 (the packet beside the dictionary),
ADR 3581; ISO 32000-2 §7.9.4 with Errata Collection 3's Issue #251, §14.3.4.

## Context

`pdf_model::xmp::spelled_date` wrote `pdf_syntax::Date`'s `Display`, which writes a zone only where
the file stated one: `/ModDate (D:20250102030405)` went into the packet as `2025-01-02T03:04:05`.
Issue #251 (approved) replaced §7.9.4's sentence on an absent zone with "the missing timezone offset
shall be assumed to be the same as Greenwich Mean Time's timezone offset (+0'00)", so the dictionary
states 03:04:05 UTC and the packet a local time of unknown zone — not "fully equivalent" (§14.3.4),
and none of the XMP Specification's profiles, which ADR 1013 section 5 says this writer emits.

## Decision

`spelled_date` writes an absent zone as `Z`, the spelling `Display` already gives a stated zero. The
viewer's `Display` is unchanged: it shows the date as the file wrote it, and `Date::offset` keeps
`None` apart from `Some(0)` for that alone. `update`, `merge` and the archive converter share the
function, so all three restate the same instant.

## Consequences

`metadata_writers_14_3_14_5_spec_audit.rs`'s B01 test passes; `xmp.rs`'s own tests now expect `Z`.
The 14.3.4 ledger note, `spelled_date`'s comment and `date.rs`'s comments on the offset are restated
(D02). ADR 1013 is a record and is not edited.
