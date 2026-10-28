# 2514 — The remedy table is the join of the requirement rows and the losses, and the blocking list holds problems no requirement states

Status: accepted — the table's population, its authority and its reading of ISO 19005-2 section
6.8; the loss sort and the two profiles it defines are RFC 0014's proposals, asked in
`doc/questions/Q706`. Session 1840, the design slot of batch 125. Context: the owner's words of
2026-10-27 asking for "a big table with all possible problems, and possible remedies";
`doc/pdfa-remedies.md` (the table), RFC 0014 (the format it feeds), RFC 0007 sections 3 and 4.6,
`doc/pdf-a-mitigations.md`, `doc/pdfa-blocking-issues.md`, ADR 2490.

## What was read

The batch brief's hypothesis was that the converter's remedy sites, its loss words and the blocking
list's reason families are three views of one set, so that the table is their join. Read against
`quorra-transform archive --remedy-sites` at all six targets, `pdf_archive`'s requirement tables,
`decision.rs`'s three answer tables and loss words, and `doc/pdfa-blocking-issues.md`'s families:

- every configurable site is a requirement row of `pdf_archive` — none lies outside its tables;
- every loss word is taken at a site or by a profile-level key (`recovered-structure`,
  `text-as-image` under `as-shown`);
- **the blocking reasons are not a view of the same set.** A document declined as a source (R01),
  a header no version names (R06a), a filter no reader decodes (R07), the encrypted, the pageless
  and the dynamic XFA form stop a document before any requirement is judged or are the document's
  rather than a requirement's; and a requirement may refuse without being a site a configuration
  can answer.

## The decision

1. **The table has three parts**: the requirement rows, the problems no requirement states, and the
   losses. A requirement row is written for every requirement whose check is `Implemented` — the
   only kind a file can fail — and is a site, or is refused, or takes a loss, or has a remedy the
   catalogue argues and no code carries out. A requirement that is a processor's obligation,
   unchecked or outside validation is listed by name on one line per family, and so is one every
   profile answers without loss; neither is a row, because neither is a problem an operator meets.
2. **The listing is the authority.** Whether a site's remedy is carried out is what `--remedy-sites`
   prints, because it is generated from the table the converter decides from; a cell of the table
   that disagrees is corrected by a small edit, and the table states this in its opening. The
   catalogue's own status words are carried for the remedies no listing names (`--font`, a shipped
   face), which the listing cannot see because they are command-line inputs rather than answers.
3. **A departure is counted, not listed**, per row: its classes are argued per entry in
   `doc/pdf-a-mitigations.md` section 4.7.5, and a remedy that goes against the requirement is a
   different thing from one that satisfies it.
4. **ISO 19005-2 section 6.8 admits an attachment that conforms to part 1 or part 2.** RFC 0007
   section 4.6's table reads that a derived PDF/A is no remedy at PDF/A-2; the section's narrower
   rule is about *which* parts an attachment may conform to, not whether a derived one may be
   attached, so a derived file converted to part 2 is a remedy there. The table and RFC 0014
   section 5 carry the corrected reading; RFC 0007 is a record and keeps its sentence.

## Consequences

- An operator writing a profile and a round choosing a build read the same rows; the rank of what
  to build stays `doc/pdfa-blocking-issues.md`'s, and the table points there rather than ranking.
- The table's cells decay as rows are built. That is accepted for a design table: section 6 of it
  names the four commands that re-derive every cell, and the listing wins where they differ.
