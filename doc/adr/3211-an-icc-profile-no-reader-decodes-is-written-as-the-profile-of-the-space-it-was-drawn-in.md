# 3211 — An ICC profile no reader decodes is written as the profile of the space it was drawn in

Session 2127, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R07 entry "an ICC profile written as its alternate"
(`UnknownFilter-ICC`). Context: ADR 2689 (a stream no reader decodes is written as the reader drew
it, absent, behind the loss `undecodable-stream`), ADR 2700 (the assumed inks written as an ICC
output profile), ADR 2701 and A780 (which profile states an uncharacterised `DeviceCMYK`);
`crates/pdf-transform/src/archive/undecodable.rs` (`Role::Profile`, `profile_family`), `rewrite.rs`
(`written_absent`). Clauses: ISO 32000-2 §8.6.5.5 Table 65, §7.4; ISO 19005-2 sections 6.1.7.2 and
6.2.4.2, ISO 19005-4 sections 6.1.6.2 and 6.2.4.2.

## Context

`UnknownFilter-ICC`'s form fills and strokes in `[/ICCBased 15 0 R]`, whose profile states
`/XXXDecode` and `/N 4` and no `/Alternate`. No reader of the standard decodes it, and Table 65 says
what a reader does then: "If this entry is omitted and the PDF reader does not understand the ICC
profile data, the colour space that shall be used is DeviceGray , DeviceRGB , or DeviceCMYK ,
depending on whether the value of N is 1 , 3 , or 4 , respectively." ADR 2689 refused the profile by
name, since writing that alternate in its place needs the licence ISO 19005 asks of a device space.

## Decision

The profile stream is written, at the same object, as **the profile of the device space the window
drew the colour space in**: for four components the profile of this renderer's assumed inks, the one
every uncharacterised `DeviceCMYK` colour is drawn through (ADR 2700); for three, sRGB, which is how
this renderer shows `DeviceRGB`. The colour space array, the content and every other object stay as
they were; the stream keeps `/N` and any `/Range`, under `FlateDecode`. So the colour space is
device-independent, as section 6.2.4 asks, and it states what the window drew rather than a press
the file never named: no `press-assumed` loss is taken, because the profile written is the drawing
itself (A780's reason for the inks profile), under every profile that takes `undecodable-stream`.
The stream is reported and recorded in `xmpMM:History` as ADR 2689's streams are, role "ICC profile".

**Refused by name** (`PROFILE_NOT_BUILT`): a profile of one component, whose `DeviceGray` no shipped
profile states; and an `/Alternate` other than the device space of `/N`, which the window draws as
that alternate and no profile here states.

## Consequences

`UnknownFilter-ICC` converts at part 4 under `as-shown` and `as-if-printed`, passes veraPDF 1.30.3,
and draws byte for byte as the source at 150 dpi; at 2b it stops, as before, at its `%PDF-3.8` header
(R06b). The `Holders` walk also reads a colour space array written as an object of its own. Tests in
`crates/pdf-transform/tests/archive/undecodable_streams.rs`, the page drawn pixel for pixel against
the source for both families.
