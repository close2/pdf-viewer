# 2007 — An unreadable destination profile is written as sRGB, and a halftone's function goes as `/TR` does

Session 1585, the archive slot of batch ninety-three. Status: **accepted**, **built**. The two
`not-built-yet` sites the census ranks first at PDF/A-2b once it is run over `doc/veraPDF-corpus`
(no pdf.js or `doc/corpora` document reaches any `not-built-yet` site at 2b; ADR 1983's census).
Code: `crates/pdf-transform/src/archive/prepare.rs` (`ReplacedDestination`),
`crates/pdf-transform/src/archive/transfer.rs`.

## 1. `graphics/destination-profile-class-and-colour-space` (9 documents)

ISO 19005-2 and ISO 19005-4 section 6.2.3 admit an output or a monitor profile over grey, RGB or
CMYK. ISO 32000-2 §14.11.5: "The data in an output intent dictionary shall be for informational
purposes only, and PDF processors are free to disregard it." This reader takes the first entry
whose profile `Profile::parse` accepts and disregards the rest, so a file whose only profile it
cannot parse was drawn as though it stated no intent. Three answers follow, decided by the file:

- **Not drawn through, `/N 3`**: the shipped sRGB profile — or the `--output-intent-profile`, where
  it is an RGB output or monitor profile — is written at the object every catalog entry names
  (`Rewrite::DestinationProfileReplaced`, a statement). It is the profile the `OutputIntent`
  rewrite gives a file stating no intent, so the claim is ADR 0948's; DeviceRGB drawn through it
  is within a 255th of what was drawn without one. The entry's own identification is kept.
- **Not drawn through, any other `/N`**: refused. A CMYK profile would carry every `DeviceCMYK`
  colour where this reader drew §10.4.2.5's conversion, and no grey profile is shipped.
- **Drawn through** (an input-class RGB profile, or a class field of `xxxx`): refused. Any profile
  written over it moves the colours it drew, and restating its class is a claim about a device.

Entries naming two objects, a profile stated inline, or a page-level array are refused by name.
Witnesses at 2b: Isartor `6-2-2-t02-fail-a`, veraPDF `6-2-2-t03-fail-b`, `6-2-3-t01-fail-b` and
`6-2-3-t03-fail-b` convert, each page byte-identical at 72 dpi before and after and veraPDF's 2b
profile compliant; `6-2-2-t03-fail-a` and `6-2-3-t03-fail-a` are the drawn-through refusal; the
three `PDF_A-4` copies stay refused by their PDF 2.0 header.

## 2. `graphics/halftone-transfer-function-only-where-required` (3 documents)

Part 2's section 6.2.5 admits a halftone's `TransferFunction` only where §10.6.5.6 requires one,
a type 5 component for a nonstandard colourant (PDF Association issue #314 removed the rule from
part 4). §10.5's second bullet is drawn here — "A transfer function specified in a halftone
dictionary shall override the corresponding one specified by the current transfer function
parameter in the graphics state" — so the entry takes ADR 1964's two answers, one site beside
`/TR` and `/TR2`: a **function** is the loss `transfer-function`, written into `xmpMM:History`
before it goes; **`Identity` is free wherever it stands**, because a conversion that completes has
removed every `/TR` and written every `/TR2` `Default`, the identity on this device, so the
component is drawn through the identity before and after. A component *lacking* a required
function is refused: the function is the producer's. The halftone must be a dictionary object of
its own, named by reference from the type 5 halftone or the graphics state; others are refused.
The three census witnesses are part 4 files with a PDF 2.0 header, refused at 2b by that, so the
witnesses are fixtures: an identity removed with the display list within a 255th (the sRGB intent
part 2 adds for the fill), an inverting function refused unauthorised and removed authorised with
the page moving and the function in the history, a type 5 `Spot` component without one refused.
