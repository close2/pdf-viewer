# 1312 — Annex K is evidence, and the extended template is refused until the fork has it

Robustness slot of batch forty-eight. ADR [1459](../adr/1459-annex-k-is-evidence-about-t88-and-the-extended-template-is-refused-until-the-fork-has-it.md). Question `Q209`.

**The data.** T.88's zip re-fetched (hash as recorded), kept at `/home/AI/specs/T.88-201808.zip`,
unpacked under `/home/AI/specs/T.88/`. Annex K is informative; its references were drawn by the
sample decoder from streams the sample encoder wrote, and both were read as evidence.

**The premise was wrong.** ADR 1447 counted eight refused streams as evidence of codec defects.
Read against the clauses, seven of the ten streams depart from T.88 — export runs of two zeros
(6.5.10), text regions referring to no dictionary (7.4.3.2), a per-symbol MMR with a wrong
SDHUFFBMSIZE (6.5.9), a height class with no OOB (6.5.5), no IAIT under SBREFINE (6.4.9) — and
the eighth is colour, which §7.4.7 excludes. Item 1 is Annex H.1's datastream but for two bytes
at `0x9B`; with Annex H.1's bytes all three pages match through the pinned codec.

**The one real defect**: EXTTEMPLATE (7.4.6.2 bit 4) is parsed and ignored, so an extended-template
generic region is drawn wrongly without a word (1 746 pixels on item 7 with its other departures
set aside). `doc/patches/hayro-jbig2-extended-template.patch` implements it against `64efcaca`
(24-byte AT field, as the clause's byte list and Figure 50 say against its "32"; context bits by
nominal position so Figure 8's SLTP context is template 0's); `git apply --check` clean alone and
beside ADR 1447's. On a scratch copy item 7's conforming part matches and nothing else moves.

**In the tree**: `decode::jbig2` refuses such an image by segment (`extended_template_region`),
until the fork takes the patch. `tests/t88_conformance.rs` holds every page to matches / departs
(clause named) / waits on a patch / outside §7.4.7, decodes under both isolations, asks the codec
directly for the waiting case, and prints the count. Run against the patched scratch codec it fails
with the instruction to delete the refusal — checked.

**Census** (one walk behind the lock, the 54 498 corpus streams, header flags only): **0** use
EXTTEMPLATE, 0 colour, 0 an unreferred text region; refinement in 13 655 text regions, Huffman
symbol dictionaries in 32, halftones in 40 — all features Annex H.1 shows the codec decodes.

**Ledger**: §7.4.7 `implemented` → `partial` (text sent to 1313; the count is the test's).
Gates: rustfmt on my files; clippy `-D warnings` on `pdf-sandbox` clean (`pdf-model`'s fails in a
sibling's `image/mcu_rows.rs`); `pdf-sandbox` tests and `pdf-model` nextest (1728) pass;
`cargo test -p conformance` passes. Behind the lock: `raster_golden`
(974 held, 0 moved) and `pdf-model --test corpus` (59 of ceiling 59), exit 0; the conformance
test prints 2 of 10 streams decoding on every page.
Left: the patch, for the owner; Annex H.2's arithmetic-coder test sequence is not yet a fixture.
