# 1978 — Overprint mode 1 is a statement under ICCBased marks and a press's loss under DeviceCMYK

Session 1571, the archive slot of batch ninety-one, on `doc/todo/72` section 2 items 9 and 4.
Status: **accepted** and **built** in `pdf-archive` (the survey) and `pdf-transform`. Context: ADRs
0816, 0948, 1157, 1241, 1942, 1964, 1965; `doc/pdf-a-mitigations.md` section 4.2.

## 1. The site and its witnesses

`graphics/no-overprint-mode-one-under-icc-cmyk` (ISO 19005-2 and ISO 19005-4 section 6.2.4.2:
overprint mode 1 while an `ICCBased` CMYK space is used with overprinting on) is reached by two
documents of the pdf.js and corpora census at every target: `102952.pdf` (77 pages, three graphics
states stating `/OPM 1`, two of them `/OP true`) and `2018-Lee-Fatherinclusiveperinatal.pdf`.

## 2. What the mode does, from §8.6.7 and §8.6.5.7

§8.6.7: "Non-zero overprint mode shall apply only to painting operations that use the current colour
in the graphics state when the current colour space is DeviceCMYK (or is implicitly converted to
DeviceCMYK ...)", and "shall not apply if the native colour space of the output device does not
include CMYK device colourants". §8.6.5.7: "When this type of implicit conversion is done, all of
the semantics of the device colour space shall also apply" — the overprint mode first among them —
and the conversion is a processor's, whose conditions its NOTE 3 says "cannot be specified in PDF".
This reader does it only on a page or group compositing in a press (§8.6.5.7's ledger row); every
other page goes to the screen's three components, and `content::colour::cmyk_tints` gives an
`ICCBased` colour no tints, so the mode reaches no `ICCBased` mark there.

**Rendered first (trap 182)**: `102952.pdf` with its three `/OPM 1` written `/OPM 0` by hand draws
all 77 pages byte-identically at 72 dpi and pages 1–8 at 720 dpi. Its states also govern
`0 0 0 1 k` text, a `DeviceCMYK` mark §8.6.7 has a CMYK device overprint under mode 1 and knock out
under mode 0.

## 3. Decision

The graphics states that set the mode in force at an `ICCBased` CMYK mark painted with overprinting
on — the survey now records them by object, beside the `DeviceCMYK` marks painted under a non-zero
mode with overprinting on (stated, or as the base of an `Indexed` space or the alternate of a
`Separation` other than `All`/`None` or a `DeviceN`, over-counted rather than under) — state
`/OPM 0`. No operator sets the mode, so that settles every mark, and a mode only falls.

- **No `DeviceCMYK` mark under those states**: `Rewrite::OverprintModeRestated`, a statement.
- **One**: `Loss::OverprintMode` (`--authorise overprint-mode`) with `Rewrite::OverprintModeZeroed`
  — what a press composites moves, nothing in the window does. `as-shown` answers `discard`,
  `as-if-printed` leaves `stop`, the shape of ADR 1965.
- **Refused by name**: a mark of either kind inside a page or group blending in four components
  (there this reader composites by the mode, so the loss would be one the window showed, and no
  profile can tell it apart); a mode set by a dictionary that is not an object; a content walk a
  bound stopped (`Survey::stopped_short`, new: an empty list from a stopped walk is not an absence).

## 4. What it changes, and what the hypothesis said

`102952.pdf` converts to 2b under `as-shown`; its 77 pages at 72 dpi differ from the source on two
(at most 8 of 255, the GRACoL output intent's `DeviceCMYK` and `DeviceGray`, which the mode does not
reach — the hand-patched copy above is identical), and at 720 dpi pages 1–4 are identical.
`2018-Lee-Fatherinclusiveperinatal.pdf` is the statement (20 pages identical with the mode at 0 by
hand) and stays refused at a standard-14 font. veraPDF's three `6-2-4-2-t02-fail` files are
statements, convert, pass veraPDF 2b and draw identically. The brief's hypothesis — a loss because
the simulation draws differently — half held: the witness is a loss because of its `DeviceCMYK` text
on a press; the window composites by the mode only in a four-component blending space, which the
witness never states.

## 5. The next refusal of the witness was the CharSet removal

With the mode answered, `102952.pdf` still did not convert: the output failed
`fonts/charset-lists-every-glyph-in-the-program`. The validator keeps 32 places of a finding and the
file has 190 (33 descriptors); `sites::descriptor_sets` answered the 32. A shortened list now takes
every `/Type /Font` object's descriptor stating the key — removing a complete `CharSet` loses nothing
by the same sentence — and the output check stays the net for a descriptor no object names.

## 6. `/TR2`, under ADR 1964's rule

`graphics/second-transfer-function-is-default` (section 6.2.5's `TR2`, admitted only as `Default`)
is answered by `super::transfer` beside `/TR`: §8.4.5's Table 57 makes `Default` "the transfer
function that was in effect at the start of the page" and has `TR2` "take precedence" over a `TR`
beside it, so a `TR2` other than `Default` is written `Default`. Where every function the file
states is `Identity`, that is the initial function on this reader's device and a statement (ADR
1952 section 3's reasoning); otherwise it is `Loss::TransferFunction`, the function written into
`xmpMM:History` first, as ADR 1964 has it for `TR`. No corpus document states one; the fixture
tests and veraPDF's `6-2-5-t02-fail-a.pdf` (a four-function array) are the witnesses.
