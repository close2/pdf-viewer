# 2712 — In a PDF/A file under a CMYK intent, `DeviceGray` is the intent's black ink alone

Session 1949, a single archive round.
Status: accepted; the reader follows ISO 19005-2 and ISO 19005-4 section 6.2.4.3 (`pdf-colour`,
`colour/intent_grey.rs`; `pdf-model`, `output_intent::grey_reading`), the converter's sentence
corrected, the pinning test of ADR 2708 amended to the new behaviour.
Context: `doc/questions/Q788` and the owner's `A788` ("yes convert to black ink only", read as
option 2), ADRs 0263, 0796, 1001, 2012, 2552 (A23), 2700, 2708; ADR 2713 for the measurement.

## The rule

ISO 32000-2 leaves a grey painted beside a four-component output intent to the processor: §14.11.5
makes the intent informational, and §11.7.2 converts a device colour of another component count "in
an implementationdependent fashion". ISO 19005-2 and ISO 19005-4 section 6.2.4.3 do not (paraphrased,
as `doc/third-party-data.md` requires of ISO 19005): where a file sets no device-independent
`DefaultGray`, a conforming reader converts a `DeviceGray` colour to CMYK by ISO 32000's classic
method when the PDF/A output intent has a CMYK destination profile, and renders it with that
profile as the source. The classic method is ISO 32000-2 §10.4.2.3:

> Nominally, a gray level is the complement of the black component of CMYK . Therefore, the CMYK
> colour value equivalent to a specific gray level shall be

followed by `cyan = magenta = yellow = 0.0`, `black = 1.0 − gray`.

## The conditions, and where each is asked

- **The file identifies as PDF/A**: its document-level XMP states `pdfaid:part`, under either
  namespace spelling `pdf_archive` accepts. A file that does not is read by ISO 32000 alone.
- **The intent in force is a PDF/A one with a CMYK profile**: the intent §14.11.5's selection
  already chose for the page (`output_intent::selected`) has `/S /GTS_PDFA1` and a four-channel
  profile. A PDF/X intent of a PDF/A file is not the PDF/A intent the clause names.
- **No `DefaultGray`**: §8.6.5.6's default is resolved before the rule, in the one place a device
  family is resolved (`ColourSpace::device_family`), so a default the resources state wins, as the
  clause and §8.6.5.6 both say.

The first two are asked once per page where the interpreter selects its intent
(`content::colour::output_intent_reading`), and the XMP is parsed only where the intent is a PDF/A
CMYK one: no file whose page draws through any other intent, or none, reads its metadata to draw
(principle 2). The answer travels as `GreyReading` beside the intent everywhere the intent travels:
the interpreter's `Reading` (operators and `cs`), `Conversion` (images, shadings and meshes, parsed
after the interpreter has handed them over, and the caches keyed on a conversion), the soft mask's
group `/CS`, an imported page (its own file's identification) and §10.8.3's separation pass.

## The construction

`DeviceGray` under the rule resolves to a one-component `Separation` whose alternate is the intent's
own profile and whose tints are a new `Tints::GreyAsBlack`, §10.4.2.3's formula. A separation is the
space every route already evaluates into its alternate, so fills, strokes, images, inline images,
shadings and a mask group's `/BC` reach the rule from the one place without a second branch:

- into the device, `(0, 0, 0, 1 − gray)` through the profile, so `0 g` draws as the press's black;
- into a group composited in the intent's press, the same four numbers exactly (`to_cmyk` keeps a
  colour already in the press), rather than the press's `B2A` separation ADR 0796 gave a device grey;
- three answers are the grey's own, not a separation's: the initial colour is `DeviceGray`'s 0.0
  (§8.6.8), not a tint's 1.0; §11.5.3's device branch (A23) weighs the four components the clause
  made, which is `1 − gray`, rather than the luminance of the press's black; and the space carries
  the family's own name, `DeviceGray`, which is no colourant §10.8.3's simulated device has, so it
  always reverts to the profile.
- A `/Luminosity` group stating `/CS /DeviceGray` under the rule is composited in the intent's press
  (`soft_mask::four_component_profile`), so a `/DeviceCMYK` group of black painted into it keeps its
  ink and masks to 0, as in the source.

**Not taken here**: a *transparency* group stating `/CS /DeviceGray` keeps compositing in device grey
(`Compositing::Grey`); a grey painted into it is weighed by its black ink and leaves the group as a
neutral grey. ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 make the intent the default
blending space, and a DeviceGray group's blending space read as the intent's black is that clause's
question, not this one's; ADR 2713 measures whether a corpus page meets it. Nor is section 6.2.4.3's
'RGB' bullet taken: a grey under an RGB intent stays this reader's grey.

## What it costs

Every grey of such a file moves: black text draws as the press's black ink, not `(0, 0, 0)`. That
is the clause's requirement, and the owner chose it knowing it (Q788 option 2). The converter writes
no `/DefaultGray`, so an archive with a CMYK intent draws its greys through the press too, and its
soft masks built of grey now mask as their sources did. ADR 2713 has the population and every page
that moved.

## A23 and the ledger

A23's departure is unchanged: §11.5.3's device branch is still taken in every space. The rule
changes what is weighed, not how: a grey under the rule is four process components, and EXAMPLE 2
weighs them. The rows of §8.6.5.6, §10.4.2.3 and §14.11.5 stay `implemented`; their notes name the
rule.
