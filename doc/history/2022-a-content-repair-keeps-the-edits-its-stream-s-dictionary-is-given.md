# 2022 — A content repair keeps the edits its stream's dictionary is given

Session 2022, a single round, PDF/A lane's worktree: the `D-defect` round 2021 found
(`hesr13613-sup-0001-Authormatrix` at part 4). ADR 2860.

**The premise.** True. HEAD reproduced every census line of the document under all eight profiles
at both parts. At 4, `as-if-printed`, `as-shown` and `state-every-loss` were refused on one row:
the output failed `fonts/cid-to-gid-map-present` (four places, three done). The other five
profiles refuse it at the operator row (2b), the information dictionary rows (4) or both, as before.

**Found.** The source holds no CIDFont object. Its four Type 2 CIDFonts are written directly in
Type 0 fonts in the `/Resources` of four text-widget appearance forms, and `sites::cid_to_gid_maps`
names all four forms as holders (ADR 2722). The fourth form is the stream whose lone `>` ADR 2859
writes as white space. `rewrite_stream` returned that repair's stream, built from the source
dictionary, before `rewrite_dictionary`, so the holder's edit was dropped. At 2b the second pass
(ADR 2534) wrote it. At 4 the first pass's output is labelled PDF 2.0, which has no default, so the
second pass declined.

**Built.** ADR 2860 (`archive/rewrite/streams.rs`, `with_its_dictionary_rewritten`). The three
content repairs (language respelling, hexadecimal repair, `content-as-drawn`) give their
replacement the dictionary edits and `rewrite_xobject`'s edits that the stream would otherwise
have been given. No profile and no loss changes. Ledger 9.7.4.2 stays `implemented`, with its code,
test and note extended.

**Freed.** `hesr13613` at 4 under `as-if-printed`, `as-shown` and `state-every-loss`. At 2b it now
converts in one pass. **Pixels** (CPU, 72 dpi, intent renamed): `as-shown` at 2b and 4 differs in at
most 2 pixels a page, by 1 level. `as-if-printed` and `state-every-loss` at 4 differ in up to 20 348
pixels, by up to 134 levels. Those are round 2021's fonts-row losses (`machine-face`,
`substitute-stretch`).

**Left.** The MediaBox `/UserUnit` bracket (2 documents at 2b). It puts operators of this program's
own around the producer's, which ADR 0816's line has to be argued against, and the round's budget
did not cover that. Also left: round 2021's other items (the Private Use `ActualText`, `issue6931`,
`issue5010`, ADR 2847 §3).

**Gates.**
- fmt 0; clippy `-D warnings --all-targets` (`pdf-transform`) 0; nextest `pdf-transform`, `pdf-archive`
  0 (1533); the new test fails under its plant (the repair's stream returned as built); archive tier 0, 0.
- Censuses against 2021 (`cmp.py`): shipped 2 NEW, stating 1 NEW, 0 GONE, 0 CHANGED. No other
  output carried the miss. veraPDF 1.30.3: 6 of 6 PASS. `rank.py --table`: the `D-defect` row goes.
