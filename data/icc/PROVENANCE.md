# The ICC profiles this program ships

Two, one per colour family a PDF/A output intent can be asked for: an RGB one and a CMYK one.
`doc/questions/A18` decides that they are shipped at all — *ship the standard profile, with a flag
to override it* — and `doc/adr/1153` decides which CMYK profile, on the same clause this file
reads for the sRGB one.

## sRGB

`sRGB2014.icc`, 3 024 bytes, SHA-256
`384b832de3412066743b52a75ee906b6fb9fb8d9e09e936fc2c43223815c6e0a`.

Downloaded on 2026-09-10 from the International Color Consortium's own registry,
<https://registry.color.org/rgb-registry/profiles/sRGB2014.icc>, and **not altered** — which the
licence requires and which the hash above is here to let anyone check.

### Why this file and not one of the other three

The ICC offers four sRGB profiles. `doc/questions/A18` says to ship the standard one, and *which*
one is decided by the clause rather than by preference.

ISO 19005-2 section 6.2.4.2 requires the profile forming an `ICCBased` stream to conform to one of
four named ICC texts: ICC.1:1998-09, ICC.1:2001-12 (profile version 4.0.0), ICC.1:2003-09
(version 4.1.0), or ISO 15076-1 (version 4.3.0). **Version 4.2.0 is not among them**, and that is
exactly what `sRGB_v4_ICC_preference.icc` is — so the obvious download is the wrong one for a
PDF/A-2 target.

This profile's own header says version **2.0.0.0**, which conforms to ICC.1:1998-09, the first
text on that list. Read out of the file rather than off a web page:

| field | value |
|---|---|
| size | 3 024, and the header's own length field agrees |
| signature | `acsp` |
| profile version | `02000000` — 2.0.0.0 |
| device class | `mntr` |
| data colour space | `RGB ` |
| profile connection space | `XYZ ` |
| `desc` | sRGB2014 |
| `cprt` | Copyright International Color Consortium, 2015 |

ISO 19005-4 section 6.2.4.2 sends the same question to ISO 32000-2 §8.6.5.5, which admits this
version too, so one file serves all six targets.

### The conformance claim above, checked against the text rather than asserted

**When this file was written ICC.1:1998-09 was not held, and the sentence "conforms to
ICC.1:1998-09" was a claim nobody could check.** The text arrived in the nine-hundred-and-fiftieth
session and the claim survives, on five things — the argument is `doc/adr/0950` section 6, and the
summary is here so that a reader of this file need not go looking:

- **2.0.0 is an earlier revision than the 2.2.0 that text states as its own**, and that does not
  matter, because neither ISO 19005-2 nor ICC.1:1998-09 requires a profile to *state* the number of
  the edition it conforms to. Part 2's sentence is a disjunction over documents; section 6.1.3
  describes the version field and says what the current number is.
- **Nothing required was added inside major version 2.** Section 6.1.3 makes a major version change
  the one that carries incompatible changes — new required tags is its own example — and Annex F
  lists what the 2.1.0 and 2.2.0 revisions actually changed: optional tags and clarifications.
- **The nine tags Table 27 requires of an RGB display profile are all here** — `desc`, `rXYZ`,
  `gXYZ`, `bXYZ`, `rTRC`, `gTRC`, `bTRC`, `wtpt`, `cprt` — and a test in
  `crates/pdf-archive/src/table/graphics.rs` asserts it against these bytes, under ICC.1:2001-12's
  Table 25 as well.
- **The sixteen non-zero bytes at 84 to 99 offend nothing.** They are a `Profile ID`, a field ICC.1
  did not define until version 4; that text's Table 9 gives bytes 84 to 127 to reserved expansion
  and, unlike every other reserved field in the document, states no requirement that they be zero.
- **The `chad` tag is not a defect either**, though ICC.1:1998-09 does not define
  `chromaticAdaptationTag`: its clause 6.2 admits tags beyond the ones it defines, asking only that
  each signature be registered with the ICC, and every tag type in this file — `desc`, `XYZ `,
  `curv`, `meas`, `sig `, `text`, `sf32` — is one that text defines.

What is *not* claimed is conformance in every respect: that would take reading the whole of
ICC.1:1998-09 against these bytes rather than its clause 6.3.

### The licence, read first-hand

The ICC's profile library states, for profiles it owns the copyright in:

> This profile is made available by the International Color Consortium, and may be copied,
> distributed, embedded, made, used, and sold without restriction. Altered versions of this
> profile shall have the original identification and copyright information removed and shall not
> be misrepresented as the original profile.

The v4 profiles beside it carry a narrower grant — distribution permitted provided the file is
unchanged including the ICC copyright notice tag, and the ICC's name not used in advertising. This
file is shipped unchanged either way, and the `cprt` tag above is intact, so both readings are
satisfied.

**This was checked against the ICC's own pages rather than taken from this project's earlier
notes.** `doc/pdf-a-conversion-limits.md` §10 and `doc/rfc/0006` §5.3 both recorded the grant
second-hand; a second-hand report of a licence is not a licence, and `doc/questions/A51` made the
same point about a different document.

## CMYK

`GRACoL2006_Coated1v2.icc`, 2 747 956 bytes, SHA-256
`7f822b354f3f4081e3b1dadd381c9f16d583257d1574e583be7d0e3433a6b45b`.

Downloaded on 2026-09-21 from the International Color Consortium's own profile registry,
<https://registry.color.org/profile-registry/profiles/GRACoL2006_Coated1v2.icc> (registry entry
<https://registry.color.org/profile-registry/GRACoL2006_Coated1v2>), and **not altered** — which
the licence below requires by name and which the hash above is here to let anyone check.

