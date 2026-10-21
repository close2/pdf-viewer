# Q550 — Does an archived file that embeds a Foxit face carry its BSD notice?

Source: session 1655, ADRs 2146 and 2147, on the brief's ask to refuse where a licence does not
admit embedding and to say so.

## Why it needs the owner

`quorra-transform archive` embeds a face this program ships into a font the file names and does
not carry (`doc/questions/A47`, ADR 1209). Round 1655 extends that to the standard 14 stated by
name alone, the largest refusal of the real corpora. That puts the faces into many more output
files. The two licences read differently:

- **Liberation Sans, SIL OFL 1.1** (`data/standard-fonts/LICENSE_LIBERATION`). Its preamble says
  the requirement for fonts to remain under the licence "does not apply to any document created
  using the fonts or their derivatives", and the program's own `name` table carries its copyright.
  Nothing is owed for an unmodified copy, and the descriptor-less route embeds only that: it
  refuses where the face would have to be restated. **The older route does not.** Where the file
  states a descriptor and widths the face does not match, section 4.9's route 2 restates
  Liberation's `hmtx` (`pdf_font::restate`). That is a Modified Version under the OFL, which "may
  not use the Reserved Font Name(s)", and the restated program's `name` table still says
  `Liberation Sans`. The finding is the route's, not this round's.
- **The ten Foxit faces, BSD-3-Clause** (`data/standard-fonts/LICENSE_FOXIT`). Redistribution in
  binary form is permitted on condition that it "must reproduce the above copyright notice, this
  list of conditions and the following disclaimer in the documentation and/or other materials
  provided with the distribution". The programs carry no notice of their own (their CFF Top
  DICTs name `ChromSerifOTF` and the like, and no copyright string). Whether a font embedded in a
  document someone archives is a "redistribution in binary form" is a reading of a licence, and
  this project's licensing is the owner's (ADR 0819). The tree has embedded these faces on the
  same reading since A47 and has never carried the notice in the output.

## What the tree does meanwhile

It embeds both families, as it has since A47, and the output's `xmpMM:History` names the face
embedded and says it is "a face this program ships". It writes no notice.

## Recommendation

Two answers, both cheap.

Carry the notice in the output, which costs nothing a reader sees. ISO 32000-2 §14.3.2 lets any
stream carry a `/Metadata` stream. The program stream would gain an XMP packet whose
`xmpRights:UsageTerms` holds `LICENSE_FOXIT` verbatim. The reading is then the safe one, whatever
the owner decides "redistribution" means. One round's work, in `pdf-transform`'s archive
`program_stream`, for every Foxit face it writes. For a restated Liberation face, rename it in
its `name` table, which is the OFL's own condition for a modified version. Or refuse route 2 for
it, as the descriptor-less route already does.
