# ADR 1184 — Who §7.4.9's JPX baseline restriction binds, and what is actually left of the row

Status: accepted, 2026-09-22.

§7.4.9's ledger row has said `partial` "for two things: the clause's restriction on colour spaces
is not checked, and thirteen corpus codestreams still decode one level off the reference software"
since ADR 0161. The second is upstream's and is held by name. The first was read for the first time
here, against the clause rather than against the note, and it is not a debt at all.

## The three sentences, and the one addressed to us

§7.4.9 states its colour-space restriction three times, and the subjects differ:

- "Data used in PDF image XObjects shall be limited to the JPX baseline set of features, except
  for enumerated colour space 19 (CIEJab)." The subject is the **data**.
- "A JPEG 2000 image within a PDF file shall have one of: the baseline JPX colour spaces excluding
  enumerated colour space 19 (CIEJab); or enumerated colour space 12 (CMYK); or at least one ICC
  profile that is valid within PDF files." The subject is the **image**.
- "PDF processors shall support the JPX baseline set of enumerated colour spaces; they shall also
  be responsible for dealing with the interaction between the colour spaces and the bit depth of
  samples." The subject is the **PDF processor**, and what it is asked for is *support*.

The first two constrain what may be written; only the third is addressed to a reader, and it asks
for a decode rather than a refusal. **A reader that checked the first two would draw less than the
producer wrote**, which is exactly the argument this row already carries elsewhere in its note
for "[t]his filter shall only be applied to image XObjects, and not to inline images" — §7.4.7's
row makes it and is `implemented` with an inline `JBIG2Decode` image decoded rather than refused.
The same reading applies here for the same reason, and it applies to the clause's `/SMaskInData`
and channel sentences too, which are likewise "there shall be" statements about the data.

So "the restriction on colour spaces is not checked" describes a check no clause asks this program
for. It is struck from the row rather than left as debt somebody might build.

**This does not make CIEJab uninteresting**, and the row already says where it does matter:
`pdf-archive` makes exactly that test for ISO 19005 targets, out of `pdf_model::jpeg2000`'s `colr`
box reader (ADR 0925). A validator is asked whether a file conforms; a viewer is asked to draw it.
The two obligations are different and this tree carries them in the two places they belong.

## What is left, precisely

- **Thirteen corpus codestreams decode one level off `opj_decompress`**, and the discriminator is
  `qntsty`: the reversible 5/3 path is byte-identical and the irreversible 9/7 one differs by up to
  87 levels of 255. That is `hayro-jpeg2000`'s, written up in `doc/JPEG2000_FEEDBACK.md`, held by
  name in `tests/jpeg2000.rs` so an upstream release closing it fails the build.
- **Whether every baseline enumerated colour space is supported is unmeasured, and the set is not
  knowable here**: ISO/IEC 15444-2 is what defines "JPX baseline", and this project does not hold
  it (`doc/questions/Q51`). The processor sentence above is the one obligation of the three that
  lands on this tree, and it cannot be discharged against a definition nobody in the tree can read.

The three entries this round was asked to close against the decoder's output were already closed
and are re-checked here rather than re-implemented: `/ColorSpace` overriding the codestream's and
the channel count that follows from it (`tests/jpx_channels.rs`, ADR 0464), `/Decode` ignored on
the clause's own condition — `/ColorSpace` absent, not `/ImageMask` alone (`tests/jpx_decode_array.rs`,
ADR 0468) — and `/SMaskInData` reaching §11.6.5.2 with Table 87's codes 1 and 2.

**One correction to the brief that set this round**, so it is not repeated: the third of those is
not "`/Decode` ignored unless the colour space is indexed". The clause's bullet is "If ColorSpace
is absent, then the Decode array shall be ignored unless ImageMask is true", and an `Indexed` space
is one of the places the array carries the space's own units rather than a condition on it.
