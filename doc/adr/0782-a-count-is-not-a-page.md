# 0782 — A `/Count` is not a page

Session 858. Status: **accepted**.

## Context

`doc/todo/03` §31 handed this round a two-line finding and a design question, and asked for the
question to be settled *in writing* before any code moved.

The finding: `Pages::new`'s recovery scan — the one that finds a page by Table 31's `/Type /Page`
declaration when the tree cannot be walked — was guarded by `count == 0`, under a comment saying
"[i]t runs only where the tree produced nothing". Those are not the same test. `count` is
`/Count`'s claim; a root stating `/Count 5` over five `/Kids` that are not in the file produces no
page at all and no scan either, because the claim is not zero. So the comment described the right
rule for seven hundred and fifty sessions and the code ran a different one.

The question, which is why the round before this did not take it: **what does `len()` mean when
the tree contradicts its own `/Count`?**

## The reading

### 1. `/Count` is defined *by* the tree, and the `shall` that keeps them consistent is the writer's

§7.7.3.2, Table 30's `/Count` cell, in full:

> ( Required ) The number of leaf nodes (page objects) that are descendants of this node within
> the page tree. NOTE Since the number of pages descendent from a Pages dictionary can be
> accurately determined by examining the tree itself using the Kids arrays, the Count entry is
> redundant. A PDF writer shall ensure that the value of the Count key is consistent with the
> number of entries in the Kids array and its descendants which definitively determines the
> number of descendant pages.

Three things follow, and the third is the one that settles the question:

- the entry's value is a property of *the descendants*, not an independent fact stated beside
  them;
- the `shall` that keeps the two consistent is addressed to the **writer**, so a file whose
  `/Count` disagrees with its `/Kids` is a file that has broken a requirement, not a file that has
  told a reader something new;
- and the standard has already said which of the two wins where they disagree: the entry "is
  redundant", and it is the `Kids` arrays and their descendants "which definitively determines the
  number of descendant pages".

A node whose children are not in the file has no descendants within the page tree, and therefore —
by the cell's own first sentence — no pages, whatever integer sits in the entry.

### 2. Nothing in the standard states a recovery, and §7.7.3.1's `shall` does not reach this

§7.7.3.1 places one requirement on a processor: "Compliant PDF processors shall be prepared to
handle any form of tree structure built of such nodes." That is about the *forms* a valid tree may
take — flat, balanced, deep — and it does not reach a `/Kids` naming an object that is not in the
file, or a node that is its own child, because neither is a tree structure built of such nodes.
Annex C's error language is about a processor's own internal limits and not about a malformed
file.

So the recovery is a **choice**, and principle 5 requires it to be documented as one rather than
presented as derived. Which it already was, since the hundred-and-seventh session: reading Table
31's required `/Type` off every object is the file's own declaration, and the ascending
object-number order the recovered pages come back in is a documented choice, because §7.7.3.2's
tree is where page order lives and a file whose tree is gone has stated none.

### 3. Where recovery crosses into guessing

`doc/traps/parsers-and-streams.md` trap 5's additive-or-substitutive test decides it, and it draws
the line in a place the todo file did not anticipate:

- Reading `/Type /Page` off an object where the tree yielded **nothing** is *additive*: it adds a
  page the file declared and displaces nothing the file said.
- Running the same scan where the tree yielded **some** pages would be *substitutive*: a tree that
  produced one page of five has stated an order and a set, and replacing them with a scan's
  ascending object numbers puts an invented order in place of a stated one.

So the trigger is *no page at all*, never *fewer than `/Count` claims*. A document whose tree
yields one page of five keeps `len() == 5` and `get(1..4) == None`, and that residue is the
clause's own — the file said five and this reader can reach one.

### 4. The half that is a loudness rule, and that this round got wrong once

The obvious conclusion from §1 is that where the tree yields no page, `len()` is the recovery's
length — including nought where the recovery finds nothing. That was written, and
`viewer-core`'s `objects_lost_inside_a_damaged_object_stream_are_said_out_loud` failed on it.

The test's fixture puts the page dictionary inside a §7.5.7 object stream whose flate data is
damaged, so `pdf_syntax` refuses the object (ADR 0366). Under the rule above the document became
*pageless*, and a document with no page on the screen has nowhere to put a page report: the
sentence naming the lost objects had nothing to attach to.

The reading corrects itself, and the correcting sentence is in the cell already quoted. The NOTE's
determination is made "by examining the tree itself using the `Kids` arrays". A reader whose
`/Kids` names an object it *refused* has not examined anything — the number of descendants is
**unknown**, not nought. So:

