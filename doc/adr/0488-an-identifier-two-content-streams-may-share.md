# ADR 0488 — An identifier two content streams may share

Status: accepted, 2026-08-22. Session 661. Amends §14.7.5.2's and §14.7.5.4's ledger rows. Extends
ADR 0134's element-to-text mapping and ADR 0486's content rectangle with the half of their key that
was missing; changes nothing ADR 0301, 0325, 0338, 0394 or 0425 decided.

## The question

ADR 0486 named it and left it unmeasured, in its own words:

> A sequence in a form `XObject`'s own content stream shares one numbering with the page's. …
> `Interpretation::marked` is keyed by the identifier alone — so a document using both routes on
> one page could attribute the form's rectangle to the page's element. That is pre-existing: the
> text `range` beside it has had the same key since ADR 0134, and no corpus document has been
> checked for it.

Two features, one key, and the key is short by half.

## What the standard says the scope is, and it answers cleanly

**§14.7.5.2**, on the identifier itself, and it is a `shall`:

> The marked-content sequence shall contain a property list (see 14.6.2, "Property lists")
> containing an MCID entry, which shall be an integer marked-content identifier that uniquely
> identifies the marked-content sequence within its content stream

*Within its content stream.* The same clause then gives a form `XObject` two ways into the
structure, and the second is the one that matters here:

> The content stream of a form XObject may contain one or more marked-content sequences that are
> associated with structure elements (see Example 5 in this subclause). The form XObject may have
> arbitrary substructure, containing any number of marked-content sequences associated with logical
> structure elements. However, any Do operator that paints the form XObject shall not be part of a
> logical structure content item.

and its Example 5 writes the collision out in full: a page whose `/Contents` states `/P <</MCID 0>>
BDC` and a form `XObject` whose own stream states `/P <</MCID 0>> BDC`, with the structure element
naming the second through Table 357's

> Stm … ( Optional; shall be an indirect reference ) The content stream containing the
> marked-content sequence. This entry should be present only if the marked-content sequence resides
> in a content stream other than the content stream for the page (see 8.10, "Form XObjects" and
> 12.5.5, "Appearance streams"). If this entry is absent, the marked-content sequence shall be
> contained in the content stream of the page identified by Pg (either in the marked-content
> reference dictionary or in the parent structure element).

**§14.7.5.4** is the route back, and it is per stream by construction:

> The tree shall contain an entry for each object that is a content item of at least one structure
> element and for each content stream containing at least one marked-content sequence that is a
> content item.

with Table 359 saying where the key sits:

> Depending on the type of content item, this entry may appear in the page object of a page
> containing marked-content sequences, in the stream dictionary of a form or image XObject, or in
> an annotation dictionary.

**And Errata Collection 3's Issue #308 says the consequence outright**, as a new NOTE 2 under
§14.7.5.4 (`/State` `Review`/`Completed`, `spec-errata emit doc/ISO_32000-2_sponsored_EC3.pdf`,
p. 750): identifiers are scoped by content stream and must start at zero, so the same one may
reappear across pages or `XObject`s. Session 610 found that note; this round read it under its own
heading rather than a neighbouring one, and it is filed where the parent tree is described, which is
the right place for it — it is a sentence about the *key*.

**So the clause is not ambiguous and the defect is ours.** The identifier is scoped to a stream,
Table 357's `/Stm` names which stream, and §14.7.5.4 gives each stream its own parent tree entry.
This tree flattened two streams' numbering into one; nothing about the standard invited it.

## The measurement, over both populations

`pdf-model --example mcid_stream_census`, built this round. Per page it groups the sequences the
interpreter closed by the stream they came out of, and counts a page with two or more marking
streams (the *condition*) separately from a page where two of them use the same identifier (the
*collision*). Beside that it reads three things out of the file's own dictionaries without
interpreting anything — a `/StructTreeRoot`, a Table 357 `/MCR` stating `/Stm`, and a form
`XObject` with its own `/StructParents` — which is 648's probe: a census that only looked at what
the interpreter produced could report a clean zero over a population whose files declare the
construction in writing, and the two disagreeing is the tell.

