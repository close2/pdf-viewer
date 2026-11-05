# Q780 — Which press does an archive state for DeviceCMYK the file never characterised?

Source: round 1937 (single round), ADR 2691, section 8 of `doc/pdfa-blocking-issues.md`.

## What the round found

Round 1936's outputs of `postscript_type4_many_outputs` and `issue9940` draw differently from
their sources: every pixel of the first, by up to 76 of 255, with a visible edge where the cyan
leaves the screen's gamut; 5 172 pixels of the second, by up to 7. Removing the output's
`/OutputIntents` alone makes each output draw pixel-identically to its source. So the cause is the
output intent, and nothing else the conversion did.

Both files paint in `DeviceCMYK` and state no profile. ISO 19005-2 and ISO 19005-4 section 6.2.4.3
admit `DeviceCMYK` only through a CMYK output intent or a device-independent `DefaultCMYK`.
ADR 1153 (the owner: "perfect. use it.") writes the GRACoL press this program ships as the
intent. This renderer drew the source through its own assumed process inks (ADR 0263), and no ICC
profile states those. So every output that gains the shipped CMYK intent draws its `DeviceCMYK`
through a different press than its source did. The report also claimed that "this renderer already
showed those colours that way". That sentence was false, and round 1937 corrects it (ADR 2691). In
round 1936's censuses, 141 shipped-profile outputs (52 documents) and 105 stating-profile outputs
(34 documents) carry the shipped press, counted as the census outputs whose catalog names the
profile in its `/Info`.

ADR 2518 already treats the same press as a named loss, `press-assumed`, when it is written as a
`DefaultCMYK` beside an RGB intent: "a statement the file's own bytes do not make". As an output
intent, it is still `Stated` under every profile. The two answers do not agree.

## The question

What should an archive state for `DeviceCMYK` that its source never characterised?

- **a.** Keep the shipped press as a statement, under every profile, with the corrected sentence
  saying that the drawing moves. **This is what the tree does now.**
- **b.** Make the shipped press as an output intent the loss `press-assumed`, as ADR 2518 does
  for the default. Only `as-if-printed` authorises it. Under the three other profiles, the
  documents above are refused unless the operator supplies `--output-intent-profile`.
- **c.** Write a destination profile that states this renderer's assumed inks: an output-class
  CMYK ICC profile sampled from `pdf_colour::assumed_press`, with its inverse. The archive would
  then draw as its source did, and another reader would draw it the same way. The profile names
  no real press, and its description would say that. It needs an ICC edition that ISO 19005-2
  section 6.2.4.2 admits, and veraPDF would have to accept it.

## Recommendation

**c** for `as-shown`, `no-data-loss` and `state-every-loss`, and **a** for `as-if-printed`, whose
reading is a printed and scanned page, so a real press is the honest statement there. **c** keeps
what the reader saw without inventing a mark. **b** loses 52 documents under three profiles to
protect the same thing.

## What the tree does meanwhile

**a**. The report's sentence for a CMYK intent this conversion adds now says that this renderer
drew those colours through its assumed inks, and that they now draw differently
(`OUTPUT_INTENT_STATES_THE_SHIPPED_PRESS` and `_SUPPLIED_PRESS` in
`crates/pdf-transform/src/archive/decision/colour_and_graphics.rs`).
