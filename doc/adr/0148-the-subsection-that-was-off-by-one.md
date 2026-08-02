# ADR 0148 — The subsection that was off by one

Status: accepted, 2026-08-02. Session 177. The first defect the ambiguous bucket's ranking named.

## How it was found

Session 176 gave the oracle's `ambiguous` verdict a ratchet and a ranking (§3a of the handover).
The ranking is by our distance from the **nearest** reference, in multiples of the bound the page
was held to, and its first answer was `issue7229.pdf page 1` at **77 bounds** — nearly the same as
its distance from the furthest, which is the shape that says every renderer disagrees with us
rather than with each other.

The side-by-side said it in one look. Three references and `hayro` draw a dense Russian
vehicle-inspection certificate; we draw the *reverse* of the form, with the stamp and the
signature. Not a rendering difference at all: **the wrong page**.

## What the file says

`issue7229.pdf` is a scan in two revisions. The original body holds one page — object 3, whose
`/XObject /Im1` is object 1, the front of the certificate — and the incremental update adds object
8 (the reverse, image 9) by replacing the `/Pages` node with `/Count 2 /Kids [3 0 R 8 0 R]`.

Its **original** cross-reference section is this:

```
xref
1 7
0000000000 65535 f
0000000009 00000 n
0000806807 00000 n
...
```

The subsection header declares seven entries beginning at object **1**, and then lists object 0's
free entry first. §7.5.4 is unambiguous about what that entry is:

> The first entry in the table (object number 0) shall always be free and shall have a generation
> number of 65,535; this entry shall be the head of the linked list of free objects.

and about the file it sits in:

> For a PDF file that has never been incrementally updated, the cross-reference section shall
> contain only one subsection, whose object numbering begins at 0.

So every entry in that section is filed one object number too high. Object 1 gets the free entry;
object 3 — the page — gets the offset of the content stream; object 4 — the page tree — gets the
page.

## What this reader did with it, for the project's whole life

`Document::load` already refused to hand back an object whose header names a different number, and
that refusal was right: returning object 2's bytes under number 3 corrupts the graph silently. But
a refusal is not a repair, and what fell out the other end was a document that *opened*:

- `Pages::len()` said **2**, because it reads the update's `/Count` and the update's `/Pages` node
  is filed correctly.
- `Pages::get(0)` walked `[3 0 R, 8 0 R]`, found that object 3 was not a `/Type /Page`, skipped it
  and returned object **8** — so page one was the page the references print second.
- `Pages::get(1)` had nothing left and answered `None`, so the second page did not exist.
- Nothing reported anything. `corpus.rs` called the document complete.

That is the failure mode this project's own handover warns about in trap 1 — the metrics cannot
see a page that is wrong rather than missing — and it survived because the *only* gate that could
see it files it under a verdict nobody was watching.

## The rule taken, and why it is a repair rather than a guess

A file states which object a cross-reference entry describes **twice**. §7.5.4 gives the number by
position — "the object number of the first object in this subsection" plus the entry's index — and
§7.3.10 makes the object at the offset name itself:

> The definition of an indirect object in a PDF file shall consist of its object number and
> generation number (separated by white-space), followed by the value of the object bracketed
> between the keywords obj and endobj

Where the two disagree by the *same amount for every entry checked*, the subsection header is
wrong: §7.5.4 requires a subsection's entries to be "a contiguous range of object numbers", and a
contiguous range displaced by one is still contiguous. The object headers win, because each one is
written next to the bytes it describes.

`xref::realigned` applies exactly that, per subsection, at table-read time. It is the same move
this tree already makes twice — `xref::rebuild` for a table that leads to no catalogue, and
`Pages::new`'s scan for a page tree that walks to nothing — a recovery from the file's own
declarations, never from another reader's behaviour.

## Three things it deliberately does not do

**It does not fire on one witness.** At least two in-use entries must agree on the same non-zero
displacement. One witness cannot tell a misdeclared subsection from a single stale offset, and
shifting a whole subsection on that evidence turns one broken object into all of them.

**It does not read every entry.** Four in-use entries per subsection are checked, by reading the
`N G obj` header alone rather than parsing the object — which matters, since the object at the
first offset of this very file is an 800 KB JPEG. A classic table has one subsection, an update
adds one per revision, and a cross-reference *stream* has none of this at all, so the cost is a
handful of header reads for a whole file. Checking all of them would be 101 318 header reads on the
specification's own PDF, which is the eager work `CLAUDE.md`'s startup rule forbids.

**It corrects free entries too, and that is the whole reason it is a subsection rule.** The first
version of this change repaired one object at a time: `Document::load`, on finding the header at an
offset naming somebody else, looked the number up in a scan of the file's own headers. That fixes
objects 2 and 3 of this file and leaves the misfiled *free* entry standing as object 1 — and object
1 is the page's image. The page then drew **nothing**, which is worse than the wrong page it drew
before. A displacement is a property of the subsection, so the subsection is what gets corrected.

## The per-object fallback stayed as well

`Document::load_by_header` survives, for the case the subsection rule refuses: a *single* entry
whose offset is stale. It runs only where an entry exists and is disproved, memoises one linear
scan of the file, and is reached by no well-formed document.

It must never run where the entry is **free**, and the caller's `?` on `XrefTable::location` is
what keeps that true. §7.5.6 makes a deletion the most recent statement about an object and ADR
0100 is the session that stopped this reader resurrecting objects its own file had deleted;
scanning for a header there would undo exactly that. `Document::misfiled_objects` names what was
repaired, because a reader that repairs a file in silence is one nobody can ask what it repaired.

## What it cost and what it bought

Every gate re-run: the corpus is unchanged at 974/0 unopenable/74 incomplete, the text gate is
unchanged at 98.2%, the dates gate is unchanged, and no oracle verdict moved except this
document's.

- `issue7229.pdf page 1`: **77 bounds from the nearest reference to under 10**, and it now draws
  the page the references draw.
- `issue7229.pdf page 2`: from `no render` to a rendered, judged page — the oracle's "no render"
  list, which is a list of pages nobody has ever looked at, goes 19 → 18.
- The ambiguous bucket goes 754 → **755**, which is a *rise written down*: a page that did not
  exist now does. Both pages are diagnosed in `AMBIGUOUS_IMAGE_REDUCTION`, where the six pairwise
  measurements say our reduction of the scan sits inside the references' own spread — on page 2
  every one of our three distances is smaller than every distance between two references.

Two hand-built tests pin the rule in `cross_references.rs`, each written as the pair trap 8 asks
for: one document assembled with `0 5` and with `1 5` over the same entries, and a second where a
lone entry points at another object and must be resolved by its own header rather than moving its
subsection. Both were checked by breaking the code they guard.
