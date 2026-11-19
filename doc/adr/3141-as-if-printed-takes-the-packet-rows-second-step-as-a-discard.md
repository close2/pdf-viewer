# 3141 — `as-if-printed` takes the packet rows' second step as a discard

Session 2110, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R14 row.
Status: accepted and built. In `doc/profiles/as-if-printed.toml`: `on-failure = "discard"` at the
three packet rows. Tests: `crates/pdf-transform/tests/archive/packet_removed.rs`
`as_if_printed_removes_a_packet_no_page_can_lay_out_and_names_it`, and
`crates/pdf-transform/tests/archive/packet_characters.rs`
`as_if_printed_leaves_a_control_character_off_the_packet_page_and_names_it`. Context: ADR 2678 (the
second step and its predicate, `keeps_per_object`), ADR 2682 (a C0 control character left off the
page), ADR 1245 (`original = "page"`), `doc/questions/A681` and `A694`.
Clauses: ISO 19005-2 section 6.6.2.1, ISO 19005-4 section 6.7.2.1; ISO 32000-2 §7.4.

## Context

R14's documents under `as-if-printed`, by the census of session 2106: `241573_ftp` at 2b and 4
stopped on `NOT_TEXT` (its catalog packet is Flate data under no `/Filter`), `balloon_a1b_jp2k` at
2b and 4 on the face search's refusal (U+0000 where the packet header's byte-order mark goes),
`JPXTestGrey` at 2b on the extension-schema container, and `poppler-742-0-fuzzed` at 4 on a packet
that cannot be edited in place. The first two are the sentence that frees the most: two documents
at each part.

Both convert under `state-every-loss`, whose packet rows name `on-failure = "append-statement"`
(ADRs 2678 and 2682). ADR 2678 admitted `"discard"` at the same rows by the same predicate and
named no profile that used it. `as-if-printed`'s metadata section already states the yardstick:
"A printed page carries none of it. `preserve` is still chosen where it is free."

## Decision

1. **The three packet rows of `as-if-printed` name `on-failure = "discard"`.** A packet a page can
   carry is still laid out on one, as before. A packet whose decoded data is not UTF-8 is removed,
   and a C0 control character no face draws is left off the packet's page; each is named in the
   report (`replaced_packets[].removed`, `.omitted`) and in `xmpMM:History`, and no statement page
   is appended. A print scanned back in carries no packet at all, so the profile's own question —
   would printing have lost this anyway? — answers yes, and appending a page stating it would
   compose a page a print never had.
2. **Nothing in the code changes.** The step, its predicate and its report are ADR 2678's and
   2682's; this is the first profile to name `discard` there.
3. **What still refuses**: a UTF-8 packet holding a character outside C0 that no face draws (U+007F,
   a C1 control) keeps the face search's refusal, since ADR 2678 hands on only what is not UTF-8
   and ADR 2682 leaves off C0 alone. No census document reaches it.

## Evidence

veraPDF 1.30.3 passes `241573_ftp` and `balloon_a1b_jp2k` written by this profile at 2b and at 4.
Drawn by this reader beside the source, `balloon_a1b_jp2k`'s page is identical pixel for pixel;
`241573_ftp`'s ten pages differ only by the device colours read through the output intent's
profile (no pixel differs by 20 %), which is that row's own change, and the packet page follows.

## What it costs

Under `as-if-printed`, a packet no page can set is gone from the archive rather than refused. An
archive that would rather keep it chooses `state-every-loss`, which lists it on a page, or
`no-data-loss`, which refuses.
