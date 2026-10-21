# 2162 — A font written inside its resources becomes an object, and a machine's face is the loss `machine-face`

Session 1663, the archive slot of batch one hundred and three. Status: **accepted**, **built** in
`pdf-transform`. It **answers** two of the refusals ADR 2146 left by name: the font that is no
object, and the font this machine draws from a face of its own. Context: ADRs 0270, 2146, 2147;
`doc/todo/72` section 3; `doc/history/1655`. Code: `fonts::gains`, `Substitutes::direct`,
`Rewriter::reference_direct_fonts`, `described::Drawn`, `decision::embedded_faces`,
`Loss::MachineFace`.

## 1. A font written directly inside its resource dictionary

ADR 2146 gives a standard font stated by name alone a descriptor, widths and, where its face asks
for it, a `TrueType` subtype. Those are entries added to the font's object, so a font written
directly inside a `/Font` subdictionary had nothing to add them to and was refused. The corpora's
handbuilt test files, among others, write their fonts that way: 8 pdf.js and 71 `doc/corpora`
documents at 2b.

**Decision.** The font is written out as an object of its own, from a spare number, holding its
own entries and the ones it gains. The rewriter then descends every dictionary and array of every
object and puts a reference wherever a dictionary equal to it stood. ISO 32000-2 §7.3.10 makes the
two spellings one value:

> Except where documented to the contrary, any object value may be a direct or an indirect
> reference; the semantics are equivalent.

No clause documents the contrary for a `/Font` subdictionary's entries. Where a clause does speak,
it asks for the indirect form: Table 109 makes `/FontDescriptor` "an indirect reference", and the
graphics state's `/Font` entry wants "an indirect reference to a font dictionary". Equal
dictionaries are one value under the same sentence, so a font inherited by many pages, or written
again in a form's resources, is written out once and every place it stood gets the one reference.
The same route serves ADR 2147's restatement, whose `/Subtype` and encoding are entries too. The
proof is ADR 2146's, unchanged.

## 2. A font this machine draws from a face of its own

ADR 0270 lets the viewer take a machine's face for a font the file does not carry. That happens by
name (`substitute::find` consults the machine first for a name outside the fourteen, so
`TimesNewRomanPSMT` and `ArialMT` reach installed faces), or by declared repertoire (a wider face
of the family where a declared code is unanswered). ADR 2146 refused such a font: the face drawn is
not ours to embed, since §9.9.1 makes embeddability a fact about a licence, and embedding the
shipped one draws other shapes. 5 pdf.js and 36 `doc/corpora` documents at 2b.

**The reading.** The file states no face. §9.5's NOTE 5 says that with programs not embedded "the
results depend on the availability of fonts in the PDF processor's environment", and that font
substitution is implementation-dependent. Embedding the shipped face loses nothing the file states,
provided that face has a glyph for every code the pages show. What it loses is this machine's
shapes. That makes it a loss with a name, and each profile's yardstick answers it:

- **`as-if-printed`: `discard`.** A printed copy is drawn in whatever face the printing machine
  had, so paper made elsewhere would not have kept this machine's shapes either. Every character
  stays, at its width.
- **`as-shown`: `stop`.** The window drew those shapes. An operator who wants them kept names the
  machine's face with `--font`, which is the statement §9.9.1 asks for. No owner's word is needed,
  and Q558 is not asked.
- **A code shown that the shipped face lacks is refused under both.** That is a mark lost, which no
  profile's bound admits.

**The proof.** ADR 2146's proof compares outlines with the source as drawn, which is the machine's
face, and so cannot hold here. `described::Drawn::FromAMachineFace` holds the converted font to the
**shipped face's own glyph** for every shown code instead: the same program on both sides, so the
same glyph index is the same outline. It also requires all 256 advances to be the source's, the
program's own, and no stretch. The widths check reads the shipped face's glyph for the same reason.

**Decision.** `Loss::MachineFace` (`machine-face`) is the conditional answer of
`fonts/font-programs-embedded` (`decision::CONDITIONAL`). The row is a statement as before, unless
the preparation embedded a shipped face over a machine's face on the descriptor route (ADR 2146)
or the restatement route (ADR 2147). **Not on the third route**: a font that states a descriptor
and needs no restating already embeds the shipped face over a machine's face as a statement. On
this machine that takes in every TrueType name `substitute::find` matches to an installed face.
Naming it is owed (`doc/todo/72`), because pricing what it refuses under `as-shown` is its own
census.

## 3. Result, the real corpora

`as-if-printed` at 2b, converted / conforming / refused / unreadable, HEAD → this ADR, binaries
built from an export holding only this round's hunks:

| corpus | 2b | 4 | 2b `as-shown` |
|---|---|---|---|
| pdf.js (974) | 751/0/212/11 → 757/0/206/11 | 697/0/266/11 → 702/0/261/11 | 747/0/216/11 → 752/0/211/11 |
| `doc/corpora` (503) | 266/0/221/16 → 345/0/142/16 | 235/0/252/16 → 309/0/178/16 | 265/0/222/16 → 324/0/163/16 |

Of the 79 direct-font documents, 64 convert. The rest stop on refusals of their own: six fonts not
one of the fourteen, six headers, three operators. Of the 41 machine-face documents, 21 convert
under `as-if-printed`. Under `as-shown` they stop at the loss. One more (`316267.pdf`) is refused by
the output's second verdict, `fonts/widths-agree-with-the-program`, which is not yet diagnosed. The
others stop elsewhere, two at a shown code the shipped face lacks.

**Pixels.** On this machine the faces drawn are mostly installed Liberation fonts, so the shapes
hardly change: `176446.pdf` page 1 is byte-identical at 100 dpi, and `008677.pdf` differs at 1310
pixels by 1 level of 255 (ADR 0396's winding). The direct witness
`T02-01_006_document-catalog-wrong-type-key.pdf` is byte-identical. veraPDF 1.30.3 at 2b passes all
six converted witnesses.
