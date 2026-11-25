# 2244 — A colour operator as drawn, an exponent as its value, and an inline key read once

Lane 3, the `as-shown` refusals round 2240's census left at section 5.1's fence. ADRs 3677–3680.

- **The list** (2240's census logs, by first class; pack and no pack the same):
  string-unterminated 7, colour-operands 5, delimiter-unmatched 3, inline-object-malformed 3,
  operands-left-over 3, marked-content 2, wrong-type 2, number-salvaged 2, exponent 2, key-twice
  2, data-extent 1, hex stray byte 1, short-of-operands 1, integer-past-i64 1 (part 4 only).
- **A probe of every carrier** listed each one's classes: the govdocs files hold six to ten classes
  and thousands of undefined operators each — transit damage, `as-if-printed`'s (ADR 3680).
- **ADR 3677**: a number in exponential form is written as the decimal of the value read, wherever
  `as_drawn` writes; the 5.1 row states `content-as-drawn`.
- **ADR 3678**: `inline_image::Scan::set_aside` states each entry the reader did not read (a key
  twice; Errata Collection 3 makes the abbreviation win); `as_drawn` writes it out under ADR 3558's
  proof. Reverses ADR 3560's first item.
- **ADR 3679**: a colour operator given 1, 3 or 4 operands that do not fit is written as `G`/`RG`/`K`
  (or `g`/`rg`/`k`), operands kept — what the reader drew — under the proof. Closing a section left
  open was built and withdrawn: the census found `050734` no longer photographed under
  `as-if-printed` (the write counted its page answered) and its output failing section 5.1.
- **ADR 3680**: string-unterminated, a delimiter after an operand and inline-object-malformed have
  no write; data-extent and the stray hex byte have one not built (one carrier each, among other
  classes). Each class has its own refusal sentence.
- **Documents** (`as-shown`, both parts): `issue18894`, `issue14256` and `InlineAbbreviations` convert;
  `sci-notation` passes 5.1 and is refused by 6.2.2's `ETBT`; `367594` and `050734` now stop at
  `string-unterminated`. Re-run after the withdrawal: all three convert, `050734` and `504076` are
  photographed under `as-if-printed` again.
- **Census** against 2240's, pack and no pack the same movement: `as-shown` either part pdf.js
  906 → 908 written, 41 → 39 refused at both; corpora 418 → 419, 49 → 48. `as-if-printed` moved
  only by `050734` (462 → 461), the withdrawn write's, re-checked converting.
  veraPDF 1.30.3 over 5509 outputs (2240: 5505): 5457 compliant, 40 not and 12 unfinished as before;
  the new outputs pass.
- Left: the `MICOM` poster's form inherits the page's `ICCBased` space (§8.10.1) and the walk starts
  each form at DeviceGray; `sci-notation`'s `ETBT` (an undefined operator with no operand before it,
  read as nothing); 2240's `Td` across a cut.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`: fmt 0;
clippy `-D warnings --all-targets` 0; sandbox `--bins` 0; nextest `--lib --bins --tests` (`xvfb-run -a`)
0, 5333 of 5333, `audit-diff.py` "failed 0, expected 0"; rustdoc `pdf-model` and `pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0.
The first run's clippy 101 (an unchecked division, a function past 100 lines, a one-element
`for`) was fixed before it; its two conformance failures were this record's placeholders.
