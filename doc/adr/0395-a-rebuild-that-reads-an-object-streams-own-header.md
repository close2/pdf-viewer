# ADR 0395 — A rebuild that reads an object stream's own header

Status: accepted, 2026-08-17. Session 560. Takes `doc/todo/17` — written by session 558 with a
witness (ADR 0393) and **deleted by this round, so its argument is now this one**. Amends §7.5.7's
ledger row; the neighbouring change is ADR 0379, which moved the *other* boundary of the same
recovery.

## The defect, and why it was invisible

`xref::rebuild` reconstructs a cross-reference table by scanning the file for `N G obj` headers.
Every object it finds is at the outermost level of the file, and **an object §7.5.7 packed into an
object stream has no header to find**. So a document whose cross-reference information cannot be
read lost every compressed object — which, in a file written since PDF 1.5, is nearly everything
that is not a stream.

Nothing said so. A compressed object's number simply had no entry, and §7.5.6 makes a number with
no entry a *deleted* object (ADR 0100), so the loss wore the vocabulary of an ordinary absence all
the way to the page. `UnknownFilter-Linearized.pdf` is the witness session 558 found: the PDF
Association's own README says it "should be fully processable", `/XXXDecode` on its first-page
cross-reference stream sends this reader to the scan, and the page drew its cat and reported
`no /Font resource named /TT0` because object 17 is one of three inside object stream 11.

## The reading

**§C.4 licenses the reconstruction and names what it scans:**

> When a PDF processor reads a PDF file with a damaged or missing cross-reference table, it may
> attempt to rebuild the table by scanning all the objects in the file.

**§7.5.7's first sentence is why a header scan does not do that:**

> An object stream is a stream object in which a sequence of indirect objects may be stored, as an
> alternative to their being stored at the outermost PDF file level.

An object inside an object stream *is* an object in the file; a scan that stops at the outermost
level has found some of them. So this is one clause implemented halfway rather than a design
question — and the standard states the missing step outright, which is unusual enough to be worth
saying:

> N pairs of integers separated by white-space, where the first integer in each pair shall
> represent the object number of a compressed object and the second integer shall represent the
> byte offset in the decoded stream of that object, relative to the first object stored in the
> object stream, the offset for which is the value of the stream's First entry.

Two further sentences of the same clause decide the two questions that follow, and both were read
rather than assumed.

**A scan can always reach the streams themselves**, because the clause forbids nesting them:

> The following objects shall not be stored in an object stream: Stream objects

so every object stream is written at the outermost level with a header of its own. The recovery
needs no search of its own; it needs the scan to say which of the objects it parsed were object
streams, which is one `/Type` lookup on an object already in hand.

**A number found in both places belongs to the ordinary object:**

> If either an object stream or a compressed object is deleted and the object number is freed,
> that object number shall be reused only for an ordinary (uncompressed) object other than an
> object stream.

That is the clause's answer to the collision case, and it is the reason the rule is "an entry the
scan already made wins" rather than "the later statement in the file wins", which is what
`scan_for_objects` does for two headers bearing one number. The two readings disagree only for a
number an object stream *later in the file* claims — and see the census below, which is what turns
this from a choice into a measurement.

**§C.4 is informative and carries no ledger row.** `conformance::ledger::NORMATIVE_ANNEXES` names
D, E, F, I, K, L, O and Q, and Annex C's own title line says `(informative)`. So the requirement
side of this work is §7.5.7's, and §C.4 is a permission quoted where the code reads it. This ADR
records that rather than inventing a row, because a row for an informative annex would break the
ledger's own definition of its population.

## What was built

**In `xref`, one fact the scan already knew.** `scan_for_objects` parses every object it finds;
where the parsed object is a stream whose `/Type` is `ObjStm` it records the number, keyed the same
way as the offset so that a number later redefined as something else stops being one.
`XrefTable::object_streams` hands them back **in file order**, because that is the only evidence a
rebuilt table has about which of two statements is more recent.

**In `Document`, the phase that needs a filter chain.** Reading those pairs means decoding the
stream, which means §7.4's filters and §7.6's decryption, and `xref` deliberately has neither — it
is building the table that resolving an indirect `/Filter` would need. So `open_with_password`
grows a second phase after `authenticate`: `recover_compressed_objects`, which is a no-op for every
document whose table the file supplied.

