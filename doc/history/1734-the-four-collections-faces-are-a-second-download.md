# 1734 — The four collections' faces are a second download, and a collection's font embeds from it

Slot 7 of batch one hundred and twelve, 2026-10-23, an archive round on A569; ADRs 2302, 2303; Q596.
I worked in `scratchpad/r1734/`, the licence read before anything was downloaded there.
**Built.** (1) `tools/cjk-pack.py` builds the pack (`doc/cjk-pack.md`): five CID-keyed programs,
`Adobe-Japan1/GB1/CNS1/Korea1/KR`, glyph *N* the collection's CID *N*, from Source Han Sans 2.005R
(OFL 1.1, renamed `QuorraCJK-<Ordering>`), each CID placed only by a mapping Adobe publishes,
desubroutinized; two builds, identical digests. (2) `archive::fonts::pack` (ADR 2302): a collection's
font under a predefined non-identity `CMap` gets one §9.9.2 subset per descriptor under
`CIDFontType0C`, advances restated and proved, the OFL in its `/Metadata`, the loss `machine-face`.
(3) ADR 2303: found beside the executable or at `--cjk-pack`, read through the broker, held to
digests the program carries, never fetched. (4) `pdf-font`: restating a CID-keyed CFF copied its
`FDArray` unshifted, so each Font DICT missed its Private DICT; now shifted, a planted test holds it.
**Census** (converted / conforming / refused / unreadable), HEAD → the pack absent → the pack:
| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| pdf.js (967) | 757/2/197/11 → same → 765/2/189/11 | 723/0/233/11 → same → 730/0/226/11 | 737/2/217/11 | 704/0/252/11 |
| `doc/corpora` (491) | 343/6/125/16 → 344/6/124/16 → same | 336/0/139/16 | 302/6/167/16 → 303/6/166/16 → same | 290/0/185/16 |
The 37 composite refusals are 24 + 13: 13 name a collection. With the pack 8 convert at 2b, 7 at part
4; under `as-shown` 11 stop at `machine-face`; `90ms_rksj_h_sample` stops on its inline descriptor,
`issue13343` at the fence (one descriptor, two widths for a CID); the `doc/corpora` move is not mine
(below). **Drawn** (page 1, 100 dpi, looked at): seven Japan1 and GB1 outputs, `issue11555` vertical,
show the same characters in place in Source Han's shapes; neither corpus holds CNS1 or Korean.
**Hypotheses.** "The region builds carry the collections' CIDs": did not hold, they are
`Adobe-Identity-0`, so the builder maps through Adobe's CMaps. "Four programs under a checksum":
five, Korea1 beside KR. "Eighteen of the 37 convert": eight. **Premise that did not hold**:
`doc/stack.md` names no place the base faces live (they are compiled in); A569's "beside the exe" is.
**Gates.** Export of HEAD and my hunks (habit 138): `cargo fmt --check`, `-D warnings` clippy
`--all-targets`, rustdoc `-D warnings` on pdf-transform and pdf-font exit 0; nextest 964 passed, 0
failed, 15 skipped; the ignored pack test, pack named, 3 passed; the `pdf-font` test planted against
the old `with_charstrings` exit 101 (trap 13). `tools/state.sh --round 1734 archive` there exit 0:
validator over 0, missed 0, `archive_corpus` 2b 525/86, cross-check 0 disagreements; three census
walks `--tree 12` exit 0; veraPDF 1.30.3, eight outputs: five PASS, three FAIL at 6.2.11.3.1 alone. `cargo test -p conformance`: all pass but `records`, on 1728's record. Arms not compared:
the `pdf-font` function changed is reached only through `restate::with_widths`, called from
`pdf-transform` alone (grep), which no arm holds (habit 119).
**Found, not fixed.** veraPDF's 6.2.11.3.1 test wants the CIDFont's supplement at most the CMap's;
ISO 19005-2 section 6.2.11.3.1 wants at least (`issue11555`, `noembed-eucjp`, `noembed-jis7`).
`143424271191706722106594310729442158189.pdf` was refused by two builds of HEAD's source at its
output's `metadata/properties-use-known-schemas` (35 properties cut, against 50) and converted by
two later ones: a build-dependent cut. 9965 s.
