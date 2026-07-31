# ADR 0083 — A PDF that is a folder

Status: accepted, 2026-07-31.

## Context

§12.3.5's portable collections were four `silent` rows, and §7.11.4.2 and §7.11.6 two more that
only make sense beside them: a collection is a PDF whose point is the *other* files inside it,
with columns to show them in, an order to sort them by, a folder tree to hang them on, and —
since PDF 2.0 — a layout to present the whole thing with.

The embedded files themselves have been read since the eighty-sixth session (ADR 0076). What was
missing was everything the clause adds around them.

## Decision

`pdf-model/src/collection.rs` reads Tables 153 through 160, and `attachment::related` reads
§7.11.4.2's `/RF`.

Three parts of the family are *rules* rather than entries, and each is answered by a function
rather than stored:

**A file's folder is written into its name.** §12.3.5.2 gives the association no entry at all: an
`/EmbeddedFiles` key of the form `<3>report.pdf` puts that file in folder 3, and files whose keys
do not conform "shall be treated as associated with the root folder". `collection::folder_of` is
that convention — and it is the reason this module reads name-tree *keys*, which no other part of
this tree had needed.

**A layout is chosen, not read.** §12.3.6: a processor "should present the first one it is
capable of displaying in the order present in the array". `Navigator::preferred` therefore takes
the layouts the *caller* can draw, because the answer is a function of the viewer as much as of
the file.

**`/D` states three fallbacks, so `initial_document` returns three values.** The container itself
when the entry is missing "or is not a valid byte string"; "the first item from the list of
files" when it names one the tree does not hold; "an empty preview window" when there are no
files. An `Option` would have collapsed two instructions into one.

## The ledger row that was about the wrong document

§12.3.6's row said a navigator is "a collection's own presentation, supplied as SWF", that the
format "is what principle 5's exclusion of clause 13 is about", and that "[a]nyone widening that
list should start here, with an argument."

**ISO 32000-2's §12.3.6 contains no media format.** A navigator dictionary holds `/Layout`: one or
more of seven named layouts — `D`, `T`, `H`, `FilmStrip`, `FreeForm`, `Linear`, `Tree` — each
described in the clause's own prose, none of them a program to run. The SWF navigator was an
Adobe extension that this standard replaced, and the row was describing it rather than the
clause. So the exclusion argument nobody had made would have been made about the wrong document,
and the work turned out to be an afternoon's reading.

That is the **fifth** wrong ledger row, and the first whose error was *which standard it
described*. The others (§8.9.5.3, §8.4.3.2, and three `implemented` rows in ADRs 0056, 0057,
0060) were wrong about this tree. The defence is the same and it is worth restating: a row is
checked by reading the clause, not by reading the row.

## Consequences

- `silent` falls 77 → **71**, and **clause 7 reaches zero silences**: §7.11.4.2 and §7.11.6 were
  its last two, so every subclause of the syntax clause is now `implemented`, `partial`,
  `reported`, `inapplicable`, `out-of-scope` or `writer-side`.
- Clause 12's silences fall to 62 — forms, signatures and what is left of navigation.
- No corpus document is a portable collection: no `/Collection`, no `/Folders`, no `/RF`, and all
  23 embedded-file keys are plain names. So both worked examples in the clause — the email in-box
  and the collection item with a `Re:` prefix — are the tests, which is trap 8's shape again.
- Nothing here draws, and both `partial` rows owe the same thing: a file browser.
