# 2518 — Under `as-if-printed` a part 4 file in `DeviceCMYK` takes the shipped press as its assumed press, and the loss is named

Status: implemented in `pdf-transform`. Session 1842, the archive slot of batch one hundred and
twenty-five. Context: `doc/questions/A694` item 2 (a); ADRs 2114 section 4, 2130 sections 2 and 3, 1978,
2490, 2519; `doc/pdf-a-mitigations.md` section 4.2; `doc/todo/72` section 3; `doc/pdfa-remedies.md`.
Code: `crates/pdf-transform/src/archive/supplied_cmyk.rs`, `decision.rs` (`Loss::PressAssumed`),
`prepare.rs`, `report.rs`.

## 1. What was refused

A part 4 file that draws in `DeviceCMYK` beside a destination profile of another family. ISO 19005-4
section 6.2.3 lets a file state one destination profile, so the licence of section 6.2.4.3 that a
CMYK intent would be is closed, part 4 admits no `DeviceN` default, and the licence left is a device
independent `DefaultCMYK`, which is an ICC CMYK profile and therefore a press. ADR 2130 wrote that
default from a profile the operator supplied; with nothing supplied the row was refused with a
sentence naming the flag. ADR 2114 section 4 and ADR 2130 section 3 declined to write the CMYK profile
this program ships in its place, because "this reader draws `DeviceCMYK` ... through its assumed
inks", which no ICC profile states, so a shipped default would be a statement the page this reader
shows contradicts.

## 2. Decision

**The press is assumed, and the word for what that costs is `press-assumed`.** Under a profile that
authorises the loss, part 4's `DeviceCMYK` row is answered by writing the shipped CMYK profile as an
`ICCBased` `/DefaultCMYK` wherever ADR 2130 writes the operator's, never over the producer's entry,
and the file's own intent is left as it was. The decision is a `Loss` (`Decision::Lost`), not a
`Stated`: the colours the file now has are the shipped profile's, and this reader's window drew them
through its own inks, so the number a person sees on a swatch moves.

- **Part 4 only.** Part 2 has a second licence, the `DeviceN` default, that states no press; the
  assumption is not offered there (`part_two_is_not_offered_the_assumed_press`).
- **The operator outranks it.** A profile supplied with `--output-intent-profile` is written as
  before, `Decision::Stated`, costing nothing and naming the operator; the assumed press is reached
  only where nothing was supplied (`a_press_the_operator_supplied_outranks_the_assumed_one_and_costs_no_loss`).
- **`as-if-printed` grants it; `as-shown` stops.** The profile's `discard` at the row's site grants
  `press-assumed`; `as-shown` grants none, so its refusal stands and its sentence now names the loss
  that `as-if-printed` grants beside the flag.
- **It is said out loud.** The report's `press_assumed` field (JSON and text) names the profile by
  its description and copyright, and the file's `xmpMM:History` records that a DefaultCMYK was added over that profile as an assumed press, names the
  loss `press-assumed` and claims no provenance for the press;
  a packet that cannot take the entry does not get the default (RFC 0007 section 5b.1, the rule
  ADR 2130 applies to a supply).
- **It is refused where it would fail the standard.** A `DefaultCMYK` over any ICC profile makes
  every `DeviceCMYK` mark an `ICCBased` CMYK one (ISO 32000-2 §8.6.5.6), and ISO 19005-4 section
  6.2.4.2 forbids overprint mode 1 with overprinting on for such a mark. A file whose survey finds a
  `DeviceCMYK` mark with overprinting on and a non-zero overprint mode is therefore refused with a
  sentence naming the clause; the same applies where the walk of the content was stopped by a bound
  and cannot say there is none. The guard binds the operator's press as well (section 5).

## 3. Why ADR 2114 section 4 and ADR 2130 section 3 stand, and are reopened for this profile only

