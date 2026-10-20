# 2098 — An `ICCBased` space over the profile already in force is named `DeviceCMYK`

Session 1631, the archive slot of batch ninety-nine. Status: **accepted**, **built** in
`pdf-archive` (the shared reading) and `pdf-transform`. It **reverses** ADR 0965 §2 and ADR 0982
§5 on the two duplicate-profile rows. Context: ADRs 0948, 1978, 1990, 2083 §2;
`doc/pdf-a-mitigations.md` section 4.2. Code: `crates/pdf-transform/src/archive/duplicate_profiles.rs`,
`Rewrite::DuplicateProfileWrittenDeviceCmyk`, `pdf_archive::cmyk_profile_uses`.

## 1. What the two ADRs held, and which half no longer holds

ADR 0965 refused `graphics/no-icc-space-duplicating-the-output-intent-profile` and
`graphics/separation-alternate-space-does-not-duplicate-a-current-profile` (ISO 19005-4 sections
6.2.4.2 and 6.2.4.4) on two grounds, and ADR 0982 upheld both.

- **Siting.** The findings name the page that *used* the space, not the array, so siting the
  rewrite looked like a second content walk. It is not one: the uses are the survey's selections,
  the one walk the findings come from, and the array is wherever the file states `[/ICCBased P]`
  for a profile `P` some use duplicates. The rewrite does not need to know which array a use
  went through, because it is made for **every** array naming `P` and is therefore owed a
  statement at **every** use of `P`. The validator and the converter now read one function,
  `pdf_archive::cmyk_profile_uses`, so the two cannot drift.
- **Value.** §8.6.5.7 leaves the implicit conversion to the processor, and §8.6.7's non-zero
  overprint mode turns on it. That stands — and it decides only marks painted with overprinting
  on, since §8.6.7 says the mode "shall have an effect only when the overprint parameter is
  true", and with it false a painting operation erases the colourants it does not specify under
  either name. The colour itself is the same colour: ISO 19005-4 section 6.2.4.3 renders
  `DeviceCMYK` through the current PDF/A output intent's profile, and ISO 32000-2 §11.7.2 makes a
  device-space object in a group "defined with a CIE-based colour space with the same number of
  colourants" take "the CIE-based space of the nearest such ancestor" for compositing — in both
  cases the very profile the array named. §11.7.2's next sentence keeps "the full semantics of
  DeviceCMYK … including overprinting", which is the one residue, and the condition below removes
  it.

## 2. Decision

`Rewrite::DuplicateProfileWrittenDeviceCmyk`, `Stated`, under both rows: every `ICCBased` array
naming a profile that some CMYK use finds identical to the intent's or the `ICCBased` blending
space's is written `/DeviceCMYK` — in resource dictionaries, `Separation`/`DeviceN` alternates,
`Indexed` bases, image and shading dictionaries, and an array object of its own — except a
group's `CS`, which is the space a use was compared with. Refused by name, each over-counted
rather than under:

- a use of the profile where the intent then current or the `ICCBased` blending space in force
  holds a different profile, or neither holds this one (another page's intent; an RGB group);
- a use outside an `ICCBased` blending space on a page stating any group colour space, since the
  survey records a blending profile only for an `ICCBased` group and none for an image;
- any dictionary in the file stating `OP` or `op` true, used or not — the survey records
  overprinting for an `ICCBased` mark but not for a `Separation` over an `ICCBased` alternate;
- an array naming the profile as a `DefaultCMYK` value, or an array object a group's `CS` names;
- a survey a bound stopped, or no array reached.

The output check stays the net for an object generation 0 does not number.

## 3. Evidence

All eight witnesses convert to PDF/A-4 and veraPDF 1.30.3 passes each (the sources fail):
`6-2-4-2-t03-fail-a`, `-b`, `-e` (the intent: one object, an MD5 equal, one stated and one
computed), `-c`, `-d`, `-f` (the page group, no output intent) and `6-2-4-4-t01-fail-i`, `-j`
(a `Separation` and a `DeviceN` alternate). Each page renders byte-identically to its source at
72 and 720 dpi. Seven `pdf-transform` tests: the intent, the alternate and the group statements
(display lists equal), and the overprint, RGB-group, device-group and `DefaultCMYK` refusals.
