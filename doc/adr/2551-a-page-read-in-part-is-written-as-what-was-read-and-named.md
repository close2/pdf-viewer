# 2551 — A page read in part is written as the entries the reader drew it from, and is named

Status: implemented. Session 1858, Slot 8 of batch one hundred and twenty-six (two sessions).
Amends ADR 2502 decisions 6 and 8; ADR 2503 is the record of what decision 6 refused. Context: ISO 32000-2
§7.3.7, §7.7.3.3 (Table 31), §7.7.3.4; ADR 0784 (a prefix of a dictionary is a subset); ADR 2550 holds the
other three documents of the slot. Code: `crates/pdf-transform/src/archive/recovered.rs` (`PartlyRead`,
`RecoveredStructure::read_in_part`, `pages_to_carry`), `crates/pdf-transform/src/archive/report.rs`
(`read_in_part` in the JSON), `crates/pdf-transform/src/archive/mod.rs`.

## The document

`poppler-742-0-fuzzed` states `/Count 1` and one `/Type /Page`. Its root's `/Kids  8 0 R ]` lacks the
opening `[`, so this reader finds the page by its recovery scan. Page 8's dictionary stops being
readable at byte 678, where a `(` inside `/TrimBox` opens a string that never closes; seven entries
were read whole, `/MediaBox` among them, and `/Contents` is not one of them (the file spells a later key
`/Cntents`). The viewer draws the page from those seven entries and says so (`Unsupported::PageDictionary`,
ADR 0784): a blank page on the producer's own sheet. ADR 2502 decision 6 refused any source with "a page
that is no indirect dictionary this reader parses whole", so R01 declined the one page the reader drew.

## The reading

§7.3.7 bounds a dictionary by its closing `>>` and by nothing else, so a reader that never reaches it
cannot know how many entries the producer wrote, nor whether the bytes after the damage were entries at
all. ADR 0784 drew the consequence for the viewer: the prefix is a subset of what the producer wrote, it
is taken only where the entries read state Table 31's `/Type /Page` themselves, and every entry the
damage took gets Table 31's default with a report. The archive has the same choice the viewer had, and
the same answer is open to it: state the subset the page was drawn from, and say that it is a subset.
Refusing it leaves the company with nothing, which is the outcome R01 exists to remove.

## Decided

1. **A page read in part is carried as the entries the reader drew it from.** `pages_to_carry` writes it
   as `Carried::Direct(with_resolved_entries(page))`: the entries read whole, with the values of the
   inheritable entries in force (§7.7.3.4) stated on it, as every rebuilt page's are. Nothing after the
   damage is written, and nothing is invented for it.
2. **It is named.** `RecoveredStructure::read_in_part` lists each such page as `PartlyRead { page,
   entries, stopped_at }`. The report's sentence, the JSON's `read_in_part` and the output's own
   `xmpMM:History` each say that page *n*'s dictionary "stops being readable at byte *b* of the source,
   and is written as the *k* entries before it, which this reader drew the page from; whatever its
   producer wrote after them is not in the archive". The loss stays `recovered-structure`: what is lost
   is the part of the structure this reader did not recover.
3. **ADR 2502 decision 6** loses "one with a page that is no indirect dictionary this reader parses
   whole"; the other refusals stand.
4. **ADR 2502 decision 8**'s parenthesis now reads: a page nested deeper than the parser's 255 arrays
   stops being readable at that value and is written as the entries before it. `carry`'s depth check at
   257 stays the backstop for a value no parser built.

## The outcome

`poppler-742-0-fuzzed` is rebuilt: one page stated, one drawn, one written, from seven entries. It is now
refused by ordinary rows rather than by the rebuild: `file-structure/stream-filters-are-standard` at 2b
and 4 (image 10's `/DCTecode`, reachable from `/Resources 2 0 R` although no content invokes it) and two
metadata rows at 4. R01 has no document left whose page this reader draws and the rebuild declines.

## Consequences

- The bounds are the parser's own: the prefix is what `damaged_dictionary` returns, read once, and the
  page count is the reader's.
- A test builds a page whose fifth entry opens an unclosed string before `/Rotate 90`: four entries
  are written, `/Rotate` is not, and the content stream is the producer's byte for byte.
