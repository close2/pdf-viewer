# Q857 — May PDF417 be built from Public.Resource.Org's ISO/IEC 15438, and will you buy ISO/IEC 16022?

Source: round 2030 (ADR 2877), carrying out `doc/questions/A840` item 3. Nothing waits on the answer: a form that
types `pdf417` or `dataMatrix` is refused by name, and Code 39 is built.

## What was found

- **PDF417.** The patents print no codeword tables (US 5,243,655), or a pre-standard PDF417 whose tables, modes
  and checksum differ from the standard's (US 5,304,786, 5,399,846). The complete text found is **ISO/IEC
  15438:2006** as published by Public.Resource.Org (`law.resource.org/pub/us/cfr/ibr/inc/iec/iec.15438.2006.pdf`),
  incorporated by reference into US federal regulation. Its front matter says the symbology is in the public
  domain; its pages carry ISO's copyright notice. A round fetched it outside the tree and has not built from it.
- **Data Matrix.** GS1's guideline and general specifications refused an automated download; US 5,612,524 is the
  pre-ECC 200 family; NASA-STD-6002D prints the size table and nothing of the encodations, the Reed-Solomon field or
  the module placement. Those are in **ISO/IEC 16022** alone.
- **Code 39** is built from MIL-STD-1189A, which prints no check character and no full-ASCII table: `checksum="auto"`
  and `code3Of9extended` are refused. **ISO/IEC 16388** or MIL-STD-1189B (free, but everyspec.com refuses an
  automated download) would close both.

## The questions

1. **PDF417**: may a round build from Public.Resource.Org's copy of ISO/IEC 15438:2006 — the codeword tables carried
   as data, the text cited and paraphrased, the PDF kept out of the tree — or will you buy ISO/IEC 15438:2015?
2. **Data Matrix**: will you buy ISO/IEC 16022:2024 (or fetch GS1's free DataMatrix Guideline by hand, which gives
   the sizes but not the placement)?
3. **Code 39**: will you fetch MIL-STD-1189B by hand (free) or buy ISO/IEC 16388:2023, for the check character and
   the extended set?

## Recommendation

1. Buy ISO/IEC 15438:2015 if the project's rule against redistributing standards it is not licensed for (ADR 0187)
   is read as covering building from a copy it did not license; otherwise allow Public.Resource.Org's copy, which
   is the one US law publishes. The round's lean is the purchase: it is the reading that cannot be wrong.
2. Buy ISO/IEC 16022:2024 — no public text defines the placement.
3. Fetch MIL-STD-1189B by hand into `~/scratch/agent/specs/`; it costs nothing.
