# 2860 — A content repair keeps the edits its stream's dictionary is given

Session 2022, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/rewrite/streams.rs`:
`Rewriter::with_its_dictionary_rewritten`).
Context: ADRs 2432 and 2722 (the `/CIDToGIDMap` holders), 2534 (the second pass), 2638 and 2859
(`content-as-drawn`), 2067 (the language respelling), the hexadecimal repair; round 2021's record.
Clauses: ISO 32000-1 §9.7.4 Table 117; ISO 32000-2 §9.7.4.2, Table 115; ISO 19005-2 section
6.2.11.3.2, ISO 19005-4 section 6.2.10.3.2.

## 1. What the census said

`hesr13613-sup-0001-Authormatrix` was refused at part 4 under `as-if-printed`, `as-shown` and
`state-every-loss` with the output failing `fonts/cid-to-gid-map-present`: four places, three
done. Its four Type 2 CIDFonts are each written directly in a Type 0 font in the `/Resources` of a
text widget's appearance form, and none states `/CIDToGIDMap`. `sites::cid_to_gid_maps` names all
four forms holders (ADR 2722). The fourth form is also the stream whose lone `>` ADR 2859 writes as
white space.

## 2. Why one was missed

`rewrite_stream` tries the rewrites that replace a whole stream first and returns the first that
applies. Three of them repair a content stream's bytes: the language respelling, the hexadecimal
repair and `content-as-drawn`. Each builds the replacement from the source dictionary, changing
`/Filter`, `/DecodeParms` and `/Length`. Returning it skipped `rewrite_dictionary` and
`rewrite_xobject`, so every dictionary edit planned for that object was dropped. Here that was the
`/CIDToGIDMap` written into the holder's direct font. At 2b the second pass (ADR 2534) wrote the
entry into the first pass's output. At 4 that output is labelled PDF 2.0, and Table 115 of ISO
32000-2 states no default, so the second pass declined. The first pass had to be right.

## 3. Decision

A replacement from one of the three content repairs is given the dictionary edits the stream's own
dictionary would have been given (`with_its_dictionary_rewritten`). Those repairs change the bytes
and their encoding, and nothing they write is a dictionary entry any other rewrite edits. So the
two compose with no ordering question. The other whole-stream branches are left as they are: each
writes a dictionary of its own construction (a metadata stream, a font program, an image), and
none is a holder any dictionary site names.

No profile's choice changes and no loss is added. The `/CIDToGIDMap /Identity` written is the
value ISO 32000-1 Table 117 gives as the default ("Default value: Identity"), which a file labelled
PDF 1.6 already meant (ADR 2246).

## 4. Consequences

- `hesr13613` converts at 4 under `as-if-printed`, `as-shown` and `state-every-loss`, and at 2b
  in one pass.
- Any census document whose content-repaired stream has dictionary edits gets them in the first
  pass. The census comparison names those documents.
- The test `a_form_whose_bytes_are_written_as_drawn_keeps_its_cid_font_s_identity_map` asserts an
  empty `settled` at both parts.
