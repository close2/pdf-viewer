Status: complete
Given: 2026-11-20, in conversation — transcribed by the round
Owes: add pdf-media-worker and quorra-sound to viewer-qt/tests/unsafe_position.rs's allowlist for ADR 1945's pre-input close_range descriptor closure (recorded in an ADR) so main turns green, with optional follow-up to consolidate keep_only_the_standard_descriptors into one shared crate so future confined helpers do not widen the list again (a viewer / sandbox round)

> Q979: Your recommendation

Reading: adopts the recommendation — names `pdf-media-worker` and `quorra-sound` in `viewer-qt/tests/unsafe_position.rs` under ADR 1945 (identical to A430's pre-input `close_range` descriptor closure before reading any untrusted bytes, while each crate's library keeps `forbid(unsafe_code)`), recorded in an ADR; also welcomes a clean follow-up consolidating `keep_only_the_standard_descriptors` into one shared crate so future confined helpers stay under a single entry.