**Pages are interpreted only for a document with a structure tree.** A page with no structure tree
has no element for a misattributed identifier to reach; the file-level counts are taken for every
document, so the skipped population is still named.

| | pdf.js + `doc/corpora` + `doc/` | SafeDocs `CC-MAIN-2021-31` |
|---|---|---|
| paths given / opened | 1289 / 1245 | 65 944 / 65 703 |
| with a `/StructTreeRoot`, so interpreted | 153 | 23 447 |
| documents with a page marked by two or more content streams | **1** | **701** |
| documents where two streams on one page share an identifier | **1** | **42** |
| pages with two marking streams / colliding | 1 / 1 | 6138 / 163 |
| identifiers colliding | 1 | 7334 |
| documents stating a Table 357 `/Stm` | 0 | 545 |
| documents with a form `XObject` stating its own `/StructParents` | 0 | 635 |

So it is real, and the two populations say different things about it — which is exactly the split
`CLAUDE.md` draws between the two questions. The small corpus's one witness is `issue15372.pdf`,
whose form's `/MCID 0` read back nothing and drew nothing, so the collision cost that file nothing
visible; the crawl's 42 include `2391456.pdf`, with 6168 colliding identifiers over three pages,
and 17 of the 42 also state `/Stm`, which means they are **conforming files this tree was reading
wrong outright**.

The instrument was calibrated against a planted collision before it was believed (trap 13): a
hand-built pair, one page whose form reuses `/MCID 0` and its twin numbering from 1, and the census
names the first and not the second.

## What was built

**`content::ContentStream`** — `Page`, `Object(ObjectId)`, `Unnameable` — recorded on every
`MarkedSpan` as its sequence closes. `Interpreter::draw_xobject` swaps it for the span of a form's
stream, from the *unresolved* resource entry, because Table 357 names a stream by reference and
resolving first throws that away. A Type 3 glyph description deliberately does **not** swap it:
§9.6.4 makes a `/CharProc` how one glyph of the enclosing stream's text is painted, so its marks
belong to whatever sequence encloses the show operation.

**`structure::Child::MarkedContent` gained `stream`**, from Table 357's `/Stm`, read unresolved for
the same reason `/Pg` is. `None` is not "unknown" — the clause makes it the statement that the
sequence is in the page's own `/Contents`.

**`content::named_sequences` is the one place the match is made**, and the three consumers go
through it: `Tree::logical_text`, `Tree::logical_range` and `viewer_core::accessibility`'s
`ranges` and `marked_extent` (ADR 0486's rectangle and ADR 0134's ranges, which is both of the
things this round was about).

**`Interpreter::enter_stream_structure`** gives §14.9's `/Alt`, `/E` and `/Lang` the *stream's* own
parent tree while that stream runs. This was a third consumer nobody had listed: the entries are
fetched from the element a `BDC`'s identifier names, and inside a form that lookup was indexing the
page's array with the form's identifier. Memoised by the object the stream is, so a form drawn a
hundred times is read once; a stream stating no `/StructParents` keeps the enclosing tree, which is
what §14.7.5.2's *first* arrangement means — a `Do` inside the page's own sequence makes the whole
form part of that sequence.

**`Tree::stream_owners`, and it is a fourth route `elements_on_page` did not have.** §14.7.5.2 gives
a form two ways into the structure and this tree asked only about the first: `object_owner` reads
`/StructParent`, the singular key of a form that is a content item *in its entirety*. A form whose
stream holds sequences carries `/StructParents` instead, and an element reached only that way was
pruned as belonging to another page — so a figure tagged inside a form reached no screen reader at
all. Table 359 forbids both entries at once, so asking for each costs one dictionary lookup.

