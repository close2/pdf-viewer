# 2676 — A packet page in a file stating no intent is given one, and a replaced packet answers the property row

Session 1927, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/prepare/appended_pages.rs`:
`packets_may_be_preserved`; `archive/prepare.rs`: the intent prepared for it, `Cleaned::replaced` and
`Cleaned::only_replaced`; `archive/mod.rs`: `met_by_the_replacement` reading the property row, and
`NO_BYTES_TO_PRESERVE`).
Context: ADR 1014 (the appended page), ADR 2596 and ADR 2669 (the statement page's intent, the
pattern followed), ADR 2442 and ADR 2455 (a replaced packet meets the rows that read its
properties), ADR 1245 (`preserve` with `original = "page"`).
Clauses: ISO 32000-2 §8.4.1 Table 51; ISO 19005-2 and ISO 19005-4 section 6.2.4.3; ISO 19005-2
section 6.6.2.1 and 6.6.2.3.1, ISO 19005-4 section 6.7.2.1.

## The premise, checked

`doc/pdfa-blocking-issues.md` gave R14 four causes over six documents. Run one by one under
`state-every-loss` at 2b and 4 (`quorra-transform archive`): `breast+cancer+immunotherapy` was
`NO_OUTPUT_INTENT`, as written. The colour cure the brief proposed — writing the page's text colour
in the intent's family — answers nothing here: the source states **no** intent, and section 6.2.4.3
licenses `DeviceGray` through an intent of *any* family. The cause was that no intent was prepared:
the intent is prepared only where a device-colour row failed, and a page laying out a packet adds
none. `PDFBOX-3148-2-fuzzed` was the "no requirement asked" defect: its catalog packet is `/A85` data
that will not decode, the replaced-packet keep skips a packet with no bytes, and with nothing else to
compose `the_preserved_pages` answered `NOT_ASKED_FOR`.

## Decision

1. **A packet page's text is licensed by an intent this conversion adds.** Where a packet row or
   the property row is answered by page, failed, and the source states no intent, the intent is
   prepared as a failed `DeviceGray` row would have asked for it — the statement page's own
   construction (ADR 2596). A source stating an intent licenses the page already and is untouched.
   An embedded file's text is left out: its row also derives and draws, and those bring their own
   families.
2. **A replaced packet with no bytes owes no page.** The existing rule — a replaced packet that will
   not decode is replaced with nothing preserved, reported per stream — now holds when it is the only
   content: the packet row is decided `preserve` with `FreshMetadataPacket` and the sentence
   `NO_BYTES_TO_PRESERVE`. The output records no `xmpMM:History` entry for it, because no page was
   appended; the report names the replacement.
3. **A packet section 6.6.2.1 replaces meets section 6.6.2.3.1 by the replacement**, as ADR 2442 and
   ADR 2455 hold for the schemas, the fields and the amendment: a packet stating no property states
   none outside its schema. `prepare_properties` counts the streams it skips as replaced, and where it
   cut none the property row takes the packet row's decision.

## What moves

`breast+cancer+immunotherapy` and `PDFBOX-3148-2-fuzzed` convert under `state-every-loss` at 2b and
4; the census of ADR 2669 states the rest. Tests: `tests/archive/packet_pages.rs`, each with its
defect planted back and seen to fail.