Read out of the file rather than off a web page:

| field | value |
|---|---|
| size | 2 747 956, and the header's own length field agrees |
| signature | `acsp` |
| profile version | `02000000` — 2.0.0.0 |
| device class | `prtr` |
| data colour space | `CMYK` |
| profile connection space | `Lab ` |
| PCS illuminant | 0.964 203, 1.0, 0.824 905 — D50 |
| created | 2007-05-24 13:55:39 |
| tags | `wtpt`, `bkpt`, `cprt`, `A2B0`, `A2B1`, `A2B2`, `B2A0`, `B2A1`, `B2A2`, `gamt`, `chad`, `desc` |
| `desc` | GRACoL2006_Coated1v2.icc |
| `cprt` | the licence quoted below |

### The edition argument, made again rather than carried over

The sRGB section's argument is about an **RGB display** profile and this is a **CMYK output**
profile, so it is re-made here against the same clause and the same two held texts rather than
assumed to transfer. What transfers unchanged is the half that is about the version *number*:
ISO 19005-2 section 6.2.4.2 names four ICC texts — ICC.1:1998-09, ICC.1:2001-12 (version 4.0.0),
ICC.1:2003-09 (4.1.0) and ISO 15076-1 (4.3.0) — and asks the profile to conform to one of them
without asking it to *say* which, so a header stating 2.0.0 is read against ICC.1:1998-09, the
first of the four. `doc/adr/0950` section 6 is that reading and it does not depend on the class.

What does **not** transfer is the tag list, because each text states a different one per class.
ICC.1:1998-09's Table 29 (its clause 6.3.3.2, colour output profiles) requires
`profileDescriptionTag`, `AToB0Tag`, `BToA0Tag`, `gamutTag`, `AToB1Tag`, `BToA1Tag`, `AToB2Tag`,
`BToA2Tag`, `mediaWhitePointTag` and `copyrightTag`. **All ten are present.** ICC.1:2001-12's
Table 28 states the same ten and adds `chromaticAdaptationTag` where the illuminant is not D50;
this profile's illuminant *is* D50 and it carries `chad` anyway, so it satisfies that text under
either branch of the condition.

This crate's own reading is narrower than either table and the profile passes it too. For a
`prtr` class, `missing_required_tags` in `crates/pdf-archive/src/table/graphics.rs` asks under
those two editions for `desc`, `cprt`, `wtpt` and one of `kTRC` or `A2B0` — the monochrome form or
the lookup-table form, which are the two shapes clause 6.3.3 gives an output profile. This profile
carries `desc`, `cprt`, `wtpt` and `A2B0`, so **`missing_required_tags` is empty for it under both
held editions**, and a test in that module asserts it against these bytes the way the sRGB one is
asserted.

`bkpt` is a tag ICC.1:1998-09 defines (`mediaBlackPointTag`) and neither table requires; carrying a
tag a text defines and does not require offends nothing, which is clause 6.2's own position on tags
beyond the required set.

ISO 19005-4 section 6.2.4.2 sends the same question to ISO 32000-2 §8.6.5.5, which admits version 2
profiles too, so this one file serves the part 4 targets as it serves the part 2 ones.

### The licence, quoted from the file's own `cprt` tag

This text is the profile's own and not an ISO text, so it is quoted rather than paraphrased:

> Copyright X-Rite, Inc.. This profile is made available by IDEAlliance, with permission of X-Rite,
> Inc., and may be used, embedded, exchanged, and shared without restriction.  It may not be
> altered, or sold without written permission of IDEAlliance.

Embedding it in an output intent is *embedding*, which the grant names. Shipping it in this tree is
*sharing*, which the grant names. The one prohibition that binds is **not altered**, and the hash
above is how anyone checks that it has not been. The report a conversion prints carries this
sentence verbatim from the tag, so a person who embeds it is told whose profile it is and on what
terms — which is `doc/pdf-a-conversion-limits.md` section 10.1's rule applied to the profile this
program supplies rather than only to one an operator hands it.

### The alternatives, and why each was declined

The registry offers several licence-clean CMYK profiles. Which one is shipped is decided by the
same clause that decided the sRGB one — ISO 19005-2 section 6.2.4.2's four editions — and then by
the licence.

| profile | why not |
|---|---|
| `PSOcoated_v3` (FOGRA51, ECI) | version 2.4.0, so the edition question is fine; its licence forbids distribution without ECI's written permission. It stays a profile an operator supplies with `--output-intent-profile`. |
| `GRACoL2013_CRPC6`, `CGATS21_CRPC1`–`CRPC7`, `SWOP2013C3_CRPC5` (IDEAlliance 2013) | licence-clean, and version **4.2.0** — which is exactly the number the sRGB section above argues is absent from section 6.2.4.2's four texts. The same argument that rejects `sRGB_v4_ICC_preference.icc` rejects these. |
| `SWOP2006_Coated3v2`, `SWOP2006_Coated5v2` | same licence, same version, equivalent standing — they describe web offset rather than sheetfed commercial printing, and one of the set has to be chosen. GRACoL 2006 is the broader of the two conditions. |
| `Coated_Fogra39L_VIGC`, `JapanColor2011Coated`, `SC_paper_eci` | each carries a copyright notice with **no grant** in its `cprt` tag. A notice without a grant is not a licence, which is the point `doc/questions/A51` made about a different document. |
