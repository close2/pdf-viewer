# The sRGB ICC profile this program ships

`sRGB2014.icc`, 3 024 bytes, SHA-256
`384b832de3412066743b52a75ee906b6fb9fb8d9e09e936fc2c43223815c6e0a`.

Downloaded on 2026-09-10 from the International Color Consortium's own registry,
<https://registry.color.org/rgb-registry/profiles/sRGB2014.icc>, and **not altered** — which the
licence requires and which the hash above is here to let anyone check.

## Why this file and not one of the other three

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

## The conformance claim above, checked against the text rather than asserted

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

## The licence, read first-hand

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
