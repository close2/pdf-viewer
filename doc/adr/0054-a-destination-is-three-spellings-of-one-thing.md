# ADR 0054 — A destination is three spellings of one thing

Status: accepted, 2026-07-31.

## Context

ADR 0053 built §7.9.6's name trees for four rows that needed them, and closed one of the four —
§12.4.2's page labels. This is the second: **§12.3.2's destinations**, which `CLAUDE.md` names in
scope by name, and which every other item in clause 12's navigation half is downstream of. An
outline entry, a link annotation and a go-to action all point at a destination; none of them can
be built before the thing they point at can be read.

The clause is unusual in that it defines *one* object three ways, in three subclauses, and every
caller in the standard may be handed any of the three:

- **§12.3.2.2**, an explicit array — a page reference, one of Table 149's eight form names, and
  that form's parameters.
- **§12.3.2.3**, the same array with a *structure element* in the first entry.
- **§12.3.2.4**, a name or a byte string looked up in one of two tables.

## Decision

**One type, one reader.** `Destination::read` accepts an array, a dictionary with `/D`, a name or
a string, because a caller holding `/Dest` or `/D` or `/OpenAction` does not know in advance which
it has and the clause never suggests it should. A `Destination` is a `Target` and a `View`.

**The `View` is parsed and not applied, and that split is the honest part.** §12.3.2.1 says a
destination is three things — the page, "[t]he location of the document window on that page", and
"[t]he magnification (zoom) factor". The first is a property of the *document* and is computed
here. The other two are properties of a window with scrolling and zoom, which this program does
not have: it fits a page to its surface. So `View` carries Table 149's parameters exactly as the
file states them and nothing acts on them yet. Inventing a viewport to consume them would be
worse than saying so, and the ledger rows say so.

Two details of Table 149 that a first implementation loses, both now in the type:

- **A null parameter is not a zero.** "A null value for any of the parameters left, top, or zoom
  specifies that the current value of that parameter shall be retained unchanged" — an
  *instruction*, not a missing value, so every parameter is an `Option<f32>` and a stated zero
  coordinate stays a coordinate.
- **"A zoom value of 0 has the same meaning as a null value"**, so a zero read from the file
  arrives as `None` and cannot be mistaken for "no magnification".

**A page number is not a page index.** §12.3.2.2's NOTE gives the integer first entry to remote
and embedded go-to actions, whose page "is in a different PDF document". So `Target::Number` names
nothing in this file and `page_index` answers `None` for it, rather than treating the integer as
an index here. One corpus document writes one in a local `/OpenAction`; the clause's own fallback
for an unresolvable open action is Table 29's "the top of the first page", and that file's number
is 0, so the two readings agree on the page anyway.

**The page tree decides what the first entry is.** `page_index` asks `Pages::index_of` *before*
reading any `/Type`, because a reference the page tree holds is a page whatever the object says
about itself; only a reference the tree does not hold can be §12.3.2.3's structure element.

## The finding: a structure destination does not need the structure tree

The ledger's own row for §12.3.2.3 had said, for two sessions, that it "needs §14.7's logical
structure tree — unread, and the reason this row cannot be closed before clause 14's is."

That is wrong, and reading the clause is all it took. §12.3.2.3 states the entire algorithm in
terms of one element's `/K` and `/Pg`:

> The kids of the structure element shall be processed in linear array order. If the first kid is
> a marked-content reference or an object reference … then the page to which that reference
> belongs shall be used as the page. If the first kid is a structure element, then processing
> shall continue down to that element using the same algorithm recursively.

The page a reference "belongs to" is Table 357's and Table 358's `/Pg`, falling back to the
containing element's — which §14.7.5.3 states outright of an object reference: "[t]his entry
overrides any Pg entry in the structure element containing the object reference; it shall be used
if the structure element has no such entry." Nothing in it needs the tree *rooted*, needs
`/StructTreeRoot`, or needs the parent tree. **A row's own note is an entry to be measured, not
believed** — the same lesson the mesh subdivision entry taught in the forty-third session, this
time about a label this project wrote three weeks ago.

The clause's fallback is kept where the clause puts it: "[i]n the case where no page content is
identified, then the page reference shall be assumed to be the first page in the document" is a
sentence about *structure destinations*, and applying it to any unresolvable reference would send
every broken link to page one and call it correct.

## The consumer: `/OpenAction`

§12.3.2.1 names four places a destination is reached from, and three of them need a person to
click something. The fourth does not: "the optional OpenAction entry in a document's catalog
dictionary … may specify a destination that shall be displayed when the document is opened", so
the viewer now opens there. Table 29 states the other half — an absent entry means "the document
shall be opened to the top of the first page" — which is what an unresolvable one gets, with
nothing reported, because the clause has already said what to do.

This puts one page-tree walk on the startup path for the 55 corpus documents that state an open
action, and none on the other 919. It is the exception `CLAUDE.md`'s "no full page-tree walk"
rule cannot avoid: a viewer cannot show the page a document asks for without finding it.

## What the corpus says

Three numbers, and the second is the one worth keeping:

- **55 of 974 documents state an `/OpenAction`; 49 name a page this reader finds.** The other six
  are five action dictionaries that are not go-to actions — two ECMAScript, two `/Named`, one
  whose `/D` states no form Table 149 lists — and the integer-first-entry file above.
- **106 named destinations are reachable from link annotations; 22 resolve.** The other 84 name
  keys *their own documents do not define*: five files carry named links and no destination table
  at all, and `pdfjs_wikipedia.pdf` links to 27 `cite_note-…` anchors while its own table defines
  `cite_ref-…`. The test asserts the two-sided fact rather than the ratio — every key that **is**
  in a table resolves — so a regression shows up as a key we failed to find and not as a number
  moving.
- **§12.3.2.4's pairing holds without exception.** The clause gives PDF 1.1's name objects to the
  catalog's `/Dests` dictionary and PDF 1.2's strings to the name dictionary's tree; the corpus's
  22 are 2 names in a catalog dictionary and 20 strings in a name tree, and not one crosses. Both
  tables are still asked, in the order the clause introduces them, because "alternatively" is a
  sentence about where a document keeps its table rather than about which objects may address it
  — but the measurement says that fallback changes no answer in any file we have.

Annex J.3.3 and J.3.4, which §12.3.2.4 requires the lookup to use, cost nothing: both reduce to a
binary comparison of the *decoded* bytes — a literal string's escapes expanded, a hexadecimal
string converted, a name's `#` escapes resolved — which is exactly what the lexer has already
produced by the time an `Object` exists.

## Consequences

- §12.3.2.3 and §12.3.2.4 are `implemented`; §12.3.2, §12.3.2.1 and §12.3.2.2 are `partial`, and
  what is missing from each is a *window* rather than a clause.
- Three rows the ledger identified as blocked on a name tree are down to two — §12.7.7's named
  pages and §14.7.5.4's `/ParentTree`.
- §12.3.3's outline, §12.5.6.5's link annotations and §12.6.4.2's go-to actions now each need
  only the gesture that reaches them: the object they point at is read.
