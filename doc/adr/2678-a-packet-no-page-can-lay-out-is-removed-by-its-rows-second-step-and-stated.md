# 2678 — A packet no page can lay out is removed by its row's second step and stated

Session 1928, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/config.rs`: `keeps_per_object`;
`archive/config/steps.rs`: the second step's admission; `archive/prepare/appended_pages.rs`: the
packets handed on and `filters_stated`; `archive/prepare/metadata.rs`: `RemovedPacket`;
`archive/statement.rs`: `packet_entries`; `archive/mod.rs`: `REMOVED_PACKET_ANSWERED`).
Context: ADR 2536 (an embedded file answered per file, the second step it built), ADR 1245
(`preserve` with `original = "page"`), ADR 2676 (a replaced packet with no bytes owes no page),
ADR 2679 (why no undeclared filter is tried), `doc/rfc/0014` sections 3 and 4.
Clauses: ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1; ISO 32000-2 §7.4.

## The premise, checked

`241573_ftp`'s catalog packet, object 67, is 532 bytes under a dictionary stating no `/Filter`;
the bytes inflate as Flate to a 1603-byte packet. Every shipped profile refused the document at
2b and 4. Under `state-every-loss` the refusal at both targets was `NOT_TEXT`, at the packet row
and, at 2b, at the property row that takes its decision. At 4 the two section 6.1.3
document-information rows refused as well, for custom keys Table 349 names no counterpart for.

`state-every-loss`'s own header said an information site's keeping step that cannot keep an
object removes and states it, and that only the embedded-file rows carried that second step out.

## Decision

1. **One predicate decides where a keeping step hands an object on**: `keeps_per_object` — the
   embedded-file rows' `preserve:append`, and the packet rows' `preserve` with `fresh-packet =
   true` and `original = "page"`. The step list's admission, the preservation's `no_page_carries`
   and the `append-statement` check all ask it, so the packet rows are configured exactly as the
   embedded-file rows are: `on-failure = "append-statement"` (or `"discard"`). `original =
   "attach"` and `"both"` are not admitted: an attachment keeps any bytes, so nothing is left over.
2. **What is handed on is exactly what the page refuses**: a replaced packet whose decoded data is
   not UTF-8, the test behind `NOT_TEXT`. A packet that is UTF-8 but undrawable (a control
   character, `doc/questions/Q778`) is not handed on and keeps its own refusal.
3. **The removal is stated in facts computed from the source**: the statement page lists the
   requirement, the loss `metadata-packet`, the stream's object, its stored length, the filters
   its dictionary states, and the reason. The report names it per stream (`replaced_packets[].
   removed`, and a `removed:` line); `xmpMM:History` records the same sentence beside the page's.
4. **The decision is `preserve` with `FreshMetadataPacket`**, as an embedded-file row whose files
   all took the second step is answered per file.

## Profiles

Only `state-every-loss` changes: its three packet rows at 2b and 4 gain `on-failure =
"append-statement"`; its 4f and 4e rows attach and are untouched. `no-data-loss` names no second
step and keeps `NOT_TEXT` by name, which a test pins. No other profile names a second step at a
packet row.
