# 0781 — The trailer a rebuild could not find

Session 857. Status: **accepted**.

## Context

`doc/todo/03` §29 left the eight-hundred-and-fifty-seventh session six unread documents: of the
files in the fetched Tika issue-tracker chunk that this tree cannot open or finds no page in,
some get a page count out of `pdfinfo`, and the claim every chunk since the four-hundred-and-
twenty-fifth had been able to make — *nothing failed to open for a reason that is this tree's* —
could no longer be repeated.

Reading them found the claim was measured with the wrong instrument, and one document class where
it is true anyway.

**`pdfinfo`'s `Pages:` is the page tree's `/Count`, not a page.** For eleven of the thirteen
documents that produce one, this tree prints the *same* number — `Pages::len` reads `/Count` too —
and neither reader has a first page, because the `/Kids` those trees name are objects the bug
report's reduction removed. Asked to draw page one at 36 dpi, poppler answers a 1×1 or a blank
US-Letter sheet for eleven of the thirteen. **Two of them carry ink**, and those two are this
tree's defect.

## The two documents

`PDFBOX-4777-0.pdf` and `PDFBOX-4777-1.pdf`, filed against Apache PDFBox. Both are PDF 1.6 files
written entirely with cross-reference streams, both are encrypted, and in both the `startxref`
address is **nineteen bytes short** of the cross-reference stream's object header — it lands inside
the `/Encrypt` dictionary that precedes it.

So `xref::read` cannot follow the address, and falls through to §C.4's rebuild:

> When a PDF processor reads a PDF file with a damaged or missing cross-reference table, it may
> attempt to rebuild the table by scanning all the objects in the file.

The rebuild scanned the body for object headers, then looked for a trailer by searching for the
`trailer` keyword, then — finding none — inserted a `/Root` naming the first object that declares
itself a catalogue. **That is the whole of the trailer it recovered.** `/Encrypt` was gone, so
`Document::authenticate` had nothing to authenticate, `is_encrypted()` answered `false`, and every
string and stream in the file came back as ciphertext: the catalogue's `/Lang` as thirty-two
random bytes, every object stream as *"unknown compression method"*, the page tree unreachable.
The document opened, reported nothing, and drew nothing. That is trap 5's failure in its purest
form — not a refusal, a plausible-looking answer.

## Decision

**A rebuild takes its trailer from the cross-reference stream, because in such a file there is no
other trailer to take it from.**

§7.5.8.1 says why the keyword search cannot finish the job:

> For PDF files that use cross-reference streams entirely (that is, PDF files that are not
> hybrid-reference files; see 7.5.8.4, "Compatibility with applications that do not support
> compressed reference streams"), the keywords xref and trailer shall no longer be used.

and §7.5.8.2 says where the trailer went:

> Cross-reference streams shall contain the required entries and may contain the optional entries
> shown in "Table 17 -Additional entries specific to a cross-reference stream dictionary" in
> addition to the entries common to all streams ("Table 5 -Entries common to all stream
> dictionaries") and trailer dictionaries ("Table 15 -Entries in the file trailer dictionary").

`xref::find_xref_stream_trailer_by_scan` is the second source, between the keyword search and the
catalogue scan that was the last resort. It takes the cross-reference stream **furthest into the
file**, which is `find_trailer_by_scan`'s own rule for its own population and §7.5.6's reason: an
incremental update is appended, so the last one is the newest.

**This is recovery and not guessing, and the difference is that the file said it.** The dictionary
being read is the one §7.5.8.2 defines as carrying Table 15's entries, the object identifies itself
with `/Type /XRef`, and what comes out of it is the producer's own bytes. Nothing is invented; what
changes is that the reader stops throwing away a trailer that was in front of it.

## What it costs

The name `/XRef` is searched for in the file's bytes and each hit is attributed to the object the
scanned table puts it inside, so a file that has one pays a byte scan and one or two object parses
rather than a parse of every object it holds. A hit inside a stream's data attributes to that
stream's own object, which then fails the `/Type` test — the byte search proposes and the parse
disposes. The whole path is behind `table.trailer.get("Root").is_none()` after a scan, so no file
that opens normally reaches it at all.

## What it buys, measured

Both documents now open, authenticate under §7.6.4.1's default user password, and draw:

| | pages, before | pages, after | `pdfinfo` | our ink at 36 dpi | poppler's |
|---|---|---|---|---|---|
| `PDFBOX-4777-0.pdf` | 0 | 25 | 25 | 2474 px | 2571 px |
| `PDFBOX-4777-1.pdf` | 0 | 1 | 1 | 22 384 px | 20 317 px |

Both report **nothing** — `Interpretation::unsupported` is empty on each — and both are rows in
`doc/checks/fixed-documents.toml`. Nothing else moves: over the 4525 documents of the fetched
chunk, exactly these two change and no document that opened stops opening.

## The test, and why it is a pair

`cross_references.rs::a_rebuild_takes_its_trailer_from_the_cross_reference_stream`, in the file
whose whole premise is trap 8's — a rule required of any valid PDF and reachable by no document
anybody has. It is two pairs, each differing **only in the `startxref` address**, because the rule
is that the two answers agree: an entry the trailer states cannot depend on the offset being right.

`/Root` alone does not discriminate — the catalogue scan finds it without reading a trailer at all
— so the first pair asks for `/Info`, which nothing but the trailer names, and the second asks the
question that decides whether the file is readable: a document whose cross-reference stream states
an `/Encrypt` this reader does not implement must refuse, whichever address it carries. Run against
the tree before the change, the first pair fails on `/Info`.

## What this does not fix, and it is named rather than left

`Pages::new` runs `scan_for_pages` — the recovery that finds a page by its own `/Type /Page`
declaration — only where `/Count` is **zero**, while its own comment says it "runs only where the
tree produced nothing". Those are not the same test: a `/Count` of 5 over five `/Kids` that do not
exist produces no page and no scan. Four documents of this chunk carry a self-declaring page object
the tree cannot reach — `PDFBOX-4339-0.pdf`, `PDFBOX-4623-1.pdf` (whose page tree node is its own
kid, which poppler also refuses), `poppler-742-0.pdf` and `poppler-750-0.tgz-0.pdf`. The fix has a
design question in it that this round did not settle — what `len()` should say when the tree
contradicts its own `/Count` — and taking it without settling that would be the shortcut principle
1 forbids. `doc/todo/03` §31 carries it with the witnesses and the line.
