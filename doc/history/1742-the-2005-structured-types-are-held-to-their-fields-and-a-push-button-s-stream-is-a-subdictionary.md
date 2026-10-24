# 1742 — The 2005 structured types are held to their fields, and a push button's stream is a subdictionary

Slot 7 of batch 113, 2026-10-24, an archive round; ADRs 2318, 2319. I worked in `scratchpad/r1742/`.
**Built.** (1) ADR 2318: every property of the 2005 edition whose value is a structure or an array
of one names its type in `STRUCTURED` — `Thumbnail`, `Job`, `Dimensions`, `Font`, `Colorant`, nine
Dynamic Media and four EXIF types — held field by field, nested `Time` fields in turn; `Font` read
under the printed `http:` namespace and the corrected `http://`. Part 4's tag row finds a stream
with no `acsp` short of the signature, as part 2's does (§8.6.5.5's stream "shall contain the ICC
profile"). (2) ADR 2319: a push button's single `N` stream becomes the one entry of a subdictionary
its new `/AS` (`Normal`) selects; a check box's or radio button's keeps a refusal in its own
sentence. (3) `redact/images.rs`: the budget, a space no removal writes back (PIMA 7667's refusal)
and any other decode failure each have one sentence.
**Premises.** The census's `normal-appearance-shape` sentence (no `/AS`) held for none of its pdf.js
documents: all are push buttons with one stream (bytes read first, habit 105). "Seven types, two
findings each": none — the witnesses (`Thumbnail` 9, `Dimensions` 12, `Font` 5, `Flash` 1 files)
state 2005 fields, and `xmpG:tint` sits in `xmpTPg:SwatchGroups`, another row's. "The next refusal
is an image rule": no; image smoothing is a loss both profiles stop on by argument and the composite
fonts wait on Q570. The first build misread `frameOverlappingPercentage`, wrapped over two lines of
its table; the validator over the veraPDF corpus caught it (`6-6-2-3-1-t02-pass-z`).
**Drawn** (100 dpi, before and after, looked at): `js-buttons`, `bug1675139`, `resetform`, `evaljs`,
buttons where they were, within one level. veraPDF 1.30.3: those four at 2b, `js-buttons`,
`listbox_actions`, `form_two_pages` at 4, all PASS.
**Census** (converted / conforming / refused / unreadable, against HEAD's export):
| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| pdf.js | 757/2/197/11 → 777/2/177/11 | 751/0/205/11 → 771/0/185/11 | 737/2/217/11 → 755/2/199/11 | 732/0/224/11 → 750/0/206/11 |
| `doc/corpora` | 344/6/124/16 → 349/6/119/16 | 339/0/136/16 → 344/0/131/16 | 303/6/166/16 → 306/6/163/16 | 292/0/183/16 → 295/0/180/16 |
The site's line goes 23 → 0 (pdf.js) and 7 → 1 (`sakula.pdf`, no `N`) in every column; no other
line moved, nothing newly refused.
**Gates.** In an export of HEAD and my own hunks (slot 1 edits the same files, habit 162): `cargo fmt
--check`, `-D warnings` clippy `--all-targets` and rustdoc `-D warnings --document-private-items` on
pdf-archive and pdf-transform exit 0; nextest 974 passed, 0 failed, 17 skipped. Plant: all six new
tests fail with their behaviour taken out (trap 13). Validator over the veraPDF corpus: 2b 970
agreed, every target missed 0 and over 0 (the misread build: over 1). Converter over it: 2b 526/85
(round 1734's 525/86), 4 271/88, every conforming input still conforming. `tools/state.sh --round
1742 archive` exit 0, cross-check 13 560 compared, 0 disagreements. Census `--tree 6` exit 0 in
both exports. Arms not compared: from `render-raster`, `cargo tree -i` reaches neither crate (habit
119). `cargo test -p conformance --test evidence --test adr_status --test records` exit 0.
**Found, not fixed.** `archive/transcode.rs` maps every `jpx_samples` error to one sentence (slot 1's
file). 9900 s.
