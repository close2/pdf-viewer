# 2263 — The survey walks a soft mask's group from the graphics state that sets it

Session 1714, the archive slot of batch one hundred and ten. Status: implemented, in
`crates/pdf-archive/src/survey/graphics_state.rs` and `crates/pdf-transform/src/archive/rewrite.rs`.
It **amends ADR 2247**, whose last section named this as open, and narrows `survey.rs`'s header
list of what is not walked.

## The reading

ISO 32000-2 §11.6.5.1 defines a soft mask's group as a transparency group `XObject` named by the
mask dictionary's `G` entry, and fixes the mask at the `gs` that installs it. Its content selects
resources and paints colours exactly as a form's does. ISO 19005-2 section 6.2.4.3 and ISO 19005-4
section 6.2.4.3 restrict where `DeviceGray`, `DeviceRGB` and `DeviceCMYK` may be used and state no
exemption for a mask's group; TechNote 0010 A026 exempts only `DeviceGray` as the `ColorSpace` of a
soft-mask **image**, which stays outside the walk for the header's reason. So a colour painted in a
mask's group is a use of that colour space, and not walking it was the survey's declared
under-report, not a reading of the clause.

## Decision

At every `gs` whose `SMask` is a dictionary, the survey walks the stream its `G` names as a form,
with route `SoftMask`, from §8.4.1's initial state: the group is not painted into the stream that
set it (§11.5.3 turns its result into one luminosity or alpha), so neither the invoking stream's
colour nor its blending space reaches inside. The group's own `/Resources`, else the page's, are
the ones in force, which is what the interpreter reads. Its ledger frame names no object, because
the interpreter's does not and the cross-check compares frame for frame.

`crates/pdf-archive/tests/cross_check.rs` therefore compares the selections under a mask like any
other place's and no longer accounts for them by their setter; it fails if the interpreter made
selections under a mask and the survey none. `editions::SHIFTS` gains §11.6.5.1, which ISO
32000-1:2008 numbers 11.6.5.2.

## The converter follows

`rewrite::DefaultSites`, the walk that finds every resource dictionary a `/DefaultCMYK` or
`/DefaultRGB` has to reach (TechNote 0010 A003's page, pattern, form and Type 3 font), now also
queues the `/G` of every soft mask an `ExtGState` names: it is a form `XObject`, and once the survey
judges what it paints, a default space written everywhere but there leaves the output failing its
own second verdict. With the walk alone, `digitally_signed_3D_Portfolio.pdf` (`doc/corpora`),
which converts on HEAD under `as-if-printed` at 2b, was refused at the second verdict on
`transparency-group-colour-spaces-obey-the-colour-rules`; with both it converts again.

## What it moved

- **The cross-check**: 13 560 selections compared, 0 disagreements (13 547 before, the thirteen
  ADR 2247 judged now compared); the interpreter made 13 selections under a soft mask and the survey
  13. Planted — the survey's call taken out — the gate fails naming 13 places only the interpreter
  ran, innermost a soft mask group (trap 13).
- **The validator over the veraPDF corpus**: missed 0, over 0 at all six targets, as on HEAD.
- **The converter over the veraPDF corpus**: unchanged at every target (2b 525 converted / 86
  refused by name, 4 267 / 92).
- **The real corpora under both profiles, by document**: `bug1755507.pdf` (pdf.js), refused on HEAD
  at the second verdict on `transparency-group-colour-spaces-obey-the-colour-rules` under both
  profiles and both parts, converts, and veraPDF 1.30.3 finds its output compliant; no document is
  newly refused on a colour row under either profile.
- **The witness's own reading**: veraPDF 1.30.3 over five of the twelve fixtures (`6-2-9-t01`,
  `t03`, `t07`, `6-2-10-t01`, `t07`) reports the page's own `DeviceGray` and the blending-space row,
  and nothing inside the mask's group. ISO 19005-2 section 6.2.4.3 states no exemption for it, so
  the reading stands and the difference is the witness's (principle 5).