- where the recovery **finds pages**, this reader has established a set of pages from the file's
  own declarations, `/Count` has been checked against the descendants that define it and
  contradicted, and `len()` follows the recovery;
- where the recovery **finds none**, `/Count` is the only statement anybody has made about the
  number, `len()` repeats it, and each page is asked for, refused, and said out loud.

The second is the louder of the two answers, which is the point. Substituting nought would convert
*this reader could not read the pages* into *the file says it has none* — trap 5's failure through
a plausible default rather than through a missing feature.

## Decision

`Pages::new` probes the tree for a page before believing a `/Count` it has not walked.

```rust
let recovering = count == 0
    || (believed.is_some() && !root.as_ref().is_some_and(|node| { … reaches_a_page(…) }));
```

`reaches_a_page` is `count_leaves`'s walk stopped at the first leaf it arrives at: `any` short
circuits, so a tree whose leftmost spine ends in a page costs that spine and nothing else. It
reads a node exactly as `count_leaves` and `find_leaf` do — a `/Kids` it cannot resolve is no
children, a `/Kids` that is not an array is a leaf unless the node's own `/Type` says `Pages`, the
same two bounds — and it deliberately does **not** build the page, because a probe that copied the
leaf's dictionary and its `/Resources` would pay ADR 0330's cost twice on every launch.

**The two conditions are ordered by cost rather than by meaning**, and `CLAUDE.md`'s "a 500-page
document must open no slower than a 5-page one" is what fixes the order: `count == 0` is an integer
comparison and short-circuits a pageless file straight to the scan, and the probe runs only where
`/Count` was believed *without a walk*, since a count that came from `count_leaves` has already
answered the question by walking.

`count` becomes `scanned.len()` where the scan found pages and stays the tree's number where it did
not — written as `scanned.is_empty()` rather than as `recovering`, which is §4's asymmetry in one
line.

## What it costs

Measured either side on `pdf-retrieve document` under callgrind, which is `Document::open` plus
`Pages::new` plus `len()` and is deterministic:

| document | before | after | |
|---|---|---|---|
| `tracemonkey_annotation_on_page_8.pdf`, 14 pages | 1 704 468 | 1 709 588 | **+5120** (+0.30%) |
| `ISO_32000-2_sponsored_EC3.pdf`, 1023 pages | 172 809 735 | 172 734 412 | **−75 323** (−0.044%) |

The probe does not scale with the page count — it is a leftmost-spine descent — and the large
document comes out *ahead*, because the object the probe resolves is page one's own and the
resolution is still warm when `get(0)` asks for it. Three `--trace` launches of ISO 32000-2 under
`Xvfb` and `lavapipe` either side: first present 137.2 / 159.7 / 132.3 ms before and 135.7 / 151.6
/ 137.6 after, which is one instrument's spread and not a difference.

## Why no page that drew before draws differently, and why that is a proof rather than a hope

`doc/todo/02` §7 asks a round that changes what is drawn to re-run `doc/todo/00`'s step 7. This
change can only make a page appear where there was none, and the reason is one line of `find_leaf`:
its `/Count` subtree skip requires `count > 0 && count <= *remaining`, and `remaining` is 0 for page
one — so **no skip can ever happen while looking for the first page**, and `find_leaf(remaining=0)`
descends exactly as `reaches_a_page` does. The two agree by construction, so wherever the probe says
*no page*, `get(0)` was already answering `None`. `Pages::get` switches to the recovered list only
when that list is non-empty, and `count` moves only then. Nothing that had a first page has a
different one.

The empirical half agrees and names its corpus: of the **974** documents in `doc/pdf.js`, **not one**
claims a positive page count and fails to produce page one, so no document in the population every
ratcheted gate walks is in the category this change reaches — which is also why the corpus, oracle,
text, census and quorra gates print what they printed before.

## What it recovers, and what it does not

`doc/todo/03` §31 named four witnesses. **One of the four is recovered** and the other three are a
different defect, which is a finding about the instrument that named them.

`batch1/PDFBOX/PDFBOX-4623-1.pdf` draws. Its root is object 2, stating `/Count 1 /Kids [2 0 R]` —
its own kid — so every descent revisits the node it came from; object 3 states `/Type /Page`,
`/Contents 5 0 R` and one `Tj` of *Hello World* at 48 pt, and takes its `/MediaBox` and its `/F1`
by §7.7.3.4 inheritance up its own `/Parent`. Pinned in `doc/checks/fixed-documents.toml` at ink
1.315.

