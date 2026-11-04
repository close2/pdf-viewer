# 2681 — A document information value that cannot be carried is removed and stated

Session 1929, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/config.rs`: `keeps_per_object`;
`archive/prepare/metadata.rs`: `RemovedInformation`, `NotCarried`, `value_type`;
`archive/prepare.rs`: `not_carried`; `archive/prepare/appended_pages.rs`; `archive/statement.rs`:
`information_entries`; `archive/mod.rs`; `archive/report.rs`). Profile:
`doc/profiles/state-every-loss.toml`. Tests: `crates/pdf-transform/tests/archive/carried_information.rs`.
Context: ADR 2680 (the carry this falls back from), ADR 2678 (the mechanism it shares), ADR 2358.
Clauses: ISO 19005-4 section 6.1.3; ISO 32000-2 §14.3.3, §7.3.5.

## The premise, checked

With ADR 2680 built, round 1929's census still found 19 documents refused at part 4 under both
stating profiles by a custom key ADR 2680 cannot carry. Among them is `241573_ftp`, whose `/Info`
states `/SaveStreams << /q 63 0 R /Q 64 0 R >>`, StampPDF's private streams. `issue15910` states
`/Copies 0`. §14.3.3 makes such a value "a text string", and these values are not. Writing one into
the packet would mean inventing a spelling for it, and ISO 19005-4 section 6.1.3 keeps no
dictionary entry for it.

## Decision

1. **The same predicate**: `keeps_per_object` is true for the two section 6.1.3 rows answered by
   `preserve` with `unmapped = "carry"`. The step admission, `no_page_carries` and the
   `append-statement` check all follow from it, as they do for the packet and embedded-file rows.
   So `on-failure = "append-statement"` (or `"discard"`) is admitted there.
2. **What is handed on is exactly what carry refuses**: a value that is not a text string, one
   holding a character XML 1.0 does not carry unchanged, or the empty key.
3. **Stated in facts computed from the file**: the requirement, the loss
   `document-information-entry`, the key as §7.3.5 escapes it, and the value's type (with the byte
   length for a string). It is never the value itself. A fixed reason follows. The report
   (`moved_information[].removed`, a `removed:` line) and `xmpMM:History` carry the same sentence.
4. **The decision stays** `InformationMovedIntoThePacket`, and each entry's fate says it was
   removed and stated.

## Profiles

Only `state-every-loss` changes: both rows gain `on-failure = "append-statement"`. `no-data-loss`
names no second step, and it refuses by name because the value is not kept. No other profile names
a second step at these rows.
