# 2750 — Overprint mode 1 under a `DeviceCMYK` mark is written 0 beside the `/DefaultCMYK`, as the named loss

Session 1969, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/overprint.rs`: `under_a_default`,
`default_history`; `archive/supplied_cmyk.rs`: `SuppliedCmyk::modes`; `archive/prepare.rs`;
`archive/rewrite.rs`; `archive/decision/colour_and_graphics.rs`: `cmyk_answer_under_part_four`).
It builds ADR 2749 and R05's last row of `doc/pdfa-blocking-issues.md`.
Context: ADRs 1978, 2518, 2541, 2693, 2747, 2749.
Clauses: ISO 32000-2 §8.6.5.6, §8.6.7, §11.7.4.3; ISO 19005-4 sections 6.2.4.2, 6.2.4.3.

## 1. The construction

At part 4, where `supplied_cmyk::prepare` would write a `/DefaultCMYK` over an ICC profile and a
`DeviceCMYK` mark is painted with overprinting on under a mode the file's own `ICCBased` marks do not
already have zeroed, `overprint::under_a_default` names the graphics states that set that mode. It
answers only where every such state is an object stating `/OPM`, the content walk did not stop
short, and no such mark is blended in four components (§11.7.4.3, where this reader composites by
the mode and the window would move: ADR 1978). A state the overprint row already zeroes for the
file's own `ICCBased` marks is left to that row, so each state is written once and named once.

- **Only where the caller authorised `overprint-mode`**, and only at part 4: `prepare` is passed
  `zero_modes`. Part 2 keeps its `DeviceN` default, which leaves the mark `DeviceCMYK` and costs
  nothing; a profile without the loss keeps `OVERPRINT_MODE_ONE`, whose sentence now says that the
  loss would have answered it.
- **The decision**: `cmyk_answer_under_part_four` carries `Loss::OverprintMode` beside the press's
  own (`press-assumed` for the shipped press, none for the inks or a supply), so the inks' statement
  becomes a decision that loses `overprint-mode` alone. The `DeviceCMYK` row's `CONDITIONAL` entry
  does **not** list the loss: a `discard` there would authorise it, and it stays the overprint
  row's to authorise, as ADR 2749 says (no new authorisation).
- **The record**: `overprint::default_history`, an `xmpMM:History` entry of its own in
  `zeroed_history`'s words, cited to ISO 19005-4 section 6.2.4.2. The default is already a
  condition of the packet taking its record (`unless_recorded`), so the entry goes with it.
- **The rewrite**: `/OPM 0` on each named state, counted under `supplied-default-cmyk`.

## 2. What it freed, and what it did not

At part 4: `as-if-printed` six of the nine, `as-shown` four (two more stop at R09's fonts). Three
are left:

- `363_Risk…` and `response-to-fiber-concerns[1]` blend their governed marks in a `DeviceCMYK`
  group, so ADR 1978's refusal stands for them. Round 1968 drew them identically with the mode at 0;
  a per-document proof by drawing both is a construction no round has decided, and ADR 1978's rule
  is the one in force.
- `digitally_signed_3D_Portfolio` now reaches the output's own verdict and fails the same row
  after the default is written. It fails the same row the same way at 2b, with the `DeviceN`
  default, where it is already a `D-defect`; its conversion lays out pages from its embedded files,
  and which `DeviceCMYK` the default misses is not traced. A converter defect, not this
  construction's.

## What it costs

Both shipped profiles name one more loss on these files; no pixel of the window moves
(`doc/history/1969`).
