# 2114 — `DeviceRGB` beside a destination profile of another family takes a `/DefaultRGB`

Session 1639, the archive slot of batch one hundred. Status: **accepted**, **built** in
`pdf-transform`. It **amends** ADR 1153 §3, whose tie-break rested on "only CMYK has" a second
licence. Context: ADRs 0947, 0927, 1153, 2007; `doc/pdf-a-conversion-limits.md` section 4.1;
`doc/pdf-a-mitigations.md` section 4.2. Code: `crates/pdf-transform/src/archive/default_rgb.rs`,
`Rewrite::DefaultRgb`, `Answer::RgbUnderPartTwoOrFour`.

## 1. What was refused, and how often

Under both shipped lower-bound profiles the veraPDF corpus's largest buildable group of refusals
was one shape: a file whose own PDF/A output intent holds a CMYK press profile and which draws, or
blends a transparency group, in `DeviceRGB`. ISO 19005-2 and ISO 19005-4 section 6.2.3 let a file
state one destination profile, so the RGB row was refused with `WRONG_FAMILY`, and the six rows that
send a space to section 6.2.4.3 — a group's `CS` (ISO 19005-2 section 6.2.10, ISO 19005-4 section
6.2.9), an `Indexed` or `Pattern` base, a `Separation` or `DeviceN` alternate — were answered by the
output intent and then refused by the output's own verdict. Before this, at PDF/A-2b, 12 documents
stopped on the first and 7 of the post-verdict refusals were the second; at PDF/A-4, 16 and 7.

## 2. The licence ADR 1153 did not count

Section 6.2.4.3 of both parts admits `DeviceRGB` where a device independent `DefaultRGB` is set
when the space is used, as well as through an RGB output intent. ADR 1153 §3 settled the
mixed page by the observation that only CMYK had a second licence — the `DeviceN` default — and
that holds of the two **output-intent-free** licences the standard gives CMYK under part 2. It does
not hold of RGB in the case where the intent is already chosen and is CMYK: there the device
independent `DefaultRGB` is the licence left, in both parts.

Which RGB space: the shipped ICC sRGB profile, the convention `doc/pdf-a-conversion-limits.md`
section 4.1 records as a choice and the shipped RGB intent already states. On this reader it is
also the page as drawn: `pdf_colour`'s `device_family` lets an output intent stand in only for the
family with its own component count, so a CMYK intent left `DeviceRGB` drawn as the screen's sRGB,
and the shipped profile read as an `ICCBased` source is the identity to one level of 255 (the test
`the_shipped_srgb_profile_is_the_identity_through_the_profile_route`). ISO 32000-2 §11.6.6's
Table 145 is why a group's `CS` is answered by the same entry: it subjects a group's device space to
remapping by the default entries of the current resource dictionary's `ColorSpace` subdictionary.

## 3. The decision

- `Answer::RgbUnderPartTwoOrFour` on the two `DeviceRGB` rows (`Some(Rgb)`) and the six rows whose
  family is the document's (`None`); its rewrites are the output intent and `Rewrite::DefaultRgb`.
- The default is prepared only where the profile in hand is not an RGB one **and** the survey shows
  `DeviceRGB` selected, or a group blending in it, with no `DefaultRGB` in the explicitly
  associated dictionary. Otherwise the row's answer is exactly what it was. The survey is walked
  for this only under a non-RGB profile, so the common conversion walks nothing more.
- It is written where the `/DefaultCMYK` is (`DefaultSites`, the structural walk renamed from
  `CmykSites`), never over an entry the producer wrote; the producer's intent is untouched.
- `Decision::Stated`, with `default_rgb::REINTERPRETS`; nothing is recorded in `xmpMM:History`,
  for the shipped RGB intent's reason — it is the same statement about the same silence, and no
  approximation the standard names is applied.

## 4. What it does not answer, and why

- **A `DeviceCMYK` group or colour beside an RGB intent.** Under part 2 the `DeviceN` default is no
  answer to a group: Table 145 excludes `DeviceN` as a group colour space. An ICC `DefaultCMYK`
  over the shipped press profile is a licence both parts admit, but it is not what this reader
  drew — without a CMYK intent `DeviceCMYK` goes through the assumed press, not that profile —
  so writing it would be a statement this reader's own page contradicts (`doc/todo/72` section 3
  names the same gap for the `DeviceN` default's sentence). These stay refused, by
  `WRONG_FAMILY` or by the output's verdict, and `--output-intent-profile` is still their answer.
- **A mixed page with no intent at part 4.** Choosing the CMYK shipped profile with a `/DefaultRGB`
  beside it would license both rows, at the same cost as the bullet above. Not taken.

## 5. Evidence

`crates/pdf-transform/tests/archive.rs`: the RGB row and the group row each answered at part 4
beside the file's own CMYK intent, conforming after; the CMYK group beside an RGB intent still
refused. The refusal census (`archive_corpus`'s `the_shipped_profiles_list_every_refusal_by_site_and_reason`)
before and after, the witnesses' veraPDF verdicts and their pages before and after are
`doc/history/1639`'s.
