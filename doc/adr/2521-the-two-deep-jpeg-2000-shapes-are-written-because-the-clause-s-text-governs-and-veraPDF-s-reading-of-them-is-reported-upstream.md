# 2521 — The two deep JPEG 2000 shapes are written because the clause's text governs, and veraPDF's reading of them is reported upstream

Session 1843, an "archive" round of batch 125, 2026-10-28.
Status: implemented. It amends ADR 2485 (whose `Status:` line is a record and is left as written):
the two refusals that ADR kept are removed.
Context: `doc/questions/A691` (the owner's ruling), ADR 2484 (the writer), ADR 2485 (the refusals),
ADR 0922 (a conflict between the clause and the reference validator), `doc/questions/Q691`.

## 1. Decision

`DEEP_OPACITY_AT_ANOTHER_DEPTH` and `DEEP_GREY_WITH_OPACITY` and their two returns are removed from
`crates/pdf-transform/src/archive/transcode.rs`. A JPEG 2000 image with a component deeper than
sixteen bits is written back as a JP2 file whatever its opacity channel is:

- an opacity channel of another depth than the colour channels is written at its own depth, and
  the file states each channel's depth in a `bpcc` box (ISO/IEC 15444-1 I.5.3.2);
- a grey image with an opacity channel is written as two channels, one of them colour.

ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 state the channel count and the common
depth of the **colour** channels, so neither shape breaks them. `pdf-archive` already reads the
clauses that way (its `an_opacity_channel_is_not_a_colour_channel` test, and the survey's
colour-channel count, which types each channel by the `cdef` box, I.5.3.6) and is unchanged.

The existing read-back (`reads_back_as`) compares the opacity depth and the opacity integers as well
as the colour samples, so a copy that did not read back as the original still replaces nothing.

## 2. Why the clause governs

A691 ruled it: veraPDF 1.30.3's failure on both shapes is an unintentional parser shortcut and not a
reading of the clause. The decompiled `org.verapdf.external.JPEG2000.parseHeader` takes the Image
Header box's component count as the colour-channel count, skips the `bpcc` box and sets a flag that
test 5 then refuses, never reads the `cdef` box that types a channel as colour or opacity, and does
not mask the sign bit of BPC. Principle 5 of `CLAUDE.md` is the rule: another implementation
disagreeing is a question taken back to the clause, and the clause says colour channels. The two
reproducers fail veraPDF at 6.2.8.3 (6.2.7.3 for part 4) tests 1 and 5 as predicted and pass this
tree's checker.

## 3. Tests

`crates/pdf-transform/tests/archive/deep_jpx.rs`: the two "keeps the refusal" tests became
`a_deep_opacity_channel_of_another_depth_is_written_with_its_own_depth` (three 20-bit colour
channels and an 18-bit opacity channel: the integers of both read back, at their own depths, the
output conforms) and `a_deep_grey_image_with_opacity_is_written_back` (one 22-bit grey channel and a
22-bit opacity channel: the colour and opacity integers read back, and the 8-bit colours the reader
draws are equal). Four colour channels keep their refusal (a JP2 file cannot name a four-channel
space, ADR 2484), and its test is unchanged.

## 4. The upstream report

`doc/pdfa-blocking-issues/verapdf-jpeg2000-issue.md` is the report drafted for the owner to file:
the four findings, the two reproducers (the converted files beside it, with the commands that make
them) and the clause each departs from. It is not filed; the owner decides. It is indexed from
`doc/pdfa-blocking-issues.md`.

## 5. Cost

A depositor whose archive runs veraPDF 1.30.3 sees these two shapes failed at 6.2.8.3 or 6.2.7.3 on
an output this tree wrote and believes conforming. The tree reports the divergence in the issue
file and in `doc/pdf-a-mitigations.md`; it does not paper over it. No corpus document carries either
shape (ADR 2484 counts none deeper than sixteen bits), so the census does not move.

## 6. Found, not fixed

The sign-bit finding (4) has no reproducer: the writer writes unsigned samples only, and a signed
deep image has no route here. It is in the report as read from the bytecode.
