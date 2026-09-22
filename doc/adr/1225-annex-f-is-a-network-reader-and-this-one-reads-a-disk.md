# 1225 — Annex F is a network reader's design, and the price of declining it is measured

Session 1194. Status: **accepted**. Decides §6.3.2.1's remaining `should` and moves that row from
`partial` to `departed`. Amends nothing it does not name; Annex F's own rows are untouched, and
`doc/todo/42` carries the plan this argues against building today.

## 1. The recommendation, and why a measurement was owed

§6.3.2.1 states, among the recommendations a PDF processor is given:

> Linearized files should be read as specified in Annex F, "Linearized PDF" and Annex G,
> "Linearized PDF access strategies".

The row carried that as unfollowed for many sessions on a sentence rather than a number — that this
tree "reads every file through the trailer and cross-reference table". Annex F's subject is
`CLAUDE.md` principle 2's own, F.1 saying the goal outright:

> When a document is opened, display the first page as quickly as possible.

A recommendation whose subject is this project's second principle is not one to leave declined on
prose, so the round measured it. The instrument is
`crates/pdf-syntax/examples/linearised_census.rs`, and the argument below rests on what it printed
rather than on what anybody expected.

## 2. What the corpus holds, by the standard's own two tests

F.3.3 fixes where the question is asked:

> The linearization parameter dictionary shall be entirely contained within the first 1024 bytes of
> the PDF file. This limits the amount of data a PDF processor will have to read before deciding
> whether the file is linearized.

Over the curated corpora — `doc/corpora`, `doc/pdf.js/test/pdfs` and `doc/`'s own specification
texts — **355 of 1507 documents state `/Linearized` inside that window**, which is not a corner
case and is the first thing the measurement settled.

The second test is Table F.1's, on `L`:

> It shall be exactly equal to the actual length of the PDF file. A mismatch indicates that the
> file is not linearized and shall be treated as ordinary PDF file, ignoring linearization
> information.

**Eighty of them fail it**, almost all by an incremental update appended afterwards — which F.3.6
says invalidates the hint tables and F.1 says makes the result one that "shall be treated as
ordinary PDF". So the population Annex F actually applies to is **263 documents**, and a route
built on the parameter dictionary has to carry that test or it will follow a stale one.

## 3. What this reader does before page one, against what Annex F would fetch

Of those 263, measured as `rchar` and `syscr` across `FileBytes::on_disk` plus `Document::open`,
then across the walk from `/Root` to the leftmost leaf of the page tree:

- **227 read more bytes before page one than `E`**, Table F.1's "offset of the end of the first
  page" — 19.2 MB across the population against 17.4 MB of first-page region.
- **256 of 263 resolve an object at or beyond `E`.** That is the page tree, and F.3.10 puts it
  there deliberately: "This object can be located in this section because the PDF processor never
  needs to consult it." Never, because Annex F's reader takes the first page's object number from
  the parameter dictionary's `O` and its page count from `N`.
- **326 of 343 read a second cross-reference section**, because `startxref` on a linearised file
  names the first-page table and this reader follows `/Prev` to the main one eagerly.

So the declined recommendation is genuinely declined. This is not a construction that reaches the
same outcome by another road, and the note that replaces the row's old sentence says so.

## 4. The price, cold, on the largest instances

Measured on the three largest genuinely-linearised documents in the corpus, each the quickest of
seven opens of a copy whose pages were dropped first (`doc/habits/measuring.md`'s method, the one
`launch_path.rs` uses), at a load average of 2.4:

| document | pages | bytes before page one | `E` | cold open | page one located |
|---|---|---|---|---|---|
| `PDF-HUL-117/me450f12project10_report.pdf`, 15.0 MB | 247 | 85 988 | 6 616 | 1.219 ms | 0.069 ms |
| `PDF-HUL-136/42828.0001.001.pdf`, 14.0 MB | 314 | 128 622 | 79 347 | 0.354 ms | 0.068 ms |
| `PDF-HUL-81/Gasp_and_Wheeze_Puppetry.pdf`, 9.9 MB | 70 | 86 689 | 46 856 | 0.165 ms | 0.238 ms |

