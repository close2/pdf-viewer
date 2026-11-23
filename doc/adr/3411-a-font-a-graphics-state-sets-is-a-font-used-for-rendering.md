# 3411 — A font a graphics state sets is a font used for rendering

Session 2177. Status: **accepted** and built. Context: ADR 3376 item 4 (`extgstate`); ISO 32000-2
§8.4.5 Table 57, §9.3.1; ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1.

## Why this validator passed it

pdf.js `extgstate.pdf` sets its text font with `gs`, whose graphics state states `/Font [11 0 R
36]`, an unembedded Courier. Table 57 makes that array the equivalent of `Tf`'s operands, with the
font given by reference. The survey's font population was the fonts a `Tf` named, so the font the
text is drawn in was not in it and `font-programs-embedded` never asked about it; veraPDF fails the
output on 6.2.11.4.1 and 6.2.10.4.1.

## Decision

The survey's graphics state carries the font a `gs` sets, with the graphics state resource's name;
a `Tf` clears it and a `gs` stating `/Font` clears the `Tf` name. A text-showing operator under it
records the font in the survey's population under the name `ExtGState /<name>`, keyed by its object
where it is one. Every font row and the substitution then read it as they read a `Tf` font: the
substitution writes the descriptor and program into the font's own object. A font written directly
inside the graphics state is recorded too; the substitution cannot place a written-out copy there,
so such a document is refused by the output's own re-check rather than written failing. A Type 3
font set this way is recorded but its glyph procedures are not walked from the `gs` (none in the
corpora).

## Measured

`extgstate` converts at 2b and 4 under both profiles with Courier given the shipped fixed-pitch
face, and veraPDF 1.30.3 passes all four outputs.
