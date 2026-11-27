# 2281 — The errata moves read, the long walks declared, and a damaged `bfchar` map written as read

Lane `gwg`. The subjects are the train's failing `fetch-errata.sh --check`, the walks that kept clock
gates queued, and two archive items record 2275 left. The ADRs are 3825 to 3828.

- **The errata** (ADR 3825): re-fetched at `d9b2519a`. Twenty-nine issues changed, #865 is new, and
  six comments are new. `site/` did not change. #865 (§12.5.5, subdictionaries on non-button
  widgets) is open and evidence: it is writer guidance, and `pdf-model` already draws its witness as
  nothing and names Table 166. The twenty-eight ISO 14739-1 moves (eight closed with no comment,
  bodies edited, implementers' evidence) are inside the PRC exclusion. None is agreed against an
  implemented clause, so no code, quotation or ledger row moves. They are listed in
  `doc/errata-read.md`'s last section.
- **The walks** (ADR 3826): Tier 0 and Tier 1 of `script_corpus`, `pdf-archive --test cross_check`,
  `archive_corpus` and `archive_unbuilt` now declare `long`. The table in ADR 3826 gives their holds
  in `heavy-walk.log`. `awkward_classes` (median 510 s) is the longest walk still `small`.
- **A damaged encoding `CMap` that maps with `bfchar`** (ADR 3827): before this round the document
  was refused at `fonts/encoding-cmap-maps-codes-to-cids`; the new test failed with that refusal.
  `encoding_cmaps` now leaves a damaged map to `font_maps`, whose map, written from its reading,
  states the `bfchar` mappings as `cidchar`. In the stream rewrite `FontMapsAsRead` now comes before
  `EncodingCmapRestated`. The test covers the damage in a section of its own and inside the `bfchar`
  section, under both profiles: the output holds, the map reads alike, `A` is CID 36 and `B` is
  nothing.
- **The leading into a form** (ADR 3828): the survey keeps no operand values and no text state, so
  it does not know the invoking leading. The construction and what blocks it, the content reader's
  repairs, are written down. Not built.
- The brief named the bfchar item as "a damaged `/ToUnicode`" and "2270's route (ADR 3781)". Record
  2275 and ADR 3804 show that the leftover is the encoding `CMap` and 2275's route, so that is the
  item built.
- Census against round 2275's, with the pack and without it: every line is the same. veraPDF 1.30.3:
  5523 outputs, 5471 compliant, 40 not, 12 unfinished, as round 2275's; the 4 fixture outputs are
  compliant. `tools/state.sh --round 2281 launch`: 0, with its clocks not measured (load 59).
- Ledger: §9.7.5.3, §9.7.5.4 and §12.5.5 stay `implemented`. §9.7.5.4 gains a test.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p
conformance`: fmt 0 (after `rustfmt` of the new test), clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 6336 of 6337, the one
failure being this record before this paragraph, rustdoc `-D warnings --document-private-items`
(`pdf-transform`, private directory) 0, fuzz `cargo check --bins` 0, `cargo test -p spec-errata` 0
(27), `spec-errata check` over the new files 0, `cargo test -p conformance --no-fail-fast` 484 of
485, the same failure.
