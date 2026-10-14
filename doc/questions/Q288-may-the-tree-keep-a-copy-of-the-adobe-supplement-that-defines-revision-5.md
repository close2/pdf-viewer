# Q288 — May the tree keep a copy of the Adobe supplement that defines revision 5?

Source: the ADR-revisit sweep of 2026-09-16, re-verified 2026-10-07. The question is not new to
this round: ADR 0829 raised it inside an ADR and said so in as many words, ADR 1161 repeats that it
is "a separate argument", and a question raised only in an ADR is a question nobody can find
(`doc/questions/README.md`'s own rule) — so it is asked here.

## The question

`crypt.rs` reads revision 5 (ADR 0820), and the one step that separated this tree's reading from
pdf.js's and PDFBox's — preparing the password with SASLprep — rests on the *Adobe Supplement to
the ISO 32000, BaseVersion 1.7, ExtensionLevel 3* (Acrobat 9.0 SDK, June 2008), the document
Table 21 points at as "a deprecated proprietary Adobe extension". ADR 0829 fetched it once, from
the Internet Archive (snapshot `20100923045157`), and its sentences live on only as quotations
inside that ADR:

> Neither supplement is vendored. `doc/md/` holds the ISO and PDF Association documents this
> project is sponsored to keep; a copy of Adobe's is a licence question nobody has asked.

The supplement is still not on this disk, so the document a `/R` 5 file's reading rests on cannot
be checked by anything — the quotations gate verifies against `doc/md/`, and a document the tree
does not hold is invisible to it. ADR 1161 keeps the same door open on the writer's side: "`/R` 5
is not written … the owner wants the read side eventually, on the Adobe supplement, and that is a
separate argument."

## Why it cannot be settled without the owner

Holding third-party documents has always been the owner's word: `Q49` (ISO 32000-1 — the owner
obtained Adobe's free copy), `Q51` (the JPEG 2000 specifications — bought), `Q97` (Adobe's XFDF
specification — the owner downloaded it, and `A97` set the discipline: cite the section,
paraphrase, never quote, nothing committed beyond the citation). Whether Adobe's licence permits
keeping this supplement, and whether the owner accepts it, is not a round's to decide.

## What the tree does meanwhile

The reading stands: `crypt.rs` opens revision 5 over the census's 41 documents on ADR 0829's
recorded sentences, and the writer emits `/R` 6 only (ADR 1161). Nothing is blocked; what is
missing is a held document behind the citations — the same gap `Q49`'s resolution closed for
ISO 32000-1.

## Recommendation

**Fetch it and hold it the way `Q97` holds the XFDF specification** — under `A97`'s discipline,
cited by section and paraphrased, never quoted, kept beside the other gitignored specification
PDFs (`doc/third-party-data.md`). Adobe published the supplement for developers, and `Q49`
already established that its free copies can be held. If the licence turns out to forbid keeping
one, the fallback is what stands today, said in §7.6.4.2's row: the reading rests on ADR 0829's
recorded sentences, fetched 2026-08 from the Internet Archive's snapshot.