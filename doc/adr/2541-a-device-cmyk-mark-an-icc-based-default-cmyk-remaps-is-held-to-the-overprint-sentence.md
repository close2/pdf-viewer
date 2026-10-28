# 2541 — A `DeviceCMYK` mark an `ICCBased` `/DefaultCMYK` remaps is held to the overprint sentence

Status: implemented. Session 1853. Section 3 narrows the guard of ADR 2518 section 5.
Context: ISO 19005-2 and ISO 19005-4 section 6.2.4.2, the overprint sentence (cited, not quoted); ISO 32000-2
§8.6.5.6 and its NOTE (2020), §8.6.7; `TechNote 0010` A028 as `crate::survey::DefaultSpace` carries it; ADR 2518.
Code: `crates/pdf-archive/src/survey.rs` (`ThroughDefault`, `IccCmykPaint::through_default`, `Cmyk::Remapped`),
`survey/colour_spaces.rs` (`process_or_remapped`), `survey/painting.rs`, `table/graphics.rs`
(`no_overprint_mode_one_under_icc_cmyk`); `crates/pdf-transform/src/archive/supplied_cmyk.rs` (`zeroed_with_the_icc_marks`).

## 1. The decision

§8.6.5.6: when a device colour space is selected, a `/DefaultCMYK` in the current resources' `/ColorSpace` "shall be
used as the colour space for the operation currently being performed", for `DeviceCMYK` itself and for it as a
`Pattern`'s underlying space, an `Indexed` base or a selected alternate; its NOTE (2020) says the current colour space
is then the default. So a selection reaching `DeviceCMYK` under an `ICCBased` CMYK default is an `ICCBased` CMYK mark,
and section 6.2.4.2 forbids overprint mode 1 with overprinting on for it. The survey decides this at the selection
(`k`, `K`, `cs`, `CS`), from the defaults the stream's context already resolves per resource scope. A `DeviceN` default
remaps to no `ICCBased` space and leaves the mark as it was.

The two parts read a default differently (A028): a default only an inherited dictionary states is part 4's reading and
not parts 2 and 3's. `ThroughDefault::Inherited` marks are therefore recorded both as `ICCBased` paints (judged at part 4
only) and as process overprints (the population `overprint::zeroing` and ADR 2518's guard read); an explicitly
associated default makes the mark an `ICCBased` one for every part and no process overprint.

## 2. Evidence and what moved

The unit test `a_device_cmyk_mark_an_icc_based_default_remaps_is_an_icc_based_one` holds the three cases (own default:
1 finding at both parts; no default: 0; inherited: 1 at part 4, 0 at part 2). Against the census of HEAD without the CJK
pack, no document moved on this row at any of the eight lines: no source converted at HEAD draws such a mark, or
`overprint::zeroing` answers it within the same conversion.

## 3. The guard narrowed

ADR 2518's guard in `pdf-transform`'s `supplied_cmyk.rs` refused a written `DefaultCMYK` wherever the survey found a
process overprint, because the output check could not see the mark the default remaps. It now can, so the guard keeps
only the case no rewrite of the same conversion answers: a `DeviceCMYK` mark under mode 1 whose graphics state is not one
`overprint::zeroing` sets to 0 for the file's own `ICCBased` CMYK marks, or that blends in four components (where zeroing
refuses), and every walk a bound stopped short. A state that governs both kinds of mark is zeroed by the overprint row's
own decision, which offers the `overprint-mode` loss (ADR 1978) where it moves a `DeviceCMYK` mark, so writing the default
behind it leaves no mode-1 `ICCBased` mark. `a_supplied_press_is_written_where_the_mode_its_marks_keep_is_set_to_zero_for_the_icc_ones`
holds the case, and fails against the unnarrowed guard. The refusal's sentence names what remains.
