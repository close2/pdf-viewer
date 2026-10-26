# 2362 — A new object is numbered past every number the sections stated

Status: implemented. Session 1764, 2026-10-26, the writer slot of batch one hundred and sixteen.
Clauses: ISO 32000-2 §7.5.4, §7.5.5 (Table 15), §7.5.6. Context: ADRs 0100, 0789, 2346 §5. I
worked in `scratchpad/r1764/`.

## 1. What the writers asked, and where it was wrong

Five writers number the first object they add. `pdf-model`'s `ViewState::allocate` and
`writing::Update::beside` and `pdf-transform`'s `attachments` and `update` took the larger of
`object_numbers().max() + 1` and the trailer's `/Size`. `archive/prepare.rs`'s `Spare` started at
`object_numbers().max() + 1` and skipped any number the document resolves. `object_numbers` yields
the numbers the reader *kept* in use, and three kinds of number a section states are not among
them:

- **A free entry.** §7.5.4's `f` and §7.5.8.3's type 0 are recorded as deletions (ADR 0100) and
  not yielded. §7.5.4 says a freed number's next use carries a higher generation: "The entry's
  generation number shall be incremented by 1 to indicate the generation number to be used the
  next time an object with that object number is created." Every writer here writes generation 0.
- **An entry declared and unread.** A subsection header states "the number of entries in the
  subsection", and an entry that will not lex is still a number the file stated. ADR 0789 lets
  `Document::load_by_header` find its object by its own `N G obj` header.
- **A table a rebuild replaced.** `xref::read` scans when the table names no object, and
  `Document::open` scans when the table names no catalog. The sections are still in the file.

Where `/Size` understated such a number, the writer gave it to a new object. §7.5.6 leaves the
original contents intact, so a reference in the file that named the old number now names the new
object. Both `tests/update.rs` plants failed on HEAD. With an unread entry for object 3 and
`/Size 3`, the new information dictionary became object 3, and the catalog's `/Keep 3 0 R`, which
named the string `(kept)`, named the dictionary. With object 3 freed at generation 1, the dangling
`/Keep 3 0 R` came to name the dictionary.

## 2. The construction

`XrefTable::highest_stated_number` is the largest of: every number any section filed, free
entries included; the end of every declared and unread range; and the highest number a table
replaced by a rebuild had stated, carried by `keep_stated` from both rebuild sites.
`XrefTable::next_object_number` is that plus one, or `/Size` where `/Size` is larger. The five
writers ask it, and nothing else in them changed. `Spare` still skips a number that resolves.

What it does not see: a section this reader could not read at all. A cross-reference stream whose
`/W` has no offset field is refused whole (ADR 1004), and the `/Prev` chain stops there. The
numbers of a section that was never read are not known to this reader.

## 3. What moved, counted

`cargo run --profile gates -p pdf-syntax --example next_number_census -- <files>` prints both old
rules beside the new answer. Over the pdf.js corpus, the veraPDF corpus and the specifications
under `doc/`, 3 910 documents opened. The incremental writers' number moved for three of them:
`outline_goto_action.pdf`, whose twelfth entry is declared and absent, and veraPDF's
`6-1-4-t02-pass-a.pdf` and `-b.pdf`, each with a free entry 18 under `/Size 18`. None of the three
resolves the number, so HEAD would have written over a deletion rather than over a live object.
For every other document the number is the one HEAD gave, so the output of each of the four
incremental writers is unchanged.

The conversion's number moved for 346 names, because `Spare` never asked `/Size`. Its output does
not carry the number. `archive/rewrite.rs`'s walk gives each object an output slot in the order
it reaches it. HEAD's and this tree's `quorra-transform` were run on all 361 files with those
names, to 2b and 4, with every loss authorised. The written files, the refusals and the reports
were byte-identical in every pair. The record has the count of pairs where an output intent was
added and so a spare number was taken.

## 4. Decision

One source of the number, in `pdf-syntax` beside the reader that knows what was stated. A
saturated `u32::MAX` stays the writers' answer for a file that states that number, as it was.
