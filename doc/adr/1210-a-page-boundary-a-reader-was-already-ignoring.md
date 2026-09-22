# 1210 — A page boundary a reader was already ignoring

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/boundaries.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/adr/1200` section 1, which read the catalogue's *none* for
`implementation-limits/page-boundary-sizes` against the clauses, found it too wide, stated the
predicate and deliberately did not build it — "a wrong geometry rule changes what a page shows".
Builds on: `doc/adr/1176` (where the standard has decided what the bytes mean, the rewrite
transcribes the decision), `doc/adr/0947` (nothing changes that no failed requirement asked to
change), `doc/adr/0816` (the fence).
Clauses: ISO 32000-2 §7.7.3.3 (Table 31), §7.7.3.4, §14.11.2.1, §7.9.5; ISO 19005-2 section
6.1.13.

## 1. ADR 1200's reading held, and the build tested each of its claims

Section 6.1.13's last requirement holds any of §14.11.2's five page boundaries to between 3 and
14 400 units in either direction. Table 31 makes `/MediaBox` required and the other four optional,
and §14.11.2.1 gives each optional one a default that is another box in the same file: the crop
box's "default value is the page's media box", and the bleed, trim and art boxes' "default value is
the page's crop box". So removing an out-of-range optional entry is the page saying itself the way
Table 31 admits rather than an edit to what it says.

Whether the removal is *lossless* is the second question, and the clause answers it:

> If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the media
> box, a processor shall treat the box as its intersection with the media box.

An over-sized entry is therefore already its intersection with the media box to every conforming
processor, and where that intersection is what the default gives, the entry carried nothing a
reader used. Every one of ADR 1200's factual claims about the corpus checked out:
`6-1-13-t09-fail-f` states a crop box of 14 402 over a media box of 4 and converts mechanically;
`t09-fail-a`, `-c` and `-e` fail at the media box itself and keep the original *none*.

## 2. The predicate is asked by asking this tree's own reader twice

`pdf_model::Page` is this project's one reading of §14.11.2.1 — its five boxes are already
defaulted and intersected with the media box. So the way to ask what a reader computes *after* a
removal is to ask that reader again, with the entry gone, rather than to write a second reading of
the clause inside the converter. `flattened` writes Table 31's two inheritable boundaries onto the
page's own dictionary as the effective values it already resolved to, and `Pages::detached` reads
the result; the before-and-after rectangles are what `RemovedBoundary::costs_nothing` compares.
That is `CLAUDE.md` principle 5's rule about one reader per clause family, kept where it would have
been easiest to lose.

Three answers come out of it, and the decision table cannot hold them because two of them are facts
about the *file*:

- **mechanical**, where every removal leaves every effective boundary where it was;
- **an authorised loss** (`Loss::PageBoundary`, `--authorise page-boundary`), where one does not —
  a box under 3 units, or an over-sized one a narrower crop box stands behind;
- **refused by name**, where the failing box is the media box. Table 31 requires it and
  §14.11.2.1 gives it no default, so there is nothing to fall back to and the routes are rescaling
  the page or tiling it, which ADR 0816's fence closes.

`decision::CONDITIONAL` is where that shows in the enumeration, under ADR 1209's class: the row is
`Answer::Mechanical` and `answer_of` routes on what the preparation found, so `--remedy-sites`
lists the site and says its answer is decided per file.

## 3. Two refusals the build added that ADR 1200's predicate did not name

Both came out of the corpus rather than out of the reading, and both are §7.7.3.4's.

**An entry an ancestor states governs every page beneath it.** Table 31 marks `/MediaBox` and
`/CropBox` inheritable, and the corpus's failing crop boxes are written on the `/Pages` node rather
than on the page — `t09-fail-f` and `t09-fail-d` both are. So the removal is made where the entry
*is*, and the cost is then asked of every page that resolves through it. A removal that moves what
a reader computes for a page whose own boundaries all met the limit is that page paying for a
sibling's failure, which is ADR 0947's first rule the wrong way round, and it is refused by name.
A page that *did* fail takes the consequences of its own removal, including §14.11.2.1's other
three boxes following the crop box they defaulted to — those are the authorised loss.

**A finding this reader cannot place is a refusal rather than a guess.** A finding naming no page,
naming no entry, or naming a boundary that is in neither the page's dictionary nor its ancestry
stops the removal with its own sentence. The population stays the validator's: which page fails at
which entry is `pdf_archive`'s reading of section 6.1.13, and this file walks the page tree only to
answer where an inherited entry lives.

## 4. The report carries both rectangles, because the removal leaves nothing to notice

`Conversion::removed_boundaries` names the page, the entry, whether *that* entry was the one
removed, and the rectangle a conforming reader computed for the boundary before and after. The
`removed` flag is not decoration: removing an out-of-range crop box can move what a reader computes
for a trim box whose own entry is absent and untouched, and a report that called that "an entry
removed" would be describing a change the file does not contain. `Conversion` loses its `Eq`
derive for it — the rectangles are §7.9.5 real numbers, and a type holding one has no reflexive
equality to claim.

## 5. What the sweep says

`archive_corpus` under the lock, before and after: **PDF/A-2b 471 converted and 139 refused becomes
474 and 136**, and `implementation-limits/page-boundary-sizes` — six refusals, sixth on the list —
is off the top-ten refusals entirely. PDF/A-4 is unchanged at 207 and 152, which is the standard's
doing rather than this build's: part 4 states no implementation-limits subclause at all.
`tools/state.sh remedies` moves 2b from 58 of 77 sites not built to 57 of 78, the extra site being
ADR 1209's.