Their argument was about *statements*: a default written as the file's colour space says what the
file's colours are, and a statement this reader's own page contradicts is false. That is true of
`as-shown`, whose contract is that the output is the page this reader shows. It is the owner's answer
that `as-if-printed` has a different contract (A694 item 2): "we will always assume that docs are
displayed only", and the profile "shouldn't try to print correctly. it's just a lower bound that what
we are allowed to lose if we printed and scanned, but it does not mean, that we should try to
'imitate' printing". A document that was printed and scanned back has colours a press made, and
which press is not recoverable from the print; the lower bound on what may be lost is "some press's
colours", and the shipped profile is a press. So under `as-if-printed` the contradiction is not a
falsehood but a loss the profile's contract already admits, and it is named so that nobody reads the
output's colours as the producer's. `as-shown` keeps ADR 2130 section 3 whole.

**The note.** `doc/profiles/as-if-printed.toml` states that "as if printed" is "as if printed and
scanned"; its one-sentence description ends "A print scanned back in, not an imitation of printing";
`quorra-transform archive --help`, `config.rs`'s module comment, `doc/pdf-a-mitigations.md` section
4.2 and `doc/todo/72` carry the same sentence. Nothing in this ADR simulates a press: no halftone, no
dot gain, no black generation; the profile is chosen because part 4 needs *a* press named, and the
one the program already ships is the one it can name without inventing one.

## 4. Measured

`archive_corpus`'s `the_shipped_profiles_list_what_real_producers_files_refuse_on`, converted counts,
`as-if-printed` at part 4, before (HEAD's census of the same commit) and after: with the CJK pack
pdf.js 841 to 846 (+5), corpora 385 to 391 (+6); without it pdf.js 830 to 835 (+5), corpora 385 to
391 (+6). At 2b and under `as-shown` at both targets, every section is unchanged. None is newly
refused. The row named 29 documents (7 in pdf.js, 22 in the corpora): 11 are converted, 9 are
refused by the overprint sentence, 9 stay refused elsewhere (2 signature assertions, one output
check on font widths, fences and embedded files). veraPDF 1.30.3 at part 4 passed 10 of the 11
converted outputs; the eleventh, `0004-637X_751_2_90.pdf`, fails ISO 19005-4 section 6.2.10.5 (glyph
widths), and its 2b output, which this route does not touch, fails the same widths under part 2's
section 6.2.11.5: a font defect of the converter that the colour row was hiding at part 4. The first build of the route, without the overprint guard, converted 19:
veraPDF failed 7 of its 19 part 4 outputs on section 6.2.4.2, which is what the guard answers. The
loss on a cyan/black fixture: the largest channel difference is 14 of 255 and no cell gains or loses
paint (`the_loss_is_a_real_shift_of_the_colours_and_no_mark_moves`). The route is not a pixel-gate
subject: no arm was exported and none compared, and the colours of a document under the loss move
by design.

## 5. The overprint finding, and what is still open

The tree's own output check for the clause (`no_overprint_mode_one_under_icc_cmyk`, in
`pdf-archive`'s `table/graphics.rs`) examines `ICCBased` paints the survey recorded. A
`DeviceCMYK` mark remapped by a `DefaultCMYK` is such a paint by ISO 32000-2 §8.6.5.6, and the survey
does not resolve the remap, so the check passes an output veraPDF fails. Seeing it needs the survey
to resolve the default per resource scope, a change to `survey.rs` that other rounds are editing and
that wants its own planted veraPDF witness; it is reported as found and not fixed. The guard in
`supplied_cmyk.rs` stands in for it, for the operator's press as for the assumed one, because a
refusal that told the operator to supply a press would send them to a conversion that prints
"conforms" over an output veraPDF fails; it also refuses a walk a bound stopped, as
`overprint::zeroing` does. It is conservative: of the 19 documents the first build converted it
refuses 8, the 7 that veraPDF failed and `response-to-fiber-concerns[1].pdf`, whose part 4 output
was not planted on veraPDF. `doc/todo/72` carries the witness for the check itself.

## 6. Evidence

`crates/pdf-transform/tests/archive/press_assumed.rs`, nine tests: the authorised press written as
the default, named, and conforming (`an_authorised_assumed_press_is_written_as_the_default_cmyk_and_named`);
nothing authorised keeps the refusal and names the loss; the operator's press outranks; part 2 is
not offered; the two shipped profiles differ as stated; overprint mode 1 keeps the refusal; the loss
moves colours and no mark; a walk stopped by a bound and an operator's press each keep the
refusal under overprint mode 1; the report's description and the history name the assumed press.
The census above and `doc/history/1842` carry the figures.
