# 1913 — An `ICCBased` image already converts each distinct sample about once, and what is left is the conversion

Session 1538. Status: **accepted**, nothing built. Context: ADRs 1469, 1900 §3 and §5; ISO 32000-2
§8.6.5.5, §10.4.2.

## 1. The question

ADR 1900 §3 counted `WTPDF`'s page 1 at 1 dpi, on one thread, at 97.2 M instructions, 60.2 M of
them in `convert_three` for one 350 × 350 `ICCBased` JPEG, and the brief proposed converting each
distinct sample once, byte-identically, on the hypothesis that a photograph has far fewer distinct
samples than pixels and the cache would halve the read.

## 2. What was measured

HEAD `512af7e5`'s release `quorra-transform render --pages 1 --dpi 1`, `RAYON_NUM_THREADS=1`, under
callgrind: **98 786 265 instructions; `convert_three` 60 243 703 inclusive, of which 49 933 476 are
50 682 calls of `Compositing::paint`, 985 instructions a conversion** (`icc::Profile::to_xyz_with`
26.0 M, `xyz_d50_to_srgb` 15.3 M). The image has **122 500 pixels and 44 328 distinct samples**
(decoded by PIL; 21 509 pixels equal the one before). ADR 1469's memo — a direct-mapped table of
2^15 entries for this image, behind a same-as-the-last-pixel test — was simulated on those samples
with its own hash: 50 754 conversions at 2^15 (callgrind: 50 682), 47 503 at 2^16, 45 695 at 2^17,
45 073 at 2^18, and 44 328 with an exact cache.

## 3. Decision

**The hypothesis does not hold and nothing is built.** The memo already converts each distinct
sample about once: an exact cache could save at most 6 354 conversions, 6.3 M instructions or
6.4 % of the page, before the price of its own probe on each of the 100 991 pixels that reach the
memo. **The 44 328 conversions that remain are 43.7 M instructions of the conversion itself**, in
`pdf-colour`. The lever ADR 1900 §5 priced at about 50 M — PdfCraft's one 8-bit transform — is a
different computation, tables built once at eight-bit precision, whose bytes are not this tree's
`f32` route's: a change of what is drawn against the processor oracle, owed its own ADR and the six
arms, and ADR 1900 §5 has said no window figure would move. **`launch_path`'s `open_kinstructions`
is not where this cost lives**: the gate counts an open (`WTPDF`'s band is 12 624 to 13 316
thousand instructions), and the conversion is page 1's draw, inside `first_page_ms`.
