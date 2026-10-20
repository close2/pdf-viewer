# 2130 — `DeviceCMYK` beside a profile of another family takes the operator's press as its `/DefaultCMYK`

Session 1647, the archive slot of batch one hundred and one. Status: **accepted**, **built** in
`pdf-transform`. It **answers** ADR 2114 section 4's first bullet. Context: ADRs 0927, 1153, 2114;
`doc/questions/A18`, `A48`; RFC 0007 section 5b.1; `doc/pdf-a-mitigations.md` section 4.2. Code:
`crates/pdf-transform/src/archive/supplied_cmyk.rs`, `Rewrite::SuppliedDefaultCmyk`,
`Answer::CmykUnderPartFour`.

## 1. What was refused

The colour family's CMYK half: a file that draws in `DeviceCMYK`, or blends a transparency group in
it, beside a destination profile of another family. ISO 19005-2 and ISO 19005-4 section 6.2.3 let a
file state one destination profile, so section 6.2.4.3's first licence is closed; part 4 states no
`DeviceN` default and §11.6.6's Table 145 excludes one as a group's colour space, so the licence left
is a device independent `DefaultCMYK` — an ICC CMYK profile, which states a press. The veraPDF census
of round 1639 found 7 such documents at 2b and 15 at part 4, refused by `WRONG_FAMILY` or by the
output's own verdict with no sentence at all.

## 2. Decision

**The press is the operator's to name, and `--output-intent-profile` is where they name it.** Where
the file states no intent, a supplied CMYK profile is the intent, as before. Where the file's own
intent is of another family, the profile was ignored — a supply that silently did nothing (trap 5).
It is now written as an `ICCBased` `/DefaultCMYK` wherever the other two defaults go, never over the
producer's entry, and the file's own intent is left as it was. The decision is `Decision::Stated`
with `supplied_cmyk::REINTERPRETS`, which says the colours now look as that press prints them; the
report and `xmpMM:History` record it as the operator's statement (RFC 0007 section 5b.1), and a
packet that cannot carry the entry does not get the default.

It answers part 4's `DeviceCMYK` row, part 2's ahead of the `DeviceN` default (one statement about
the four numbers, not two — the `DeviceN` construction is not prepared beside it), and the six rows
whose family is the document's — a group's `CS`, an `Indexed` or `Pattern` base, a `Separation` or
`DeviceN` alternate — where the survey finds `DeviceCMYK` drawn and no `/DefaultRGB` answers them.

**With nothing supplied, part 4's `DeviceCMYK` row says what to do.** It is refused with
`NO_PRESS_SUPPLIED`, naming section 6.2.4.3, the closed licence and the flag, where it was refused
with `WRONG_FAMILY`, which keeps only its RGB half. The six rows whose family is the document's keep
the intent's answer and the output's own verdict: which of the document's spaces failed such a row is
the finding's to say and not the survey's, and a sentence about `DeviceCMYK` given to a row an RGB
alternate failed would be wrong — a round that tried it refused documents the intent converts. The
survey is asked only where a profile was supplied, so a conversion nobody supplied a press for walks
nothing more than it did.

## 3. Why not the shipped press profile

ADR 2114 section 4 stands: this reader draws `DeviceCMYK` without a CMYK intent through its assumed
inks (`pdf_colour::assumed_press`), which no ICC profile states, so a shipped `DefaultCMYK` would be
a statement this reader's own page contradicts. A profile the operator supplies is their statement,
not this program's, and its pixels are theirs to choose.

## 4. Evidence

`tests/archive.rs`: the refusal's sentence at part 4; the supply at part 4 beside the file's own RGB
intent, conforming, the profile byte for byte, one intent, the history naming the operator; a group
in `DeviceCMYK` at 2b answered with a supply and refused by the output's verdict without one. The census
`a_supplied_press_profile_answers_the_colour_family_refusals` sweeps the veraPDF corpus under the
printing profile with the shipped press profile standing in for an operator's; `doc/history/1647`
has its counts beside the unsupplied census's.
