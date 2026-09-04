# 0860 — The write side has a corpus walk, and it found a `/Count` nobody counted

Session 909. Status: **accepted**. The first of this round's two records: RFC 0003 §5.2's five
write verbs, driven through the core over every corpus document, and the three defects that came
out of it.

## Context

`doc/todo/58` §5 had this owed since the write side landed: "[e]very read generator is measured
against every corpus document by nothing, and so is every write." Round 906's own record put it
more sharply — the write side "has no corpus walk … which is the strongest candidate this stream
has for a gate."

Every other writer in this tree has one. `pdf-transform` carries six: `writer_corpus`,
`split_corpus`, `merge_corpus`, `pages_corpus`, `optimize_corpus` and `foreign_corpus`. Each of
them found a defect its predecessors could not see, and one of them found a file that was corrupt
while every raster it produced was bit-identical. The write side had `tests/a_write.rs`: five
verbs over four committed documents and one corpus document — thirteen assertions about five
files, against a population of 974.

The gap is not only in numbers. **The five verbs go through a writer no `pdf-transform` walk
touches at all**: `Plan::Update`, round 906's fourth writer, which edits §7.7.3.2's page tree in
place by §7.5.6's incremental update instead of rewriting the file. `pages` and `merge` were
walked over the corpus; `update` had never been.

## Decision

`crates/pdf-vfs/tests/write_corpus.rs`, in the six walks' own pattern, and through **the core**
rather than through the transform library — because what a face does to a document is
`Vfs::write` and `Vfs::remove`, not `pdf_transform::apply`. For every corpus document, on a fresh
in-memory backing per verb:

| verb | what it does | what it is held to |
|---|---|---|
| insert | a one-page document copied to `pages/0001.pdf` | the carried page draws as itself at position 1, and every page the document had draws as itself one ordinal further down |
| delete | `rm pages/0001.pdf` | every surviving page draws as itself one ordinal *up* |
| attach | a file copied into `attachments/` | it reads back byte for byte, beside every file the document already filed |
| detach | the same file removed | the listing is the document's own again, and §7.5.6's prefix property holds against the file as it was **before either commit** |
| set information | `meta/info.json` overwritten and written straight back | what was written is stated and what was omitted is not, and the second write changes nothing |

and on every one of them: §7.5.6's prefix property read off the *file* after the commit, the
document re-opening, the page count and the renumbered listing, §14.7.5.4's `/StructParents`
stripped from the carried page and untouched on every page that was already there, and RFC 0002
§9's first layer — the same insertion computed twice.

Two things about the walk's own shape are worth stating, because both were decisions:

- **It supplies §7.6.4.1's password through a `Workers` of its own.** `Vfs` passes `None` at the
  one place it spawns a worker, which is `doc/todo/58` §5's recorded shortfall, so eight corpus
  documents would have been in the refusal list rather than in the population. The walk implements
  the `SecretSource` that shortfall describes, at the seam where it would go — which is also a
  demonstration that the seam is in the right place.
- **The rasters do double duty.** An insertion at position 1 moves every page down by one and a
  deletion of page 1 moves every page up by one, so the comparison is *between different
  ordinals*. That makes one raster comparison a check of both the writer and RFC 0003 §5.2's
  "[o]rdinal names are **positions, not identities**".

## What it found

### 1. A node with no `/Count` counted as zero — ours, fixed

`pdf_transform::update`'s `count_of` was documented as "§7.7.3.2's `/Count` as the document states
it for a node, **or the leaves under it counted**" and implemented as
`.and_then(Object::as_integer).unwrap_or_default()`. A node that states no `/Count` therefore
counted as **zero**, and an insertion under it wrote `/Count 1` over a node that now held two
pages. `poppler-91414-0-53.pdf` and `-54.pdf` are the witnesses: the update committed, §7.5.6's
prefix held, and the two-page document read back as one page.

Table 30 makes the entry required, so a node without one is malformed — and what a malformed
node's descendants are is a question the *tree* answers rather than the missing entry. `count_of`
now walks, and `leaves_under` reads a node **exactly as `pdf_model::count_leaves` does**: a
`/Kids` that is not an array is a leaf unless the node's own `/Type` says `Pages`. That agreement
is the point rather than an economy — the number written into `/Count` is the number the reader on
the other side of the file will count if it disbelieves the entry, so a second reading of the same
tree would be a second answer.

**This is trap 28 at the smallest scale it comes in**: the comment above the fallback was a claim
about the code, and the code had never made it.

### 2. An edit to a tree no reader enters — ours, refused

Two documents were edited correctly into a page tree that the catalog does not reach.