**The whole of open-to-page-one on the largest of them is 1.29 ms.** What Annex F could remove from
it is the main cross-reference table — 77 KB of the 86 KB read — and the page-tree walk, 0.069 ms.
`doc/checks/launch-path.toml` puts time to first page at 30.3 .. 85.4 ms on every row it bands, of
which cold graphics bring-up is 23.3 .. 41.9 ms, by `CLAUDE.md` principle 2's own choice to put the
device on the critical path. So the recommendation's best case here is **under a millisecond inside
a figure whose floor is thirty**, and its largest single term is the driver's.

## 5. Why the hint tables in particular buy nothing here

This is the part that is about the standard rather than about a clock, and it is why the departure
is a decision rather than a deferral.

F.2 states the premise the whole annex is designed against:

> The performance bottleneck is assumed to be in the transport system (throughput or round-trip
> delay), not in the processing of data after it arrives.

and what the hint tables are *for*:

> this means that the PDF processor needs to have sufficient information to determine the byte
> ranges for all the objects needed to display a given page of the PDF file so that it can specify
> all those byte ranges in a single request.

A hint table computes byte ranges for a **request**. This program issues none: it opens a file and
reads the ranges it wants, one `pread` at a time, at a cost F.2's own reader would recognise as no
cost at all. F.4.1 then states what reading the tables costs a reader who does it anyway:

> Interpreting the hint tables requires reading them sequentially; they are not designed for random
> access.

> The PDF processor will be expected to read and decode the tables once and retain the information
> for as long as the document remains open.

That is work *added* to the launch path, in a stream that must be decoded before any of it can be
used, to answer a question a local reader answers with an offset it already has. And F.4.1's last
paragraph concedes the rest of it for a modern file:

> In linearized PDF files that use object streams (7.5.7, "Object streams"), the position specified
> in a hint table for a compressed object is to be interpreted as a byte range in which the object
> can be found, not as a precise offset. PDF processors need to locate the object via a
> cross-reference stream, as it would if the hint table were not present.

**F.1 states the consequence of declining, and it is the one this ADR accepts**:

> A PDF processor that does not support this optional feature can still successfully process
> linearized files although not as efficiently.

Not less correctly. The standard prices its own recommendation in efficiency, and section 4 above
is that price, measured on this machine, for this corpus.

## 6. What is decided, and what is not

**Decided**: §6.3.2.1's `should` is declined, and its row is `departed` rather than `partial` —
every `shall` in the clause is executed, the one recommendation is not, and this is its cost.
F.4's hint tables are not built, and a later round is not to build them on the ground that a `should`
is open: it is closed, here, with a number.

**Not decided, and deliberately left open in `doc/todo/42`**: the two cheap thirds of Annex F's
benefit do not need the hint tables at all, and both come out of Table F.1's parameter dictionary,
which is 1024 bytes this reader could read at any time.

- **`/N` and `/O` instead of the page tree.** Table F.1 gives "[t]he number of pages in the
  document" and "[t]he object number of the first page's page object" directly, and F.3.7 makes the
  first page's object self-sufficient — "This page object shall explicitly specify all required
  attributes, such as `Resources` and `MediaBox`; the attributes may not be inherited from ancestor
  page tree nodes." That removes the 256-of-263 objects resolved beyond `E`, and it is worth 0.069 ms
  on the largest document here.
- **A lazy `/Prev` chain.** F.3.4 makes the first-page table sufficient for page one — "The
  first-page trailer shall contain valid `Size` and `Root` entries, as well as any other entries
  needed to display the document" — so a reader that followed `/Prev` only on a miss would not read
  the main table at all. That is the 77 KB of the 86 KB.

Both are worth writing down and neither is worth building against a figure of 1.29 ms whose floor
is the graphics driver's thirty. The second is also not a local change: `XrefTable::len`,
`object_numbers` and the trailer merge are all statements about the whole chain, and the oracle's
determinism rests on `Document::open` being a function of the bytes. That argument belongs to the
round that takes it, with a figure that has moved.

**And the trap this round did not fall into.** Both of the above must be gated on Table F.1's `L`
test from section 2, because 80 of 343 files in this corpus state a parameter dictionary that
describes a file that no longer exists. A route that trusted `/O` on one of those would open the
wrong page and say nothing.
