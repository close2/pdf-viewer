# 2708 — Under the assumed-inks intent, a grey inside a soft mask moves, and the page's blending space does not

Session 1947, a single archive round.
Status: accepted; the converter's sentence corrected (`pdf-transform`,
`archive/decision/colour_and_graphics.rs`: `OUTPUT_INTENT_STATES_THE_ASSUMED_INKS`), the renderer
unchanged, the renderer's side put to the owner as `Q788`.
Context: ADRs 0263, 0796, 1194, 1437, 2552 (A23), 2700 section 3, 2701; `doc/pdfa-blocking-issues.md`
section 8.

## The premise, and where it failed

ADR 2700 section 3 left six `as-shown` outputs drawing up to 25 levels from their sources:
- `bug1703683_page2_reduced`
- `bug1755507`
- `issue12798_page1_reduced`
- `issue13520`
- `issue18032`
- `issue9017_reduced`

It blamed the page's blending space: a page naming none would composite in the intent's four
components. **That premise does not hold.** Five of the six pages state `/Group /CS /DeviceCMYK`,
and the sixth states no group. This reader composites a page with no `/CS` on the device's three
components, with or without an intent (`transparency::page_press` answers `Device`).

The round stated each output's `/DeviceCMYK` groups as the intent's own `ICCBased` profile, with the
intent renamed. All of them together draw the output exactly. One at a time, they find the groups
that carry the difference, and **every one is inside a soft mask**:
- In four documents, a `/Luminosity` group stating `/CS /DeviceCMYK` paints `DeviceGray` images,
  fills or shadings.
- In the other two, a `/DeviceGray` mask group draws an isolated `/DeviceCMYK` group of black.

Every other group, the page groups included, moves a pixel by at most one level.

## The reading

**The mask's luminosity is not what moves.** §11.5.3 has two branches, the `Y` of CIE 1931 XYZ "[f]or
CIE-based spaces" and a grey "by implementation-defined means" for device ones. This reader takes the
device branch in every space (A23, ADR 2552), so the CMYK numbers weigh the same either way.
`a_cmyk_ink_masks_the_same_in_the_source_and_under_the_assumed_inks_intent` holds a mixed ink to it.

**The conversion between the grey and the CMYK moves.**
- **In the source,** the two are device spaces. They meet through §10.4.2.3's classic conversion,
  black alone, so a grey's mask is the grey again.
- **In the archive,** the group is the intent's profile (§8.6.5.7 NOTE 3, Annex P, and ISO 19005
  section 6.2.4.3's rule that a conforming reader uses the intent as `DeviceCMYK`'s source). §11.7.2
  then leaves a device colour of another count to be

  > converted or mapped to a CIE-based colour space in an implementationdependent fashion.

  This reader separates it by the profile's `B2A` (ADR 0796), cyan, magenta and yellow included, and
  EXAMPLE 2 weighs that separation differently from black alone.

**ISO 19005 says otherwise.** ISO 19005-2 and ISO 19005-4 section 6.2.4.3 fix the conversion for a
`DeviceGray` colour in a file whose PDF/A intent is CMYK and which sets no device-independent
`DefaultGray`: CMYK by the classic method, then through the profile. That draws the six masks as the
sources. It also draws every plain grey of such a file through the press's black rather than as
this reader's neutral grey, and the converter writes no `DefaultGray`.

## Decision

- **Not (b).** No statement keeps the source's mask together with its plain greys:
  - a `/DefaultGray` keeps the greys and brings back the colorimetric separation;
  - a `/TR` maps one mask value to another and cannot undo a separation;
  - a rewritten group `/CS` changes the space the producer's marks composite in.
- **Not (c).** The difference is not required. ISO 19005's own rule removes it.
- **(a) is the owner's.** It moves every grey of every PDF/A file with a CMYK intent, and a
  standard other than ISO 32000 decides it, so it is `Q788` with a recommendation. The renderer is
  unchanged meanwhile.
- **The report's sentence is corrected.** It said the page now blends in the profile's components,
  and it now says a soft mask mixing grey and `DeviceCMYK` content can draw differently.

## Not settled

ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 make the intent a conforming reader's
default blending space. This reader blends a page with no `/CS` in its own three components, which
§14.11.5's "free to disregard" permits. None of the six depends on it.
