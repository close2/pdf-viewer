# Q694 — May three reopenings change what an earlier answer of yours allowed: the photograph's gate, the press, and A48's line?

Asked by round 1828, the review slot of batch 123 (ADR 2490), under your answer A681 item 1 ("and
yes open the blocks"). The list behind every figure is `doc/pdfa-blocking-issues.md`; the figures
are documents freed under `as-if-printed`, at 2b / part 4, each given the remedies ranked above it.
ADR 2490 decides the reopenings that change no earlier answer (R02 `image-smoothing` first); these
three would each change one, so they are yours. Nothing below is built.

**1. The photograph's gate (A570, ADR 2394 item 2).** Today a page is photographed only where its
text was never machine-readable: the gate refuses a page whose readback names any character. Two
remedies would widen it under `as-if-printed` to *any* page holding the fault: R08, an unembedded
font no route of this tree carries (39 documents; frees 27 / 30), and R03, a content-stream fault the
target refuses — an operator the base standard does not define, a UTF-8-invalid name, a box or a
width part 2 will not take (52 / 54 documents; frees 30 / 29). The loss is `text-as-image` for that
page alone, its text named in the report; the other pages keep their text. It is what a company
does when it prints and scans the letter. `as-shown` keeps the gate as it stands.

*Recommendation*: **yes, under `as-if-printed` only**, and at 2b and every part 4 flavour as ADR
2394 offers it today; 2u and 2a keep the refusal, since those levels exist to keep a page's text.

**2. The press (ADR 2114 section 4, ADR 2130).** At part 4, 29 documents draw `DeviceCMYK` beside
an RGB intent (frees 25; every one converts at 2b). ADR 2114 section 4 refused a shipped
`/DefaultCMYK` because this reader draws `DeviceCMYK` through its own assumed press, which no ICC
profile states. Two answers: (a) under `as-if-printed`, the shipped press profile becomes the
default when the operator names none, loss `press-assumed`, the profile named in the report and in
`xmpMM:History` — the output describes a print on that press; (b) under `as-shown`, this reader's
assumed press is written as an ICC CMYK profile computed from `pdf_colour`'s own conversion, so the
file states what the window showed — which states this reader's choice in the file, and needs an
ICC table writer this tree does not have.

*Recommendation*: **(a) yes; (b) yes, after the table writer is built**, as its own round. Until
then the part 4 documents can take 2b by R06b, which ADR 2490 decides.

**3. A48's line ("state an interpretation the standard defines; never fill in an absence").** R11:
a TrueType code reaches its glyph by a route ISO 19005-2 section 6.2.11.6 does not admit (30 / 28
documents; frees 23 / 20); the remedy rewrites the embedded program's `cmap` and the encoding so the
admitted route reaches the glyph this reader drew. The route this reader took is the one §9.6.5.4
leaves to the processor, so the output would state this reader's choice as the file's. R03a: a code
reaches `.notdef` in the embedded program (30 documents; frees 23 / 23); the remedy gives the
program a glyph for that code whose outline is its own `.notdef` outline, which meets ISO 19005-2
section 6.2.11.8's letter while drawing what this reader drew.

*Recommendation*: **R11 yes under `as-shown`**, where the window's drawing is the yardstick, and
under `as-if-printed`, where it is what the print showed; **R03a yes**, with item 1's photograph as
the alternative if you would rather the section's purpose than its letter.

**Meanwhile**: the tree refuses all of these as it does today, under their present sentences; batch
124 builds the rows ADR 2490 decides — R02, R06b, R03a (unless your answer to item 3 prefers the
photograph), R04 and R01, in that order — and an answered item takes its rank's place.
