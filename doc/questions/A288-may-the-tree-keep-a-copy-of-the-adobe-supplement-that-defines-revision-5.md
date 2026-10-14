Status: partial
Given: 2026-10-08, in conversation — transcribed by the round
Owes: the `doc/third-party-data.md` row for the held supplement — the file was not writable by this session's account when this answer landed, so the row is owed by the first round that can append it, citing this file

> For Q288 I agree with your recommendantiation, please answer it.

Reading: the recommendation adopted is the question's own, quoted and attributed: *"Fetch it and
hold it the way `Q97` holds the XFDF specification — under `A97`'s discipline, cited by section
and paraphrased, never quoted, kept beside the other gitignored specification PDFs."*

Done in the same sitting as the answer, so only the manifest row named in `Owes` remains:

- **Fetched and held** at `doc/AdobeSupplement-EL3-2008.pdf` — *Adobe® Supplement to the ISO 32000,
  BaseVersion: 1.7, ExtensionLevel: 3*, Adobe® Acrobat® SDK, June 2008, Version 9.0 — from the
  Internet Archive's snapshot `20100923045157` of
  `adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/adobe_supplement_iso32000.pdf`, the same copy
  ADR 0829 fetched when it settled the SASLprep step. **1 373 256 bytes**, the size ADR 0829
  recorded; SHA-256 `50d0bbc96028dca1a54cf3c671054536d26e9494dc52877061763b4aed08ac48`.
- **Gitignored** with every other specification PDF (`/doc/*.pdf`, `.gitignore:46`), so nothing
  is committed beyond the citation.
- **Converted** by `tools/spec-md.py` into the ignored `doc/md/AdobeSupplement-EL3-2008.md`
  (140 pages, 54 869 words), so the quotations gate can check a citation of it like any other
  held document — the gap Q288 was asked about.

**`A97`'s discipline binds `crypt.rs`'s §7.6.4 citations from here**: the document is Adobe's,
single-reader — cited by section and paraphrased, never quoted. The sentences ADR 0829 recorded
from its fetched copy, including the two that state a revision-5 password is prepared with
SASLprep, now rest on a held document instead of on one round's fetch.