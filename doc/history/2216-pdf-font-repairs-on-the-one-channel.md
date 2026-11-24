# Round 2216 — `pdf-font`'s repairs on the one channel

The silent-repair campaign's third fix round (ADR 3477, review 2194), in the `lane3` worktree.

## What changed

- `pdf_font::repair::CLASSES`, 16 classes (ADR 3565): the program's (`loca` extent, format and
  order, a table past the program, a collection cut to one face, a `/Subtype` the bytes contradict,
  rewritten CFF Font DICTs, an unreadable program set aside), the maps' and the dictionary's
  (`/ToUnicode` not a stream or damaged, an embedded `CMap` damaged, `/Widths`, `/W` and `/W2`
  malformed, `/Differences` malformed, an `/Encoding` name outside the three) and one bound.
- The loader records on the document it holds; the byte parsers return what they repaired. A
  `loca` format that both lengths contradict is now corrected, and asked before the extent.
- The validator loads every font in `read_whole`. `MAPPING` (ADR 3566): all to 5.1 but the bound;
  the widths classes to 6.2.11.5 too, read only at the fonts a page renders (`record_logged_at`).
  The program classes went to 6.2.11.4.1 in a first census, which refused ten pdf.js documents
  "not built yet" for want of a font with no program; they are 5.1's until a route writes a
  program as read. A unit test holds the font classes to the mapping and the table.
- The converter (ADR 3567): every font class is at an object; what a route replaces is answered,
  the rest is refused by the output's check under the class's name.
- 2194-B05: two tests pass, their entries removed from both manifests; B05 and T03 `fixed`.

**Census.** Without the pack, as-if-printed pdf.js 2b converts 909 and refuses 50, corpora 438 and
40. Fifteen pdf.js documents are refused by the output's 5.1 under a font class, none at the
corpora: IdentityToUnicodeMap_charCodeOf, bug1650302_reduced, bug859204, bug920426,
issue11131_reduced, issue17671, issue2537r, issue3323, issue3405r, issue4402_reduced,
issue7074_reduced, issue7200, issue8480, issue9278, javauninstall-7r. The first cut's 6.2.11.4.1
mapping refused ten of them "not built yet" instead. With the pack: pdf.js 2b 910 and 49 (the pack
converts issue15053), corpora as without, the same fifteen refused.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 over the five crates; `pdf-sandbox --bins` 0; nextest
of the five 0, 5219 of 5219, no failure against an empty manifest; rustdoc 0 on `pdf-font` (after
two private links were unlinked) and `pdf-archive`; fuzz `cargo check --bins` 0; conformance 0 once the gates were a paragraph.

Left: T04's hinting instance (F062) and the drawing-time tolerances F061–F063, the descriptor's
numeric fallbacks (F044–F049, F064–F068), the TrueType `cmap` routes (F054–F056) and the program
table readers (F074–F086); a converter route that writes a repaired program as read.
