# 1964 — A transfer function is a loss an operator authorises, and its statement is kept

Session 1564, the archive slot of batch ninety, on `doc/todo/72` section 2 item 4.
Status: **accepted** and **built** in `pdf-transform`. Context: ADRs 0816, 1210, 1942, 1952
section 3; `doc/pdf-a-mitigations.md` section 4.3. **Reverses ADR 1952 section 3 (b)** by
argument.

## 1. What was owed

ISO 19005-2 section 6.2.5 and ISO 19005-4 section 6.2.5 forbid a graphics state's `/TR`. ADR
1952 removed it where every `/TR` the file states is the name `Identity` (26 of the census's 27
documents) and refused any other, deciding for the rest: `as-if-printed` a `discard` with the
function kept in `xmpMM:History`, on §8.4.1's Table 52 making the function compensation "for
nonlinear response in a particular output device", which a press was taken to replace with its
own; and `as-shown` a stop. Since ADR 1952 the site had dropped off `--remedy-sites` and the
census: a `Stated` row is not a site, though one document was still refused at it.

## 2. The mechanism: a per-document loss, and the record a condition of it

`graphics/no-transfer-function-in-a-graphics-state` is `Conditional::PerDocument(
Loss::TransferFunction)`, the shape ADR 1210 gave the page boundaries: `super::transfer::removal`
reads the validator's findings, and where every `/TR` is `Identity` the row is the `Stated`
removal it was; where one is not, it is `Rewrite::TransferFunctionRemoved`, a loss the caller
authorises (`--authorise transfer-function`, or `discard` in a configuration). The removal writes
each function into the output's own `xmpMM:History` first — the state's `/TR` as stated and every
object it reaches in clause 7's syntax, a stream's data decoded and in hexadecimal — and the JSON
report's `removed_transfer_functions` carries the same statement per state. A packet that will not
take the record withdraws the removal, as `A48`'s constructions are withdrawn; a function that
does not decode, or past 16 384 decoded bytes or 64 objects, is refused rather than removed
without its record. The site is listed again (`--remedy-sites`, the census) as conditional.

## 3. Why no shipped profile authorises it

ADR 1952's premise for (b) does not hold. Table 52 has a reader initialise the transfer to "a
suitable device dependent value"; a graphics state's `/TR` replaces that value, and §10.5 binds
every processor alike — "the PDF processor shall apply the transfer function after performing any
needed conversions between colour spaces" — so a press does not replace the producer's function:
it applies it, on paper as on a screen. §10.5 says a page meant to be device-independent "should
not define a current transfer function in the graphics state", and a file that defines one all
the same may depend on it: page one of `issue6931_reduced.pdf`, whose one line of text says its
logo should be red, draws it red under its four sampled functions and as a near-black negative
without them (72 dpi, rendered by `quorra-transform
render`, the `/TR` entries blanked byte for byte as the rewrite removes them: 5.83 % of pixels
differ, worst tile 188 of 255). Printing would have carried it, so `as-if-printed`'s yardstick says
`stop`, and `as-shown`'s says so for the window. Both profiles now state the `stop` as a row.

## 4. What it changes

No pixel of a converted file: no shipped profile authorises the loss, a document whose every
`/TR` is `Identity` converts as before, and the witness is refused at 2b by
`fonts/font-programs-embedded` (a standard 14 font with no descriptor) whatever is authorised.
`--remedy-sites --to 2b` lists the site again, as conditional: 75 sites where it listed 74. With
the loss authorised, veraPDF's `6-2-5-t01-fail-a.pdf` converts at 2b and 4 (`archive_corpus`'s
authorised run, with ADR 1965: 479 to 481 at 2b, 220 to 222 at 4) and veraPDF 1.30.3 calls the
output compliant; its four sampled functions are in the output's `xmpMM:History`, and the eyes
they drew black are drawn white (0.03 % of the page's pixels at 72 dpi) — the loss, as stated.
The `Identity` statement's sentence now cites §8.4.1's Table 52, which ADR 1952 called Table 53.
Owed: `/TR2` other than `Default` (`second-transfer-function-is-default`, no corpus document),
and a halftone's `TransferFunction`.
