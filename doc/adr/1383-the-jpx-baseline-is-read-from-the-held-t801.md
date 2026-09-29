# 1383 — The JPX baseline is read from the held T.801

Status: accepted and **built**.
Context: the owner's answer `doc/questions/A169` (read in the main checkout); ISO 32000-2 §7.4.9;
ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3; ITU-T T.801 (08/2002), held as
`doc/T-REC-T.801-200208.pdf` and prepared as `doc/md/T.801.md`. Code: `crates/pdf-model/src/jpeg2000.rs`
(`FileType`, `FragmentList`, `FirstLayer`, `Collection`, `Headers::keeping_colour`,
`Codestream::required_extensions`), `crates/pdf-model/src/image.rs` (`jpx_colour_choice`,
`jpx_drawn`, `JpxSpace::Lab`), `crates/pdf-archive/src/table/graphics.rs`
(`jpeg2000_uses_the_baseline_feature_set`), `crates/pdf-transform/src/archive/decision.rs`
(`JPEG2000_BASELINE_NOT_REACHED`), `crates/pdf-transform/src/redact.rs` (the `Lab` arm).
Amends ADR 0928 (the row it kept `Unchecked` is checked) and ADR 1184 only in naming the text.
T.801 is cited by clause and paraphrased, never quoted (`doc/third-party-data.md`).

## 1. The text, and one premise that did not hold as stated

T.801 is the identical joint text of ISO/IEC 15444-2. Its title pages give the 08/2002 edition,
copyright 2003, running heads ISO/IEC 15444-2:2003 (E). **They do not say Amendment 1 and
Corrigenda 1 and 2 are integrated**, which Q169 asserted; ITU's catalogue page does, and lists
three later corrigenda and amendments and the 2021 and 2023 editions after it. ISO 19005-2 cites
ISO/IEC 15444-2:2004. The revisit condition is the owner's: that ISO edition, if the editions are
ever found to differ on M.9.2's set. None of the later texts has been read.

## 2. M.9.2, read alone

M.9.2 defines a baseline *file* in nine subclauses: JPEG 2000 compression only (M.9.2.1); the
first compositing layer one codestream, the file's first (M.9.2.2); no codestream extension
required but an irreversible array-based decorrelation — one collection, one stage — and the
non-linear point transformation (M.9.2.3); a colour specification from a list, and one at an
approximation of 3 or better (M.9.2.4); fragments inside the file, in order (M.9.2.5, M.9.2.6);
the JP2 Header box first (M.9.2.7); opacity a reader shall interpret (M.9.2.8); other data that
does not change the picture (M.9.2.9). M.9.2.4's list: the enumerated sRGB (16), sRGB-grey (17),
sYCC (18), e-sRGB (20), ROMM-RGB (21), e-sYCC (24), CIE Lab (14) and CIE Jab (19) each with default
or enumerated parameters, and the Restricted and Any ICC methods. §7.4.9 adds CMYK (12) and takes
19 out of what PDF data may use.

## 3. The reader: §7.4.9's choice is made here, not by the codec

§7.4.9: "PDF processors shall support the JPX baseline set of enumerated colour spaces". The
codec read the first `colr` box only, drew CIE Lab as one flat colour through an abstract profile
§8.6.5.5 admits for no space, applied its sYCC and Lab conversions under a stated `/ColorSpace`,
and refused an image outright over a code it lacked. Now `jpx_colour_choice` decides and hands the
codec the decision as data: set-aside boxes become T.801 M.11.20 Free boxes, which move no byte. A
stated `/ColorSpace` sets all of them aside. Otherwise precedence, then approximation (0 last),
then file order rank them, and the first this tree draws is kept, an Any ICC profile read as the
restricted method's. None drawn leaves the codec's channel-count fallback, which is §7.4.9's own
last sentence. **Drawn as defined**: 12, 16, 17, 18, 21, and 14 under D50 as §8.6.5.4's `Lab`.
**Fallback, named**: 20 and 24 (PIMA 7667), 19 (CIE Publication 131), 14 under another illuminant.
Those texts are not held; that is the row's second residue beside the thirteen codestreams, so
§7.4.9 stays `partial`.

## 4. The validator: every subclause a file can break

`graphics/jpeg2000-uses-the-baseline-feature-set` is `Implemented`: M.9.2.1 to M.9.2.7, each
finding naming its subclause and M.9.2.4's naming every stated space and whether the list holds
it. Four readings are decisions. The rule binds the *data* whether or not the dictionary states
`/ColorSpace`. A bare codestream fails, because M.9.2 defines a file and §7.4.9's filter expects a
full JPX structure. The `jpxb` compatibility code is a declaration, not a feature used — a plain
JP2 file never states it. CIE Jab stays on this row's list because its own row reports it. Not
read: M.9.2.6's fragments-before-the-codestream order and M.9.2.7's second sentence. The veraPDF
corpus's seven 6.2.8.3 witnesses agree, `over` 0.

## 5. The converter refuses by name

Where the dictionary states `/ColorSpace`, ADR 1371's domain is the dictionary's and transcoding
to `FlateDecode` would keep it — owed, not built. Where it does not, an off-list space is what the
samples mean and writing a baseline code relabels them; a required extension is what decoding
needs. `doc/pdf-a-mitigations.md` section 4.5 carries the entry.