## The one recovery, and what it costs

The strict reading has a price and the accessibility census charged it immediately: **61 elements
in two of the corpus's 153 tagged documents lost their place**, and the ratchet failed. Those two
files — `issue19971.pdf` and `bug1721218_reduced.pdf` — put every marked-content sequence inside one
form `XObject`, state no `/StructParents` on either the page or the form, and name every content
item with a bare integer. Read strictly they say nothing at all: §14.7.5.2 makes an absent `/Stm` a
`shall` that the sequence is in the page's own content stream, and it is not.

So `named_sequences` carries one recovery, with a condition drawn from the clause rather than from
the corpus: **where the page's own content stream holds no sequence with that identifier, and
exactly one other stream does, that one is answered.** There is then one sequence on the page the
file could mean, the `shall` it broke leaves no other candidate, and the moment two streams carry
the identifier the answer is empty again rather than both.

What it costs, written down rather than discovered later:

- A file that breaks the `shall` **and** repeats an identifier across two streams now gets nothing
  where it used to get something wrong. That is the right direction and it is still a loss.
- An attribution this crate inferred is not one the document stated, and **nothing tells a caller
  which it was**. There is no channel here for a readback shortfall, for
  `Interpretation::codes_without_a_character`'s reason (ADR 0152, trap 11): a report costs the
  oracle a judged page, and this is a shortfall in the readback rather than in the picture.
- The recovery cannot resurrect the defect it replaced: a collision requires the page's own stream
  to carry the identifier, and that is the case where the clause decides outright.

## An appearance stream is `Unnameable`, and that is a limit rather than a gap

`crate::annotation::Appearance` resolves `/AP` and keeps the stream rather than the reference to it,
and §12.7.4.3's constructed appearance is this program's own bytes with no object at all. So a
sequence closing inside an appearance is recorded against a stream nothing can name.

That is *sound* — Table 357 requires `/Stm` to be an indirect reference, so no marked-content
reference can name such a stream and no element can claim a sequence in it — and it fixes the half
that misleads: the page's own `/MCID 0` can no longer be answered with a widget's. What is not
fixed is the other direction, an `/MCR` whose `/Stm` names an appearance stream, which now finds
nothing where it used to find the wrong thing. **No corpus or crawl document exercises it**: over
the small corpus, zero sequences close in an unnameable stream. `doc/todo/31` carries the remainder.

## What the gates read

Nothing moved that draws. The oracle is 908 agrees, 65 contradicted, 786 ambiguous and
`render-quorra` 933 of 957 agreeing — both identical to ADR 0486's — so no pixel changed. Both text
gates are unmoved (99.2% over 974 documents against `pdftotext`, 99.8% over 40 against PDFBox's
frozen extraction, 98.26% of matched word boxes in bounds), the selection census reports zero
disagreements with the readback, and the accessibility census reads **102 853 elements, 93 267
placed by their own marks, 1336 with no place, and 876 of 876 untagged pages still answering the
honest empty tree with 0 invented** — every figure equal to ADR 0486's. That is the shape this
change should have: a key that was wrong on 42 documents in 65 703 and right by accident everywhere
else, corrected without moving what was already right.

## What this does not do

- **The negative claims here decay** (`doc/habits.md`). "Zero sequences in an unnameable stream"
  and "no crawl document whose recovery is ambiguous" are measured over 1245 and 65 703 documents
  on this day, and the crawl is one crawl.
- **A page is still asked for its elements page by page.** Nothing here changes ADR 0445's answer
  shape.
- **`/StmOwn` is unread.** Table 357's fourth entry names "[t]he indirect reference to the PDF
  object referencing the stream identified by the Stm key", whose common use is "to identify the
  annotation dictionary owning the appearance stream" — which is exactly what would let an
  appearance stream stop being `Unnameable` from the *structure* side. It is worth taking with the
  `/AP` reference, as one item, and not before.