**It reads the header and not the objects**, which is the one design decision here worth the word.
`CLAUDE.md`'s startup rule forbids eager work, and a cross-reference entry is made of a *number*;
the object at it is what `expand_object_stream` reads when something asks. The difference is
measured on the widest such document on this disk — 10 MB, 316 object streams, 142 641 compressed
objects — through `pdf-model/examples/open_cost`:

| `Document::open` | five samples |
|---|---|
| before this change | 13.0–17.5 ms |
| header only, as shipped | 52.3–75.6 ms |
| expanding every stream's objects | 196.5 ms (one sample) |

**A budget rather than a count.** Expanding object streams during recovery is work an attacker
chooses, and the number of streams is the wrong axis: a kilobyte of headers can name thousands.
`RECOVERY_DECODE_BUDGET` is 64 MiB of *decoded* object stream, checked before each stream is
started, so the worst case is the budget plus one stream — which is the exposure any document with
one large `/Contents` already has. The floor under the figure is the census: the widest real
expansion among every rebuilt document on this disk is 12.6 MiB. A file that exceeds the budget is
not refused; it recovers what the budget reached and says how many streams it did not.

**Every refusal is counted and typed, and a host is told.** `CompressedRecovery` carries the
streams the scan found, the ones whose header was read, the objects entered, the unreadable ones, the ones
beyond the budget and the numbers declined to the top-level scan. `is_whole()` is the one-sentence
question, and `viewer-core`'s open-time note now says which case a person is looking at: a rebuild
that recovered part of a file must not read like one that recovered all of it. What a *readable*
stream then fails to yield is unchanged and unmoved — ADR 0366's damaged-prefix rule answers it
through `Document::objects_lost_to_damage`, reached in the same order it would be under a whole
table: the entry says where the object lives, the expansion says whether the bytes are there. Both
sentences are pinned end to end in `viewer-core`'s headless suite, on one document that exercises
each of them at once.

## The census, before and after

`pdf-model/examples/rebuild_census` reads each object stream's `N` pairs itself rather than through
the reader's expansion, so the population is measured with an instrument that is not the code under
test (trap 8). Over the 65 944 crawled documents, the 974, and the four corpora:

- **261 of the 65 944 reach a rebuild** (0.40%), 28 of the 974, and 108 of the 277 files in the
  corpora. Of the crawled ones, **30 carry object streams** — 549 streams holding 247 902 objects
  by their own `/N`, of which 223 684 are named by a header this reader can read.
- **Before: 223 661 of those numbers were located nowhere at all** and 23 resolved, every one of
  the 23 being a collision with a top-level object. **29 documents lost at least one object.**
- **After: 223 661 are located inside an object stream** and 214 710 resolve. The 8 951 that do not
  are in streams whose decode stops short, refused by ADR 0366's rule and counted by it.
  **8 documents still lose an object**, each to a filter this reader has no decoder for or to
  damage.
- **All 23 collisions come from a stream whose decode stops short**, so the clause's reuse rule is
  not deciding any file on this disk against a header this reader could read whole. That is the
  measurement the strict rule was chosen with, and a later round questioning it has the number.
- **No rebuilt document anywhere states an unresolvable `/Root` while a compressed object is a
  `/Type /Catalog`**, so `rebuild`'s catalogue search was deliberately left at the outermost level.

**The population's own verdicts, surveyed on both arms** (the 30 crawled documents, the two of the
974, the two `pdf-differences` witnesses and `govdocs1-error-pdfs/050734.pdf`; 35 documents through
`tools/safedocs survey`): **7 pageless → 4, 17 incomplete → 17, 0 unopenable both ways, and nothing
moved backwards.** `1899648.pdf` and `2145835.pdf` gain a first page and draw complete;
`2760849.pdf` gains one and reports; `UnknownFilter-Linearized.pdf` becomes complete and draws its
*Hello!*; `3744416.pdf` goes from 121 skipped show operations to 6 and `7188417.pdf` from 248 to
none, both still reporting resources their damaged streams do not carry.

## What it did not move, which is the other half

**The display-list digest over all 974 first pages is byte-identical**, both arms in one sitting
with the same worker on disk — so no pixels moved, and no quorra lane or ink sweep was owed. Every
gate figure is unchanged, with one exception that is a gain rather than a movement: `issue17147.pdf`
joins the tagged population in `structure.rs` and `logical_order.rs`, because its `/StructTreeRoot`
is one of the nine objects its object stream holds and its cross-reference stream cannot be decoded.
The document has not changed; what it says has become reachable.
