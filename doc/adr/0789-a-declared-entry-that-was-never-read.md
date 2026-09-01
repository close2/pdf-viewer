# ADR 0789 — A declared entry that was never read is not a deletion

Status: accepted. Session 864.
Clauses: ISO 32000-2 §7.5.4, §7.5.6, §7.5.8.2, §7.3.10.
Code: `crates/pdf-syntax/src/xref.rs`, `crates/pdf-syntax/src/document.rs`.
Tests: `crates/pdf-syntax/tests/cross_references.rs::an_entry_the_reader_cannot_read_is_not_a_deletion`,
`::a_cross_reference_stream_shorter_than_its_own_index_says_what_it_lost`,
`doc/checks/fixed-documents.toml` (`cairo-85141-3.pdf`).

## The witness

`corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-3.pdf`, a matplotlib figure filed
against cairo, opened, counted one page, and had none — the state
`pdf-model/examples/standing_count_census` was built to enumerate, and the one cause that census's
own doc comment says "would be a defect of the scan rather than of the file". It was.

The file's cross-reference table is 28 entries of exactly 20 bytes each, and a corrupter has put a
`/` or a 0x1f where a digit or an end-of-line belongs in seven of them. Entry 3 reads
`0000006251 000/0 n `. Tokenised, its generation field is `000` followed by the *name* `/0`, so
the entry does not read — and `read_classic_table` abandoned the rest of the subsection there,
keeping objects 0 to 2 and dropping the twenty-five entries after it, the page's own among them.

## What the reader then believed

`Document::load` asks `XrefTable::location` and returns `Object::Null` where the answer is `None`.
That is right for two conditions and wrong for a third:

- **a number no section mentions** names nothing;
- **a number an entry marks `f`** names nothing — §7.5.6 makes a deletion the most recent
  statement about an object, and ADR 0100 is the session that stopped this reader resurrecting
  objects its own file had deleted;
- **a number a subsection *header* declared and whose entry was never read** is neither. It has
  been mentioned and not described.

The third was reaching the same `None` as the first two, so a file that damaged one field of one
entry lost every object after it — silently, because a lost object is `Object::Null` and a null is
what §7.3.10 gives a reference to an object that is not there.

## The clause

A file states where an object is **twice**: §7.5.4's entry, and §7.3.10's header written beside
the object's own bytes. `Document::load_by_header` already prefers the second where the first is
disproved, and ADR 0100's rule keeps it away from a number the file deleted. What this decision
adds is a *third* statement the file makes, above the other two, in the subsection's header:

> The two integers denote (respectively) the object number of the first object in this subsection
> and the number of entries in the subsection.

§7.5.8.2's `/Index` says the same thing of a cross-reference stream — "[a]n array containing a
pair of integers for each subsection in this section". So the header names a range of object
numbers for which entries are here, and §7.5.4 gives exactly one way of saying a number in that
range names nothing:

> There are two kinds of cross-reference entries: one for objects that are in use and another for
> objects that have been deleted and therefore are free.

Both kinds are entries that were *read*. A number the header covered and no entry described
carries neither, so nothing has said it names nothing, and the object's own header is the file's
one surviving statement about it.

## What was built

`XrefTable::declared_and_unread(number)` — true where some section's header declared an entry for
that number and no section produced one. It is filled from three places, each the same fact:

- `read_classic_table`'s three abandonment paths record the remainder of the subsection
  (`first + index` for `count - index` numbers);
- the same function records a single number whose type field is a keyword that is neither `n` nor
  `f`. §7.5.4's third field is "the keyword n" or "the keyword f" and nothing else, so a run of
  regular characters that merely begins with one — `n\x1f`, which is what this file's entry 8
  writes when its two-byte end-of-line is none of the three the clause allows — says neither *in
  use* nor *free*;
- `read_xref_stream`'s short-data path records the records `/Index` states and the data does not
  carry, which is the population `XrefTable::entries_lost` has counted since it was written. Its
  own doc comment already made this argument — "the ones past the end of the data are absent
  rather than deleted … everywhere else in this reader a number with no entry has been deleted
  (§7.5.6, ADR 0100), and here it has not" — and nothing acted on it.

`Document::load` asks the two questions in order: `location` first, then `declared_and_unread`,
and only the second reaches `load_by_header`. A free entry and an unmentioned number are
untouched, because both are answered by the first question.

`misfiled_objects()` reports what was recovered either way, so the repair stays answerable rather
than silent; its doc comment now names both conditions.

## What it costs and what it cannot do

Nothing for a well-formed file: `unread` is empty, and the second question is asked only where the
first answered `None`, which for a document that opens normally is a number nobody asks for. The
recovery is `load_by_header`'s existing one — one linear scan of the file, memoised.

It cannot invent an object. Where the header scan finds nothing for the number, the answer is
still `Object::Null`, which is what four of the five other standing-count documents in this
corpus get and rightly (ADRs 0784, 0786, 0787).

## The witness after

`cairo-85141-3.pdf` draws page one at the producer's own 378 × 54 and reports five things, every
one of them the file's own remaining damage: two Type 3 fonts whose descriptions the same
corrupter hit, a string literal whose `(` it ate, the `BT` with no `ET` that follows, and a
`FlateDecode` content stream whose Adler-32 fails after delivering all 1114 of its bytes. The page
is blank because that is what its marks amount to once the fonts are gone — so the row in
`doc/checks/fixed-documents.toml` pins the reports and a 0-to-1 band, which is the page *existing*
rather than the ink.

## The general form

This is the fourth sentence in the family ADRs 0343, 0784 and 0787 belong to, and it is about the
other side of a recovery. Those three ask *what a prefix of the thing is* before drawing one. This
one asks what a **silence** is: an absent answer can be the file saying nothing or the reader
having read nothing, and a reader that cannot tell them apart turns its own failure into a
statement about the document.
