# ADR 0366 — §7.5's two storage structures, and the byte a scan was taking off every stream

Status: accepted, 2026-08-15. Session 531. Takes `doc/todo/03` §11 — the three roles the
damaged-stream census still left silent after sessions 521 and 524 — and finds, in the course of
reading the first of them, that most of the population those rounds measured was this reader's own
defect. Amends §7.3.8.1, §7.3.8.2, §7.5.7 and §7.5.8's ledger rows.

## What §11 left, and what the split says it is

§11 named three silent roles: `unclassified` (296 of the crawl's 2260 damaged streams), an object
stream (144) and a cross-reference stream (10). The first is not a role at all — it is what the
census could not classify — so the round begins by splitting it, which §11 said was "a dictionary
question the way `Role::of`'s other arms are". It is not quite: a page's `/Contents` was already
classified by *who names it*, and every remaining kind is the same shape. `who_names_what` walks
every object once and records the entry each stream is named under, because the standard makes the
naming entry the statement of the role — Table 31's `/Contents`, Table 110's `/CharProcs`,
§12.5.5's `/AP`, §8.6.6.3's fourth array element, §9.10.3's `/ToUnicode`, Table 119's `/Encoding`,
§12.3.4's `/Thumb`, §12.6.4.16's `/JS`, Annex K's `/XFA` — and two more arms read the stream's own
dictionary, §9.7.5.3's `/Type /CMap` and Table 77's `/ShadingType`.

*(This paragraph read "Table 122's `/Encoding`" and "Table 78's `/ShadingType`" until the
five-hundred-and-forty-fifth session. Both are the numbers the five-hundred-and-thirty-seventh
corrected in `damaged_stream_census.rs` — a Type 0 font's `/Encoding` is Table 119's and
`/ShadingType` is stated once, in Table 77, for every shading — and that round amended the code and
not the ADR the code came from.)*

**The split accounts for the whole 296 and the answer is that almost none of it was owed.** Of the
crawl's `unclassified`, **193 are a `/ToUnicode` CMap** — the one silence session 524 argued for
rather than assumed (§9.10.3 costs no mark, and its missing codes are already counted in
`Interpretation::codes_without_a_character`) — and **26 are a Type 3 glyph description**, which ADR
0359 made loud. Seventy-seven remained unclassified. The `form XObject` row splits too: 37 forms and
**9 annotation appearances**, both loud since ADR 0359. So the largest silent bucket in the role
table was, in its majority, the two things the last two rounds had already decided.

## The object stream, §7.5.7 — and the file that disproved the reader

The clause states each compressed object's extent as well as its start. The header is "N pairs of
integers … the second integer shall represent the byte offset in the decoded stream of that
object", "[t]he byte offsets shall be in increasing order", and NOTE 7 (2020):

> processing of each object in an object stream starts at the specified byte offset in the
> decompressed stream and ends prior to the byte offset of the next object or when the end of
> stream is encountered.

That answers ADR 0356's sharper question — *does the standard state the thing's extent?* — for
every object in the stream but the last, whose end is the end of the stream. So a prefix of an
object stream **is** a smaller collection of the same kind: the objects whose stated end the prefix
carries are whole, in the producer's own bytes, at offsets read from a header that sits in front of
them. The last object under a damaged decode is the one case where the end is unknown, and
`expand_object_stream` used to parse it anyway from bytes that stop early. That is a fabrication
rather than a shorter thing of the same kind, and it is the quiet sort: a truncated token still
parses, so `/Length 12345` cut short is `/Length 123` and the number names a value nobody wrote.
Such an object is now not read, and the numbers are recorded —
`Document::objects_lost_to_damage`, beside `misfiled_objects`, for the reason that accessor gives.

**The first corpus witness of that rule disproved the tree instead**, which is trap 1 arriving from
the other direction. `1284583.pdf` of the crawl holds `528 0 obj << /Type /ObjStm /N 3 /First 21
/Length 529 0 R /Filter /FlateDecode >>`, and this reader saw **83 raw bytes where object 529 says
84**, inflated all 159 bytes of the payload, found no final block, and called the stream damaged.
The new rule then refused its last object — a real destination dictionary the file carries in full.

## §7.3.8.2's `/Length` when it is indirect, which is where the population went

Table 5 makes `/Length` "(Required; shall be an indirect reference)" for a producer that does not
know the length until the data is written, so the indirect form is a route the standard *requires*
rather than an oddity. A **parser** cannot take it: resolving a reference needs the document that
parsing builds. `Parser::parse_stream_data` therefore falls back to searching for `endstream` and
dropping one preceding end-of-line — §7.3.8.1's "[t]here should be an end-of-line marker after the
data and before endstream" — and where a producer wrote none, the byte it drops is **the data's**.

The document above is exactly that, and it is not a curiosity: it writes CR alone as its
end-of-line throughout and an indirect `/Length` on every stream, so every stream in it was a byte
short. For a `FlateDecode` stream the lost byte is usually the last of RFC 1951's final block, so
the stream reads as *truncated* while being whole.

`Document::with_stated_length` applies the file's own statement one layer up, under the same guard
the parser puts on a direct length: the stated end is taken only where `endstream` is actually
there, so a *wrong* `/Length` still loses to the search. §7.3.8.2's "[a]ll of these constraints
shall be consistent" is what makes that check the right arbiter either way. This is trap 5's own
sentence — "[w]here a clause gives a parameter two routes, implementing one of them is the failure
mode that reports nothing" — with the clause being §7.3.8.2 and the two routes being direct and
indirect.

**What it moves is the population three rounds have been measuring.** One process per archive, 145
archives, 0 failures, before and after, with the *before* reproducing sessions 521's and 524's
figures to the digit:

| the crawl's 65 944 documents | before | after |
|---|---|---|
| damaged streams | 2260 | **1273** |
| documents holding one | 726 | **99** |
| page-one `/Contents` damaged | 90 (69 truncated, 21 corrupt) | **28 (7 truncated, 21 corrupt)** |
| of those, drawing at least one command | 85 | 26 |
| `/Contents` undecodable | 24 | 24 |
| damage reports, over documents | 1182 over 432 | **327 over 39** |
| images short of the grid §7.3.8.2 infers | 54 in 8 | 54 in 8 |
| short sampled functions | 0 | 0 |

**The corrupt column does not move and the truncated one collapses**, which is the mechanism's own
signature: a slice one byte short cannot corrupt a deflate stream, it can only stop it before its
final block. The pdf.js corpus moves the same way, 57 damaged streams over 20 documents to **48
over 11**, and `openpreserve`'s 267 from 299 over 32 to 294 over 28. Sessions 521 and 524 measured
what this reader saw; what the *files* contain was always the smaller number.

**And the object-stream rule then costs nothing on this disk.** After the fix, **no crawled
document loses an object to a damaged object stream at all**, and the only losses anywhere are 6
objects in `issue19484_1.pdf` and `issue19484_2.pdf` — whose header pairs the prefix never reached,
so their numbers were never known and nothing that used to be read is lost. The rule is therefore
pinned by a hand-built pair (trap 8), differing in RFC 1951's BFINAL bit and nothing else.

## The cross-reference stream, §7.5.8 — a shortfall that is not damage

Table 17 states this stream's extent as plainly as §7.10.2 states a sample array's: `/W`'s "sum of
the items shall be the total length of each entry; it can be used with the Index array to determine
the starting position of each subsection", with `/Index` defaulting to `[0 Size]`. So the arithmetic
— rows stated × row width against the decoded length — is the predicate, **not** the damage flag,
which is ADR 0356's better-predicate lesson one clause over. And a prefix of this table *is* a
smaller table of the same kind: every field's width comes from `/W` and every row's object number
from `/Index` and its own position, so nothing in a row depends on a row after it, and a partial row
was already refused by construction.

What the shortfall is worth saying is the asymmetry it creates. Everywhere else in this reader a
number with no entry has been **deleted** — §7.5.6's most recent copy, and ADR 0100 is the session
that stopped this reader resurrecting what a file had deleted — and here it has not: the file meant
to say something about that number and the bytes are gone. `XrefTable::entries_lost` counts them and
`viewer-core`'s notes say so.

**The condition has no members on this disk**, which is trap 11's honest outcome rather than a
failure: **0 entries lost over 65 944 crawled documents, over the 974 and over `openpreserve`'s
267**, and the crawl's ten damaged cross-reference streams lose not one *stated* row. `comments.pdf`
says why, checked by hand: its damaged one is `/W [1 4 2] /Index [582 1 585 1 693 2 697 6]` — 10
rows of 7 bytes, 70 bytes stated, **70 bytes decoded**, and the only thing missing is zlib's final
block. Damage is not shortfall. So this rule too is pinned by a hand-built pair.

## Where the two reports go, and why not the page

Both are facts about the **file** rather than about any page, which is where `viewer-core`'s
`notes.rs` already puts §7.5's rebuilt cross-reference table. The cross-reference shortfall is known
when the document opens and is said there. The object-stream loss is **not**: nothing expands an
object stream until an object inside it is asked for, and making that eager is what `CLAUDE.md`'s
startup rule forbids — so `notes::losses` says it when it becomes known, once, with the count it has
so far, and the viewer carries the "said" mark in `Open` rather than in the document.

**Deliberately not a page's report**, and the reason is `CLAUDE.md`'s: `pdf_model::interpret` is a
pure function of the file, the view state and the page, and a report that fired there would depend on
which pages had been read before it. The cost of that choice is that no gate sees these two
sentences; the benefit is that **no page leaves the oracle's judged set** for a fact that is not
about a page.

## What it cost, in the units trap 11 asks for

Nothing, in every instrument this tree has. `examples/display_list_digest` over all 974 corpus
documents is **byte-identical** across the whole change, so no page moved and `doc/todo/00` step 7
and the quorra lanes need no re-run, on ADRs 0343, 0356 and 0359's reasoning. The corpus gate's
incomplete count is 64 before and after; the oracle's verdicts are identical bucket for bucket —
906 agrees, 67 contradicted, 786 ambiguous, 1691 judged pages — measured by running both gates on
the reverted tree in the same sitting; the pdftotext gate is 99.2% (22836/23015 words) and the word
boxes 98.26% on both sides. PDFBox's frozen extraction, whose corpus was checked out for this round,
reads 99.8% (14257/14281) in both of its orders.

That a change moving 987 damaged streams out of the crawl's population moves nothing in the gate
corpus is worth stating rather than hiding, and the census says why rather than the digest: the nine
streams the fix un-damages in the 974 are **all images**, and the count of images short of the grid
§7.3.8.2 infers is **1 before and 1 after** — so every one of them already carried every sample its
own dictionary asks for, and the missing byte was inside the compressed stream's tail rather than
inside its output. What the fix removed there was a false *report*, not a wrong picture.
