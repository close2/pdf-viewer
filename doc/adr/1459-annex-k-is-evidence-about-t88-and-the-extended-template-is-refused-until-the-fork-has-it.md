# 1459 — Annex K is evidence about T.88, not T.88, and the extended template is refused until the fork has it

Session 1312. Status: **accepted**; the in-tree half **built**, the codec half **written and not
applied** — it is the owner's to apply to the fork, as ADR 1447's was.
Context: `crates/pdf-sandbox/tests/t88_conformance.rs` (new), `crates/pdf-sandbox/src/decode.rs`
(`extended_template_region`, `segment_header`), `doc/patches/hayro-jbig2-extended-template.patch`
(new), `doc/third-party-data.md`, `doc/questions/Q209`.
Clauses: ISO 32000-2 §7.4.7; ITU-T T.88 (08/2018) sections 6.2.5.3 to 6.2.5.7, 6.4.9, 6.5.5,
6.5.9, 6.5.10, 7.4.1.5, 7.4.3.1.7, 7.4.3.2, 7.4.6.2, 7.4.6.3, Annexes H.1 and K, cited and
paraphrased (ITU reserves its rights; `doc/third-party-data.md`).

## 1. The premise, and what the data turned out to be

ADR 1447 left "of T.88's ten Annex K conformance streams, the pinned codec decodes two" as evidence
of defects in the fork. Annex K says less than that sentence assumed. It is **informative** — its
own title line says it is not an integral part of the Recommendation — and T.88 Table K.1 states only
that the sample software's reconstruction of each stream equals the original image. The references
were drawn by the sample decoder from streams the sample encoder wrote. So a disagreement between
this tree and a reference is a question for T.88's clauses (principle 5), not a defect by
definition, and the round read each one against the clause before writing a patch.

## 2. The inventory

Each stream reaches `pdf-sandbox`'s filter in §7.4.7's embedded organisation (file header, end of
page and end of file dropped, page association 1, page-0 segments as `/JBIG2Globals`, one image per
page) under both isolations, which agree on every case. "Pinned" is `64efcaca`.

| T.88 Table K.1 item, stream | exercises | pinned codec | verdict |
|---|---|---|---|
| 1 `codeStreamTest1_TT1`, 3 pages | Annex H.1: Huffman, MMR, halftone, then arithmetic, then refinement/aggregation | pages 2, 3 match; page 1 refused | **the file departs from Annex H.1** at two bytes (below) |
| 2 `codeStreamTest1_TT2` | Huffman symbol dictionary | refused | departs: 6.5.9, 6.5.10, 7.4.3.2 |
| 3 `codeStreamTest1_TT3` | arithmetic symbol dictionary | refused | departs: 6.5.10, 7.4.3.2 |
| 4 `codeStreamTest1_TT4` | generic region, template 1 | refused | departs: 6.5.10, 7.4.3.2 |
| 5 `codeStreamTest1_TT5` | refinement in a symbol dictionary | refused | departs: 6.5.5, 6.5.10, 7.4.3.2 |
| 6 `codeStreamTest2_TT6` | refinement in a text region | refused | departs: 6.4.9, 6.5.10, 7.4.3.2 |
| 7 `codeStreamTest1_TT7` | generic region, extended template | refused | departs: 6.5.10, 7.4.3.2; **and the codec's defect** (section 3) |
| 8 `codeStreamTest3_TT8` | coloured text region | refused | outside §7.4.7, which requires COLEXTFLAG 0 |
| 9 `F01_200_TT9` | one MMR generic region | **matches** | — |
| 10 `F01_200_TT10` | one arithmetic generic region | **matches** | — |

The departures, each read in the clause and confirmed in the sample software's source (read as
evidence; nothing of it is in the tree):

- **Item 1 is Annex H.1's datastream but for bytes `0x9B`–`0x9C`**: the file holds `11 00` where
  Annex H.1 prints `01 10`. These open the text region's symbol ID Huffman table (7.4.3.1.7); as
  filed they give the third symbol no code at all, and Annex H.1's own walk through the segment
  decodes its bytes to code lengths 2, 2 and 1. With Annex H.1's bytes all three pages match their
  references bit for bit through the pinned codec — T.88's own worked example, Huffman, MMR,
  halftone, pattern dictionary, refinement and aggregation included.
- **6.5.10, items 2 to 8**: the export runs are two zeros, which never reach SDNUMINSYMS +
  SDNUMNEWSYMS; the sample encoder writes exactly two zero runs and its decoder reads two values
  and exports every new symbol.
- **7.4.3.2 and T.88 Table 34, items 2 to 8**: SBSYMS is the symbols of the dictionaries the text region
  *refers to*, and these text regions refer to none; the sample decoder uses every dictionary
  before them.
- **6.5.9, item 2**: SDHUFFBMSIZE is not the collective bitmap's byte count, and each symbol is
  MMR-coded on its own instead of the height class's collective bitmap.
- **6.5.5 step 4, item 5**: no OOB ends the refinement/aggregate height class (the encoder's own
  comment says it leaves it out), and the dictionary refers to none yet exports three symbols where
  it defines one.
- **6.4.9, item 6**: no instance T is coded although SBSTRIPS is 4; the sample software skips IAIT
  whenever SBREFINE is 1.
- **7.4.1.5 NOTE 3, item 8**: a COLEXTFLAG region shall combine with REPLACE, and this one says OR.

