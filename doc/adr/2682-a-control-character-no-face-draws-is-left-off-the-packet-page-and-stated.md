# 2682 — A control character no face draws is left off the packet page and stated

Session 1930, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/prepare/metadata.rs`: `OmittedCharacter`,
`without_controls`, `is_omitted_control`, `CONTROL_NOT_SET`; `archive/prepare/appended_pages.rs`:
the packet loop and the statement's site; `archive/statement.rs`: `omitted_entries`;
`archive/prepare.rs`: the history; `archive/mod.rs` and `archive/report.rs`: the report).
Profile header: `doc/profiles/state-every-loss.toml`. Tests:
`crates/pdf-transform/tests/archive/packet_characters.rs`.
Context: `doc/questions/A778` (the owner's answer), ADR 2678 (the second step it shares), ADR 2677
(the face's encoding), ADR 1025 section 4 (the layout's choices), ADR 2681.
Clauses: ISO 19005-2 sections 6.6.2.1 and 6.2.11.8, ISO 19005-4 sections 6.7.2.1 and 6.2.10.9;
ISO 32000-2 §7.2.3, §14.3.2.

## The premise, checked

`balloon_a1b_jp2k`'s second packet (object 2) holds U+0000 as the value of its header's `begin`
attribute, byte offset 17. Every profile that answers the packet rows by page refused it at 2b and 4
with `NO_FACE`: `as-if-printed`, `as-shown`, `only-metadata-loss`, `keep-everything`,
`no-data-loss` and `state-every-loss`. `refuse-any-loss` and `derive-attachments` refused it because
nobody authorised the packet's replacement. A scan of the decoded packets of the census's eight
`NO_FACE` documents found a C0 control character in this one only.

## Decision

1. **The same step and the same predicate.** A packet row's second step is admitted by
   `keeps_per_object`, which yields `no_page_carries = Discard`. Where it is admitted, a packet that
   is UTF-8 is laid out without its C0 control characters. The step hands on a part of the packet
   rather than all of it, and nothing new decides where. `append-statement` lists each character
   on the statement page, and `discard` names it in the report and `xmpMM:History` only.
2. **Which characters**: U+0000 to U+001F, except LINE FEED and CARRIAGE RETURN, which §7.2.3
   makes end-of-line markers and the layout already reads as such, and except HORIZONTAL TAB,
   which ADR 1025 section 4 sets as a space. U+007F and every C1 control are outside C0, so they
   still ask the face search for a glyph and still take `NO_FACE` (A778's scope, pinned by a test).
3. **Stated in facts computed from the packet**: one entry per code point per stream. Each entry
   gives the requirement, the loss `metadata-packet-character`, the stream's object, the code
   point written `U+XXXX`, the count, and the byte offsets in the decoded data (the first sixteen,
   then "and N more", `MOST_OFFSETS_NAMED`). A fixed reason follows: no face has a glyph for the
   character, and setting it would show `.notdef`, which sections 6.2.11.8 and 6.2.10.9 forbid. The
   character itself is never written. The report (`replaced_metadata_packets[].omitted`, an
   `omitted:` line) and `xmpMM:History` carry the same sentence.
4. **The packet keeps its existing rows' answer.** The fresh packet replaces it, and the decision is
   the page's own `preserve`. The packet is carried rather than removed, so `removed` stays `None`.

## Profiles

No profile file changes its rows. `state-every-loss` already names `on-failure =
"append-statement"` at the three packet rows (ADR 2678); only its header now says what the step
also takes. `no-data-loss` and the four other paging profiles name no second step, so they keep
`NO_FACE`, which a test pins for `no-data-loss`.

## A778's `Owes`, answered

Option (b) is built above. Option (a) would show the byte as its control picture, U+2400 plus the
code, with `/ToUnicode` mapping back. It waits for a shipped face that covers the Control Pictures
block, and none does: by `fc-query`, the four Liberation Sans faces and the ten Foxit Type 1
programs under `data/standard-fonts/` map nothing in U+2400–U+243F, and `data/cjk-pack/` holds no
font. Shipping one is its own round. When it is built, (a) replaces (b) under both stating profiles,
because the byte is then carried rather than lost.

## What it costs

A page that claimed to keep the producer's bytes now keeps all of them but the control characters,
and it says so on the page it appends. The control characters carry nothing a reader of the packet
needs: in the witness the one character stands where ISO 16684-1 puts the byte-order mark.
