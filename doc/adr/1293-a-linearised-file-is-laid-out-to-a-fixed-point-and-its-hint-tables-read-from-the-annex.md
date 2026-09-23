# 1293 — A linearised file is laid out to a fixed point, and its hint tables are read from the annex

Status: accepted and **built**.
Context: `crates/pdf-syntax/src/linearize.rs` (`serialize_linearized`, `state`),
`crates/pdf-transform/src/optimize.rs` (`OptimizePlan::linearize`, `linearization_plan`),
`crates/pdf-transform/src/bin/quorra-transform.rs` (`--linearize`),
`crates/pdf-transform/tests/linearize.rs`, `crates/pdf-transform/tests/support/linearized.rs`,
`crates/pdf-transform/tests/optimize_corpus.rs` (the linearised arm).
Answers: `doc/questions/A03` — linearisation ratified, 2026-09-22.

## 1. Where the writer sits

A second writer beside `serialize::serialize`, taking the same `Assembly`: the caller names the
pages and the first page (`linearize::Plan`), and the module decides every object's part, number
and offset. `optimize` is the only caller, behind `--linearize`, off by default — F.1 describes a
linearised file as "intended to be generated once and read many times", which is a choice about
how a file will be read and not a saving every caller wants.

## 2. The order, and the one permission taken

Parts 1, 2, 3, **5**, 4, 6, 7, 8, 9, 11. F.3.6 permits part 5 before part 4, and it is taken so
that no position a hint table states is *equal* to the hint stream's offset: F.4.1 adds the
stream's length to "a position greater than the hint stream offset", and says nothing of one that
equals it, while every stated position is in part 6 or later and part 4 always holds the catalog.
Part 10 is not written; every table fits in the primary stream.

**A page's objects** are F.3.7's "All objects that the page object refers to, to an arbitrary
depth, except page tree nodes, other page objects and DPart tree nodes", read literally, less three
that later clauses place themselves: a page's `/Thumb` (F.3.10's thumbnails), a part 4 object and
what only it reaches (F.3.5's threads, whose information dictionaries F.3.10 puts in part 9), and an
embedded file stream (F.3.10's own category, needed contiguous for F.4.7). The clause is unclear
here — its literal walk and F.3.10's categories claim the same objects — and qpdf was read for this
alone (`QPDF_linearization.cc`, `updateObjectMaps`): it too stops at other `/Page` objects and
`/Pages` nodes and treats `/Thumb` as its own category, and it follows an annotation's `/Parent`
into the field hierarchy, which this writer also does. Agreement, taken as evidence.

The lettered order (a)–(g) is a `should`, followed as far as a writer that reads no content stream
can; (d)'s one `shall` holds because every resource it covers precedes the first content stream.
Part 8 is ordered by a plain depth-first walk instead, for F.3.9's "all components of the structure
shall be grouped together". Part 9 claims category by category in F.3.10's order, each category a
contiguous run; a thumbnail object anything else reaches is left to that other category, because
thumbnail shared objects "shall not be referenced from any other objects".

**Inherited attributes** are copied down and then removed from the page tree nodes. F.3.10's
"pushed down and replicated in each of the leaf page objects" was read as *moved*: once every leaf
states what it inherited, the nodes' copies decide nothing. qpdf's checker refuses to check a file
whose nodes still state them, which is how the question arose; the reading is the annex's words, and
qpdf's agreement is evidence for it. An object only a node's attribute reached is not written, so
linearising the output again removes nothing more.

## 3. Correct by construction: a fixed point, not a patch

Parts 2 and 3 state offsets, and their lengths move every offset after them. Every object is
rendered once with its final number; then the layout is computed, parts 2, 3 and 5 are rendered
from it, and the loop repeats until none of the three changed length (at most 32 passes, and three
or four in practice, because lengths only grow). Nothing is written before the fixed point, so no
field is patched afterwards and none is padded to a guessed width — a padded field is a claim about
a number the writer did not yet know. The cost is holding the rendered file's structure in memory;
a carried stream's data stays the assembly's `Arc<[u8]>`.

## 4. The hint tables, and the choices the annex leaves

Written: the page offset (F.4.2) and shared object (F.4.3) tables always, and every table Table
F.2 makes "Required only if" the document has what it names — `/T`, `/O`, `/A`, `/E`, `/V`, `/I`,
`/C`, `/R`, `/B` — plus `/L`, which has no condition and is written where page labels are indirect.

- **F.4.1's packing is the annex's**: fields run "without regard to byte boundaries", and only a
  table begins on a byte. qpdf pads every item's run to a byte — its source comment says each row, not
  only each table, must begin on one — after Adobe's implementation notes, and so misreads this
  writer's tables; the fixtures' shared object table is where `qpdf --check` says so. Padding the
  runs makes qpdf report no error on the plain fixture — measured, then reverted. The clause is
  clear, so it stands; `doc/questions/Q131` asks the owner whether that convention is worth a
  departure.
- **Numerators** (Table F.4 item 5) at a denominator of 1: 0 for an object a page reaches through
  `/Resources` or `/Contents`, d + 1 otherwise. The finer fractions need the content stream read.
- **Content offsets** (items 6 and 7): the page's content streams where they lie in its own
  section; 0 and 0 where they do not, which the table does not provide for.
- **One width for every shared identifier**: Table F.10 item 7 cites Table F.3 item 11 as its
  width while item 6 states one of its own, so both are set to the width that covers the page
  offset table's and every extended table's identifiers, and both readings decode alike.
- **One object per shared group**: F.3.9's single entry for a structure is permitted, not required.
- **The shared object table's items run across both sequences**, "Item 1 for the first group, item
  1 for the second group", which is also qpdf's reading. Table F.6 item 1 cites Table F.3 item 4
  for the first page's location, which by its own words is item 2.
- **An empty required group** states the position it would occupy, and zero objects.
- **No MD5 signature** (Table F.6 item 3): which objects are the same resource is a content claim.

## 5. What is refused by name, and its price

- **Object streams in a linearised file** (F.3.1's conditions and F.4.1's byte-range positions):
  `--linearize` defaults `--object-streams` to `disable`, and asking for both is exit 4. About one
  round: the numbering F.3.1 states, object-stream positions in every table, and the carrier's
  `/Extends` chain kept within one part.
- **Encryption in a linearised file** (F.3.5's `/Encrypt`): exit 4. Under a round: the serializer's
  `Protected` handler asked per object under the final numbering, the dictionary in part 4.
- **F.3.7 (b)'s bead arrays**: a `/B` or a bead's `/T` the producer left out is not synthesised,
  and nothing says so. About a day: both are derivable from §12.4.3's thread chain.

## 6. qpdf, as evidence

`qpdf --check` over the fixtures: the file is read as valid PDF and as linearised; its linearisation
warnings are two disagreements, each taken back to the annex. The shared object table misread (the
packing above), and a warning that `/O` names the wrong page where `/OpenAction` names page 2 — qpdf takes
page 0 as the first page always, and F.3.7 says "that page shall be considered the first page", so
the annex stands. `tests/linearize.rs` holds qpdf to "no error" and prints its warnings.