**All three references fail on it, and that is a disagreement worth stating rather than a score.**
`poppler` prints *Syntax Error: Loop in Pages tree* and writes a 1 × 1 image; `mupdf` prints *format
error: cycle in page tree*, then *Page tree load failed. Falling back to slow lookup*, then the same
error again, and writes an empty file; `ghostscript` prints *Couldn't get page info* and *page not
found*. Principle 5 says a disagreement is a question to take back to the specification, and this
one has an answer: every mark on the page is derived from the file's own declarations — Table 31's
`/Type`, §7.7.3.4's inheritance up the page's own `/Parent`, the `/Contents` the page names — and
nothing here is tuned to any of the three. `mupdf`'s middle line is the most interesting of them:
it has a recovery of its own, tries it, and does not reach the page either. That is agreement about
the *tree* — which is a cycle, and all four readers say so — and not about the page.

The other three — `PDFBOX-4339-0.pdf`, `poppler-742-0.pdf`, `poppler-750-0.tgz-0.pdf` — are
unchanged, and the reason is that **the tree is not what is broken in them**: each one's page
object is damaged in its own dictionary, so no reader gets a dictionary at all. Object 3 of the
first begins `3 0 obj \xbc<< /Type /Page …`, with a stray byte where the dictionary should open;
object 8 of the second writes `/TrimBox [0.000000 0.000000 595.276000 841.890000X:XA\xf7…`, an
array that never closes and runs into the stream after it; object 14 of the third leaves its
`/ProcSets` array unterminated and writes `/Con\x91ents` and `e@dobj` besides. §31's table was
written from a byte search for `/Type /Page`, and a byte search cannot tell a page object the tree
cannot reach from a page object nothing can parse. **`doc/habits.md`'s rule — run the reader and
the grep, and the reader is the instrument under test — is what separates them**, and running it
turns a claim about four documents into a claim about one.

## The second track, in the same sitting

`--bin owed`'s reading list opens with §12.9.1, whose note said the data a measurement needs is
read and "the tool that would take two points from a person is not built". Reading the clause found
one of its `shall`s stated in this tree and executed by nobody:

> Any measurement that potentially involves multiple viewports, such as one specifying the
> distance between two points, shall use the information specified in the viewport of the first
> point.

`measurement.rs`'s module comment quoted that sentence and said it was "why `Viewports::at` takes
one point and callers with two are expected to ask about the first" — and this tree had no caller
with two points. It needs no tool at all: two points and a page are the whole of its input.

`Viewports::distance` is that sentence plus Table 267's `/D` cell, which states the *order* the
arithmetic goes in and is the half a hypotenuse would get wrong:

> The first element in the array shall specify the conversion to the largest distance unit from
> units represented by the first element in X . The scale factors from X , Y (if present) and CYX
> (if Y is present) shall be used to convert from default user space to the appropriate units
> before applying the distance function.

So each axis is converted by its own array's first factor, `/CYX` brings the y result into x's
units, the distance is taken *there*, and §12.9.2's `format` walks `/D` over it. Neither `/O` nor
the `/BBox` corner order takes part, and that is the arithmetic rather than an omission: a distance
is invariant under a translation and a rotation. A `/Y` with no `/CYX` answers with nothing,
because Table 267 refuses the calculation itself — "if not specified, these calculations may not be
performed (which would be the case in situations such as x representing time and y representing
temperature)" — and answering would be this reader inventing a metric for a plot of temperature
against time.

Trap 8 throughout: `doc/pdf.js` and `doc/corpora` hold one viewport between them and it is `GEO`,
so both fixtures are the clause's own numbers, and the ordering test is built so that no single
factor applied after a hypotenuse can produce its answer.

§12.9.1 stays `partial` for exactly one reason now, and it is named rather than gestured at:
nothing supplies the two points. No crate under `viewer-*` names `Viewports`, `measurement` or
`distance` — checked by grep — so the clause's "users of interactive PDF processors" have no
interaction here. That is a host, not a reading.

## Consequences

- One document of the fetched Tika chunk draws that did not, and it is a page a producer wrote.
- `Pages::len()` is now checkable against the tree rather than a repetition of an entry, on the
  population where the two can differ, and is unchanged everywhere else.
- A refusal that used to arrive with the first page can now arrive with the document, where the
  lost object is page one's own. Nothing was made eager — the object the probe wants is the one
  `get(0)` was about to want — but `viewer_core::notes::losses` says its sentence *once*, so a test
  reading one channel and discarding the other is asserting about the file's layout rather than
  about the program. `objects_lost_inside_a_damaged_object_stream_are_said_out_loud` reads both and
  pins the count at one, which is what `losses_said` exists for and what no single-channel reading
  could check.
- §12.9.1's two-point rule is carried out, and this tree can answer a measurement question it could
  previously only hold the data for.
