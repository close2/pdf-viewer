# 1401 — The incomplete population is held by name, and each name was read against its clause

Session 1282. Status: accepted and **built**.
Context: `crates/pdf-model/tests/corpus.rs` (`INCOMPLETE`, `whose_defect`,
`the_corpus_opens_interprets_and_rasterises`).
Amends: ADR 1081's consequence that left `MAX_INCOMPLETE` a count, on the ground that the run
already prints its members' classification in full. Builds on: ADRs 0433, 0730, 1075.

## 1. Why the count was not enough

The run prints the composition, and nothing asserts it. A document that stops reporting while
another starts leaves 61 at 61; a document whose deciding report moves from the file's column to
this reader's leaves every count where it was. Both are findings, and ADR 1081's own argument for
`LOCKED`, `UNREADABLE_ENCRYPTION` and `PAGELESS` — a ceiling cannot tell a swap from nothing — is
the argument here. `INCOMPLETE: [&str; 61]` is held by `gate_ratchet::population`, in both
directions, with a comment beside each name or group saying the clause its report rests on.

## 2. What the sixty-one are, read one by one

Twenty were opened (`qpdf --show-object`, `mutool trace`, the font program under fontTools where
a glyph was in question) and read against the clause the report names. The partition:

| deciding mechanism | documents | clause |
|---|---|---|
| Identity-H over a non-embedded `CIDFontType2`, no usable `/ToUnicode` | 18 | §9.7.5.2, §9.7.4.2, §9.10.3 |
| a token neither operand nor defined operator; operand shortfall | 13 | §7.2.3, §7.8.2, §8.5.4, §9.4.1 |
| no `/MediaBox`, or one enclosing no area | 3 | §7.7.3.4, §7.9.5 |
| a resource name the current resource dictionary does not define | 3 | §7.8.3, §7.7.3.4, §7.5.7 |
| an embedded font program its filter reports damaged; `/DescendantFonts [null]` | 4 | §7.4.1, §9.7.6.1 |
| an image whose dictionary and data disagree | 5 | §7.4.8, §8.9.6.3, Table 87, §7.4.7 |
| an annotation its clause cannot draw | 5 | Table 166, Table 93, §12.7.4.3 |
| `/Contents` under `JBIG2Decode`; a page object that stops part-way | 2 | §7.4.7, §7.3.7 |
| **the file's** | **53** | |
| a font program with no outline for any code shown | 4 | ADR 0270, §9.8.2, §9.6.5.4 |
| a form or glyph that invokes itself, stopped at `MAX_FORM_DEPTH` | 3 | a bound of ours |
| **neither one** | **7** | |
| a stand-in `/DA` face with no Arabic glyphs | 1 | `doc/todo/22`, ADR 0348 |
| **this reader** | **1** | |

**No reason was stale.** None of the sixty-one rests on anything built since it joined: no CCITT,
JPEG 2000, encryption-handler or machine-face refusal is among them (those populations are elsewhere
or empty). **No hidden debt was found in the file's column** — the likeliest places were checked:
`issue6541.pdf`'s missing `/R41` is a pattern that states its own `/Resources`, which §7.8.3's
inheritance does not reach; `issue5954.pdf`'s `/F1` is on the parent while the page states its own
`/Resources`, which §7.7.3.4 uses "in its entirety"; `issue20232.pdf`'s blank ⌀ is §9.8.2's
Symbolic flag read as the clause says.

## 3. What is left

The one debt is ADR 0348's list for `freetext_no_appearance.pdf` — an Arabic glyph source,
joining-form selection and right-to-left order, together or not at all — which is a contract, not
a round's small construction. A change that makes any listed document draw completely now fails
the gate by name, and the name leaves the list with its reason in the same change.