- `issue9418.pdf`'s catalog states **no `/Pages` at all**. `pdf_model::Pages` recovers such a file
  by scanning §7.7.3.2's own `/Type /Page` declarations, and a scan has no *positions* in it, only
  object numbers — so the insertion, spliced perfectly into the `/Kids` of the page's `/Parent`,
  came back **after** page 1 rather than before it, because the new object has the higher number.
  The walk caught it on the rasters: page 1 drew as the document's own page and page 2 drew as the
  page that was supposed to go first.
- `issue21436.pdf`'s catalog `/Pages` names an object whose `/Type` is `Page`, while that page's
  `/Parent` names a node the catalog does not reach. The splice went into the orphan and the
  document read back unchanged.

§7.7.2's Table 29 settles it: the catalog's entry is "( Required; shall be an indirect reference )
The page tree node that shall be the root of the document's page tree". A page whose `/Parent`
chain does not pass through that object is a page the catalog does not reach, and an update that
edits its chain writes a correct `/Kids` into a tree nobody enters. `the_catalog_reaches` refuses
both, by name, on insert and on delete alike — trap 5, an input this verb cannot honestly serve.

**Repairing the catalog was considered and declined.** Writing a `/Pages` into `issue9418.pdf`'s
catalog would make the document better, and it would also be this program inventing structure the
producer did not state; where the catalog names a *different* tree, as in `issue21436.pdf`,
choosing between them is guesswork. A refusal that says which clause is unsatisfied leaves the
decision with whoever has the file.

### 3. §7.5.6's sentence was said where a page is deleted and not where a file is — ours, fixed

RFC 0003 §5.3 asks for it in both places: "**§7.5.6 deletion does not destroy bytes** … this RFC
only insists the refusal/behaviour be stated where the user deletes."
`pdf_transform::update::delete_page` warned; `pdf_transform::attachments::remove` did not. Found by
writing the face rather than by the walk — a mount's only channel for such a sentence is a log
line, and the test that asserts the line found nothing to assert on.

### 4. Sixteen documents that cannot be written twice the same way — the clause's, not ours

Sixteen insertions produced different bytes on the second run, and all sixteen documents are
encrypted with AES. This tree already knew why and had written it down in
`pdf_syntax::write::identify`: "§7.6.3.1 requires a fresh random initialisation vector in front of
every AES string and stream, so an encrypted document's update differs from one save to the next
by construction."

So RFC 0002 §9's first layer does not bind there, and the walk says so rather than holding a list
of exceptions. **What still binds is the length**: the same plaintext under the same crypt filter
is the same number of bytes whatever the vector is, so a difference in *length* is a difference
this clause does not explain and fails the run. The walk also counts encrypted documents whose two
insertions *agree*, which is what a crypt filter without an initialisation vector — RC4 — would
produce; the count is printed because it is the discriminator between "encrypted" and "AES".

### 5. Most of this corpus has one page, and the deletion verb is thin because of it

883 of the documents refused the deletion with `update`'s own sentence: "this document has one
page, and §7.7.3.2 makes /Kids \"an array of indirect references to the immediate children\" of a
node that has some". That is not a defect and not a surprise once seen — the pdf.js corpus is
mostly reduced single-page test cases — but it is a fact about the instrument that anybody reading
its numbers needs: **the delete verb is measured on the tenth of the corpus that has a second
page.** It is written here rather than in the gate's output because the gate prints the count and
a reader can divide.

## Consequences

- `doc/todo/02` §2 gains two lines: `cargo build --profile gates -p pdf-vfs --bins` — which that
  section's own map already said would be owed by "whoever adds" a `--profile gates --test` line
  for this crate (trap 10) — and the walk itself.
- The walk is a **gate from its first run**: no known-failure list, and `HELD` is empty.
- It costs about two and a half minutes over 974 documents, which is the same order as
  `pages_corpus`. It draws three pages a document, five ways.
- What it does *not* do is ask anybody else. `pdf-transform`'s `foreign_corpus` is where qpdf,
  poppler and mupdf read this suite's output, and `Plan::Update`'s output is not in that walk
  yet — `doc/todo/58` carries it.

## Trap 13, and how the walk was shown to fail

The first run failed, on real conditions, before any of the fixes above existed: three renumbering
failures, two rasters that drew differently, and sixteen nondeterministic insertions. That is the
strongest form of the calibration — the sweep found defects nobody had planted.

The load-bearing assertion is the raster one, and after the fixes it has no witness left, so it was
calibrated deliberately: `splice_into_tree`'s `before` was inverted, so that every insertion lands
one position late, and the walk was re-run. The figures are in the round's record.
