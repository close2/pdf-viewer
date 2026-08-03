# ADR 0158 — A name that ends in the word "Symbol"

Status: accepted, 2026-08-03. Session 197. Found by opening the sixth page on the ambiguous
ranking, on the shape §3a says to prefer: 3.52 from the nearest reference and 3.55 from the
furthest, which is everybody-against-us.

## The page

`issue8697.pdf` is 250×50 points and its whole content stream shows one string:

```
BT 10 20 TD /F1 18 Tf [(W)-2(ha)-3(t )17(Op)-3(er)-3(ating)-2( )21(Sy)4(s)-3(tems)-3( )9(D)-3(o)] TJ ET
```

Four references draw *What Operating Systems Do*. **We drew a single `∝`**, with `1 command`
and `unsupported: []` — the shape trap 1 exists for.

The font is a `/TrueType` with no `/FontFile2`, so it is substituted. What it states about
itself is worth listing, because the file states the same thing three times:

| entry | value |
|---|---|
| `/BaseFont` | `/SegoeUISymbol` |
| `/Encoding` | `/WinAnsiEncoding` |
| `/Flags` | `32` — Table 121 bit 6, Nonsymbolic |
| `/Widths` | Helvetica's, from `/FirstChar 32` |

`SegoeUISymbol` is a sans-serif face whose *name* ends in the word. `substitute::family_of`
matched `folded.contains("symbol")` and returned `Family::Symbol` — the standard-14 font —
before it read anything else, and `substitute_code_table` then took the encoding from that
family, so code `0x57` meant `Omega` rather than `W`. Almost none of those Greek names resolved
in the face this machine offered for the request, and the one that did drew `∝`.

## What the clause determines

§9.6.5.4's opening sentence, which is a `shall` and states two conditions disjunctively:

> If the font has a named Encoding entry of either MacRomanEncoding or WinAnsiEncoding , or if
> the font descriptor's Nonsymbolic flag (see "Table 121 -Font flags") is set, the PDF processor
> shall create a table that maps from character codes to glyph names

This document satisfies both. The table is therefore the Latin one, and neither symbolic
standard-14 font has a glyph under a Latin name.

The statement is not peculiar to TrueType. §9.6.5.2 says of a Type 1 program that "An Encoding
entry in the PDF font dictionary, if present, shall override a Type 1 font's mapping from
character codes to character names", and Table 121's own prose says the Nonsymbolic flag means
"the font's character set is the Standard Latin character set (or a subset of it) and that it
uses the standard names for those glyphs". Both are about the *codes*, not about a container
format.

And §9.8.2 is the clause that permits a flag to decide a substitute at all, in the sentence that
names this exact use:

> This influences the font's default base encoding and may affect a PDF processor's font
> substitution strategies.

So the answer is determined, and it is determined *against the name*: a substring of a
`/BaseFont` is a guess about a face, and the encoding and the flag are statements about the
codes.

## The rule now

`substitute::states_latin_codes` reads §9.6.5.4's two conditions — the Nonsymbolic bit in the
descriptor, and a named `/Encoding` (or an encoding dictionary's `/BaseEncoding`) of
`MacRomanEncoding` or `WinAnsiEncoding` — and where either holds, `family_of` skips the two
name matches that select a symbolic standard-14 face. Everything after them is unchanged: the
name still chooses between serif, sans and monospace, then §9.8.3.2's PANOSE, then the flags.

`/Encoding` is read here rather than through `pdf_font::base_encoding` on purpose. This is a
question about what the document *said*, and a `/BaseEncoding` this crate does not implement —
`MacExpertEncoding` is refused — still says the codes are Latin.

**No exception for a `/BaseFont` spelled exactly `Symbol`.** The clause states none, and Table
121's "[t]his flag and the Symbolic flag shall not both be set or both be clear" makes bit 6 a
deliberate statement rather than a default somebody left alone. A file naming `/Symbol` and
flagging it nonsymbolic contradicts itself, and the clause says which half wins.

## What it cost and what it bought

- `issue8697.pdf` draws the sentence. Its distance from the nearest reference went **3.52 →
  0.21** bounds (and from the furthest, 3.55 → 0.99), its worst mean 14.18 → 4.97 and its SSIM
  0.6448 → 0.9551, so it leaves §3a's printed ranking altogether. It is **still `ambiguous`**,
  which is the honest end state: the face is a substitute, so its glyph shapes are this
  machine's rather than the document's, and no verdict can say otherwise. What changed is that
  the page says what it says.

  **Corrected in session 202**: this ADR and the group it added originally read the ink as
  "ours 9.22, `hayro` 9.08 │ `mupdf` 18.40, `ghostscript` 18.62, `poppler` 18.75" and concluded
  that three `libfreetype`-linked renderers were darkening stems. The instrument was halving the
  first two, because our artefacts and `hayro`'s carry an alpha channel and the measuring command
  was averaging it in. All five are within 0.6 of each other — ours **18.43**, `hayro` **18.16**,
  `mupdf` 18.40, `ghostscript` 18.62, `poppler` 18.75 — which is ink conserved and the difference
  confined to where the glyphs are. ADR 0163.
- The text gate reads it back at **100%**, up from below the 0.90 floor, and
  `TEXT_BELOW_FLOOR` goes 36 → 35. The corpus readback moved 22 849 → 22 852 of 23 269 words.
- Every other gate is unchanged: the corpus's 80 incomplete documents, the oracle's
  893/78/788, and `ambiguous_undiagnosed.txt` are identical.

## The finding that is not about fonts

`text_extraction.rs` had a paragraph on this document, and it was the only entry on that gate's
list described as "a question about a clause" rather than a defect. It said the file draws
`Ωηατ Οπερατινγ Σψστεµσ ∆ο` where `pdftotext` reads `What Operating Systems Do`; that this is
"a Symbol font whose glyphs are Greek and whose codes are Latin"; that §9.10.2's second method
therefore takes each code to the glyph's own name; and that "both readbacks are defensible".

Every sentence is true, and the conclusion is wrong, because all four are about the *readback*.
None of them asked why the Greek was on the page. The gate that could see the symptom reasoned
about its own half of the pipeline and closed the question one stage downstream of the defect —
which is trap 1 wearing a comment instead of a metric, and the second time in five sessions that
a claim about a picture has survived because no gate can check prose.

## Alternatives rejected

- **Fix `substitute_code_table` instead.** The encoding would then be Latin while the face
  searched for was still `Family::Symbol`'s — `StandardSymbolsPS` before any text face — so the
  codes would resolve to names the chosen face does not have. The family and the encoding are
  one decision and belong in one place.
- **Match the name more precisely — only `Symbol`, `SymbolMT`, `ZapfDingbats`.** That would fix
  this file and leave the rule resting on a list of spellings, which is the thing the clause
  exists to replace. It also gets `issue8697.pdf` right for a reason the file does not state.
- **Report rather than substitute.** Nothing here is unimplemented; the wrong answer was
  chosen from data the file supplies.
