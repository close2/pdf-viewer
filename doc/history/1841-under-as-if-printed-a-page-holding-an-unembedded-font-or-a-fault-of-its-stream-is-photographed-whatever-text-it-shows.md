# 1841 — Under `as-if-printed` a page holding an unembedded font or a fault of its stream is photographed whatever text it shows

Slot 2 of batch 125, 2026-10-28, an "archive" round (Sonnet, high, two sessions); ADRs 2516 and 2517
and Q707 written, no number left unused. I worked in `scratchpad/r1841/`. No ledger status moves: an
archive round on R08 and R03 (A694 item 1), no clause row changes state.

**Premise** (the gate is a function of the profile and the target): did not hold as stated.
`text-as-image` is granted by both shipped profiles, so a gate keyed on it widened
`as-shown`; the wide gate has a loss of its own, `page-as-image`, that `as-if-printed`'s `discard` at the font
site grants (ADR 2516). Second: at Level B a photographed page wrote no `xmpMM:History` (ADR 2517).

**Built.** `Loss::PageAsImage`; `archive/fenced.rs` (the R03 rows a page's stream fails and the pages they
name); `fonts/photographed.rs`: `Gate`, the wide path with the page's text named (1000 characters, the count
left out), the faults of the document's own stream baked in, the reader's gaps (Image, Shading, Content, a
bound, its §9.3.8 and §11 shortfalls) refused; `fonts.rs`: fonts photographed around (16); bounds of 32
pages and `1 << 28` pixels on the wide gate alone, the fenced pages refused before any is drawn; the report
and the History carry the text. An inline image's `/I` stays refused under `as-shown` and is photographed
under `as-if-printed` (the test says both).

**Census** (`archive_corpus`, versus HEAD, `as-if-printed`, converted counts at 2b / part 4):
with the CJK pack pdf.js 838 to 857 / 841 to 860, corpora 386 to 394 / 385 to 391; without it
pdf.js 827 to 856 / 830 to 859, corpora unchanged 386 to 394 / 385 to 391; `as-shown` identical in all
four sections. Freed by name: 19 + 8 and 19 + 6 with the pack, 29 + 8 and 29 + 6 without. None newly
refused by this change. veraPDF 1.30.3 passed all 64 freed outputs at 2b and all 60 at part 4 (both arms); every
output names `page-as-image` in its history. Still refused by the photograph's own gates: 16
documents the reader does not draw whole, 7 with a structure tree (Q707), 1 past the pixel bound.

**Found.** 31 letter pages of dense text cost 634 MiB peak and 47 s; a page at the per-page budget 1.7 GiB.

**Gates.** Export `fin4` (HEAD plus my paths, workers rebuilt): `cargo test -p pdf-transform --no-fail-fast` exit 0
(140 lib, 421 archive), clippy `-D warnings`, `fmt --check` and rustdoc `-D warnings` exit 0; the pack census
exit 0: every freed document and count as before, 3 corpora documents (027613, JA1-1996-PDF, JA3-1998-PDF)
refused alike under `as-shown` and with the earlier sources, so not this change's; veraPDF 27 of 27 at 2b and
25 of 25 at 4 pass; the refusal of 33 and 40 pages 0.3 s, 17 MiB. `--bin cited` and `quotations` exit 0;
`cross_check` exit 0. The worktree's clippy fails in siblings' `mod.rs` and `prepare.rs` only. `conformance`
exit 101: `evidence` on round 1843's record, not mine; `records` exit 0. `archive_corpus` width check exit 0.

**Unfinished.** Structure-tree and optional-content conditions kept (Q707); object-level R03 findings,
the MediaBox row and the transfer function are not photographed. First session 3720 s, about 150 tool
uses; this one 7200 s, about 140. **Validation.** An Opus validator failed the first reading on three groups — the wide gate admitted four of this reader's own shortfalls as the document's departures, the page bound's refusal came only after thirty-one pages were drawn, and three comments and a corpus check were loose — and passed the round after it fixed all three, its hostile forty-page plant refused in a fifth of a second and its unembedded-font probe converted with both losses named.