A conforming decoder refuses these streams, and the pinned codec does. **The fork owes nothing
for them**, and no patch is written for them: a codec that matched them would have grown the
sample software's habits. To learn what the codec does with the parts that *do* conform, a scratch
copy was taught the sample's conventions and nothing else: items 3, 4 and 7 then match their
references, and items 2, 5 and 6 stop on further departures of the same kind.

## 3. The one defect in the codec: the extended template

T.88 7.4.6.2 bit 4, EXTTEMPLATE, gives a template-0 arithmetic generic region twelve adaptive
pixels (6.2.5.3 Figure 3(b), 6.2.5.4, T.88 Table 5) and an AT field of twelve coordinate pairs (7.4.6.3
Figure 50). The pinned codec parses the flag and ignores it: it reads four pairs, starts the
arithmetic decoder sixteen bytes early, and **draws a wrong bitmap without a refusal**. With item
7's other departures set aside, its page differs from the reference in 1 746 pixels; the
conforming part (its page information and generic region alone, held to the reference outside the
text region's rectangle) differs in 1 778.

`doc/patches/hayro-jbig2-extended-template.patch` implements it, against `64efcaca`, independent of
ADR 1447's patch. Two readings it rests on:

- **The field is 24 bytes.** 7.4.6.3 calls it a 32-byte field, but its byte list and Figure 50
  both end at GBATY12, byte 23, and item 7 carries 24. The prose number is contradicted twice by
  the clause itself, so the enumeration is taken.
- **The context order.** 6.2.5.7 leaves the gathering order free provided it does not depend on
  where the AT pixels are. Figure 3(b)'s sixteen nominal positions are Figure 3(a)'s sixteen, so
  each extended pixel takes the bit the codec's template 0 gives that nominal position; a nominal
  placement then produces template 0's context, and Figure 8's SLTP context is the same value for
  both.

A pixel at a time through `Bitmap::get_pixel`, deliberately: the census below finds no real stream
using it, so a word-at-a-time gatherer would be speculative optimisation of code nobody measured.
Applied to a scratch copy: item 7's conforming part matches, every other outcome is unchanged, the
codec's unit tests pass, and its Clippy findings are the pinned tree's own.

## 4. In the tree: refused out loud until the fork has it

A silent wrong picture is the one outcome principle 1 forbids, so `decode::jbig2` refuses an image
whose streams hold a template-0 arithmetic generic region with EXTTEMPLATE set, naming the segment
(`extended_template_region`, on the segment-header walk `whole_segments` already does). Nothing
else changes; the census says no corpus image is refused by it. **It is to be deleted the day the
fork takes the patch, and the test says when**: for that case it asks `hayro_jbig2` directly as
well as the filter, and the codec matching the reference fails the test with that instruction.

## 5. The test, and where the data lives

`t88_conformance.rs` holds every page of every stream to one of four outcomes — matches, departs
(clause named; a match fails, because it would mean the sample's habits), waits on a patch (refused
by the filter; the codec matching fails), outside §7.4.7 — plus Annex H.1's page 1 and the
conforming generic regions of items 3, 4 and 7. It prints the table and the count of streams that
decode on every page; the ledger points at the printout rather than repeating it.

The data is **not committed**. Its notice licenses reproduction and distribution for including,
evaluating and conformance-testing an implementation of the Recommendation — narrower than the
Apache-2.0 terms the repository is offered under. The test reads `$T88_CONFORMANCE_DATA`, or the
directory the zip was unpacked to under `/home/AI/specs/T.88/`, and prints a sentence and passes
where neither exists (ADR 1154's shape). Whether to commit it beside its notice is the owner's
question, `doc/questions/Q209`.

## 6. The census, and the urgency it sets

One walk behind the lock over every JBIG2 stream the corpus holds — the same **54 498** ADR 1447
counted, extracted by `image_codec_seeds` with no size ceiling and read by a scratch walker over
segment headers and flags only (22 end inside a header and are not counted):

| feature Annex K exercises | streams using it |
|---|---|
| generic region, extended template (EXTTEMPLATE) | **0** |
| COLEXTFLAG on any region, or a colour palette segment | **0** |
| a text region referring to no segment | **0** |
| generic region, template 0 / template 3 / MMR | 30 756 / 81 / 1 |
| TPGDON | 6 787 |
| symbol dictionary, Huffman / refinement-aggregate | 32 / 8 305 |
| text region, Huffman / refinement | 31 / 13 655 |
| pattern dictionary and halftone region | 40 |
| striped page with end-of-stripe segments | 10 183 |

**No real stream uses the one feature the codec lacks**, so the refusal of section 4 refuses no
corpus image (`raster_golden` holds all its pages unmoved) and the patch is owed for conformance,
not for any document on this disk: low urgency, and no reason to hurry the fork. Nor does any real
stream share the sample encoder's departures, so nothing argues for a reader tolerating them.

## 7. What it means for §7.4.7

§7.4.7's filter decodes ISO/IEC 14492:2019 "excluding colour palette coding". The extended template
is not colour, so one requirement of the clause is not executed — formerly in silence, now
reported. The row is therefore `partial`, not `implemented`, until the fork takes the patch and the
refusal is deleted. Colour (item 8) is the clause's own exclusion and stays outside; nothing in
Annex K's evidence shows another feature of 14492 the codec lacks.
