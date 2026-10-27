# 2481 — A signed JPEG 2000 component keeps its sign statement, and its image is written back as JPEG 2000

Session 1823, a "clause" round of batch one hundred and twenty-three, 2026-10-27.
Status: implemented — a cleared `JPXDecode` image with a component whose `Ssiz` states a sign
goes back as JPEG 2000 with the sign bit kept, at every depth; an image of such a component at
sixteen bits or fewer is no longer written under `FlateDecode`.
Context: ADR 2470 and ADR 2480 (the writer and its file), ADR 1371 (the codec's output is the
image's samples), ADR 0187 (how the texts are cited).

## 1. The premise, and what was found

Round 1818 suspected that the decoder's clamp into the depth's range lost a signed component's
negative samples on redaction. It does not. The decoder (`hayro-jpeg2000`) adds 2^(p−1) to every
component whatever its `Ssiz` says, so a signed value `s` is delivered as the integer
`s + 2^(p−1)` in 0 to 2^p − 1, and flipping the sign bit of a written file changed no delivered
integer. The negative samples survive; **what redaction lost is the statement that the component
was signed**, because the output was always written unsigned.

## 2. The reading of the specification

ISO/IEC 15444-1 Table A-11 gives `Ssiz` a sign bit, and G.1.2 shifts the DC level back for an
unsigned component only. ISO 32000-2 §8.9.5.2 has an image's components "initially decomposed
into integers in the domain 0 to 2 n -1", and neither it nor its neighbours §7.4.8 and §7.4.10
define a map from a signed value into that domain. That is a silence of the specification, read
against the titles around §7.4.9, and the tree's offset — the decoder's — is a documented choice,
not a derivation. The consequence is chosen the least invasive way: the integers stay the
domain's, and the sign statement is written back.

A signed component's coded value is `s`, which is what an unsigned plane's integer less its DC
shift is, so a plane flagged signed has the coded bytes of the unsigned plane and only its `Ssiz`
(and `BPC`, in the `ihdr` and `bpcc` boxes) sign bit differs. `pdf_jpx_write::file` takes the
flags; the test that pins this in the writer compares the two files and finds four bytes
different.

## 3. The decision

- `OwnRaster` carries the flags, colour components first and the opacity channel last, read from
  the source codestream's `Ssiz`. `written_as_jpx` is "deeper than sixteen bits, or any component
  signed", and `deep::encode` passes the flags to the writer.
- **A change of output form.** A signed image of sixteen bits or fewer was written under
  `FlateDecode` and is now written as JPEG 2000. No other filter's samples state a sign, so a
  `FlateDecode` stream would lose the statement; the cost is a codestream where a deflate stream
  was, and a `/Decode` pair per component at the component's own depth.
- A signed component of an image whose codestream states other components than the raster has
  (a palette's, an opacity channel the dictionary does not ask for) cannot be matched to a plane.
  Past sixteen bits the page is refused by name; at sixteen bits or fewer it is cleared under
  `FlateDecode` without the sign statement, as every such page was before, and so is a signed
  image whose colour box cannot be written (ADR 2480). That is a sign statement not kept, priced
  here, in preference to a page that was cleared and is refused.
- The cleared region is the domain's zero, which for a signed component is its most negative
  value, as black is for an unsigned one: the redaction destroys the samples and states no value.

## 4. Proof

`a_signed_jpx_component_keeps_its_sign_statement_through_a_redaction` at twelve-bit grey and
twenty-bit RGB with mixed signs. Planted: the flags dropped in `deep::encode` and the signed arm
removed from `written_as_jpx` each turn it red, and are put back.
