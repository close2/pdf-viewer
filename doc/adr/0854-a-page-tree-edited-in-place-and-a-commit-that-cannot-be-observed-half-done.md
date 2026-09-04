# 0854 — A page tree edited in place, and a commit that cannot be observed half-done

Session 906. Status: **accepted**. The first of this round's two records: what RFC 0003 §5.2's
write verbs turned out to *be* once the exclusion in `CLAUDE.md` was read against them, and the
transaction a file-system face needs around them.

## Context

RFC 0003 §5.2's table says what each write verb means and how it is saved. Three of its five rows
say "incremental update" and the first says "transform layer: page insertion, saved as §7.5.6
append". Rounds 899 and 902 landed the core and the confined worker with all five declared and
refused, so this round's job looked like plumbing: build the transform call for each row, add the
POSIX transaction, done.

It was not, and the reason is one sentence in `CLAUDE.md`. Its amended authoring exclusion says
that what a user does to a document already open is not authoring, and that **it is written back
by §7.5.6's incremental update — never a rewrite of what was there**. RFC 0003 §3 repeats the
commitment in its own words: "[e]very write below is an append … so the *mechanism* needs no new
permission."

**`pdf-transform`'s `pages` and `merge` do not append.** They renumber every object, build one
page-tree node, reconcile every document-level construct and write a whole new file. That is
right for a command line, whose output is a path the caller named. Committing such a file *over*
the document a person has mounted is exactly the rewrite the exclusion forbids — the producer's
bytes would be gone, and with them anything a signature or an archive rests on.

So RFC 0003 §5.2's first two rows had no implementation in this tree at all. What existed was a
verb with the right *name* and the wrong *shape*.

## Decision

### 1. A fourth writing verb: the document edited in place

`crates/pdf-transform/src/update.rs`, `Plan::Update`, three edits: a page taken out of §7.7.3.2's
tree, another document's pages carried into it, and §14.3.3's entries set. Each is a set of
replacement objects and a new cross-reference section appended to the file it read, and the output
is the whole file — the source's bytes, byte for byte, and then the update.

**It is a different verb rather than a flag on `pages`**, and the boundary is the same one RFC 0002
§6.2 already draws between `pages` and `merge`: not the kind of edit but the shape of the output.
`split`, `merge`, `pages` and `optimize` write a file the caller named; `update` writes into the
file it read. A caller choosing between them is choosing what happens to the bytes that are
already on disk, which is a bigger decision than which edit is wanted.

What the page-tree edits rest on is Table 30, and both entries are read as the clause states them.
`/Kids` is "an array of indirect references to the immediate children of this node", so one entry
goes or several arrive at the position asked for; `/Count` is "[t]he number of leaf nodes (page
objects) that are descendants of this node within the page tree", so it falls or rises by that
many on the node and on every ancestor up the `/Parent` chain Table 31 requires each page to
state. A node emptied by a deletion leaves *its* own parent too, because a node with no children
is not one of the immediate children of anything.

**A deleted page is freed and its content is not.** §7.5.6: "[d]eleted objects shall be left
unchanged in the PDF file, but shall be marked as deleted by means of their cross-reference
entries." Freeing the page object is also what makes every reference to it *correct* rather than
dangling — §7.3.10 makes a reference to an undefined object "a reference to the null object", and
§12.3.2.2 makes a destination "an indirect reference to a page object", so an outline item or an
`/OpenAction` naming the deleted page resolves to null, which is the answer `pages --delete`
reaches by rewriting. Everything *below* the page is left in use, because nothing here can prove
another page does not share it. RFC 0003 §5.3 insists this be said where a person deletes, and the
verb says it in a warning.

**What an in-place insertion cannot carry, it refuses or names.** A rewrite reconciles the
document-level constructs because it is building a catalog; an update is not. So a page carrying a
§12.7 widget is refused by name — §12.7.4.2 makes a field's fully qualified name its identity, and
a widget whose field did not come with it is a form this program would have invented — and
`/StructParents` is stripped from each carried page with the loss counted, because §14.7.5.4's key
indexes *this* document's parent tree and a carried key would name another page's elements. An
`/AcroForm`, an `/OCProperties`, an `/Outlines`, a `/Names` or a `/StructTreeRoot` in the incoming
document is named in a warning. §12.4.2's labels *are* rebuilt, one entry per page of the new
list, because the clause's indices "shall be fixed, running consecutively through the document
starting from 0 for the first page" and no range of the old tree survives a move.

One thing the standard's own reading forced into the trailer: a document that states no `/Info`
and is being given one has nowhere to say so, because §7.5.5's Table 15 makes `/Info` the
trailer's entry. `pdf_syntax::write::incremental_update_extending` takes the entries an update's
own trailer states; `/Size`, `/Root`, `/Prev` and `/ID` stay the writer's, because they describe
the section under them.

### 2. The commit is a replacement, and §7.5.6 is what makes it checkable

Two questions, and they have opposite answers.

**Is the update *checked*?** Yes, and by the broker, which RFC 0003 §6 forbids to parse a PDF.
The confined worker computes the whole updated document and the broker compares its prefix with
the file on disk, byte for byte: "changes shall be appended to the end of the file, leaving its
original contents intact" is a property of two byte strings, and checking it is a comparison
rather than a parse. A worker that answered with anything else — compromised, or defective — does
not get its answer written.

**Is the update *appended*?** No: the file is written whole to a temporary beside it, synced, and
`rename(2)`d over the original. §7.5.6 makes the bare append look free, and it is not. The last
bytes of an update are `startxref`, an offset and `%%EOF`, which is what §7.5.5 makes a reader
enter the file by — so a `write(2)` cut short inside those thirty bytes leaves a file whose tail
names a cross-reference section that is not there. A rename cannot be cut short: POSIX requires it
to be atomic, so a reader sees the whole old file or the whole new one, and a crash before it
leaves the original untouched. The replacement also gives the file a new inode, which means a
reader holding it open keeps reading the old document — the same guarantee RFC 0003 §5.4 already
makes inside this crate, here enforced by the kernel instead of by a type.

What §7.5.6 buys is therefore not the writing but the **checking**, and that is worth saying the
right way round: because the new file is the old file plus a suffix, the broker can prove no byte
of the producer's was lost without reading either as a PDF.

### 3. The generation transition is designed rather than discovered

Our own commit changes the file, so it changes RFC 0003 §5.4's key, so the rule rebuilds the tree.
What must not happen is that the rebuild looks like somebody *else*'s edit, because a face reacts
to those differently — it invalidates the kernel's caches, tells the file manager, and may want to
say so out loud.

`Vfs` therefore holds the generation, the writes in flight and *the key its own last commit left
on the file* under one lock, and the whole commit happens inside it: read the generation, ask the
worker, check the prefix, replace the file, record the key, build the next generation. No
operation can fall between the two generations and see a tree belonging to neither. The generation
built for a recorded key is `Provenance::Ours`; any other new key is `Provenance::Foreign`; the
first one a mount ever serves is `Provenance::Opened`.

Every read blocks for the length of one commit. That is the cost of a transaction, and it is what
a caller of a file system expects `close(2)` to be doing.

### 4. A write staged against a generation that is gone is refused

The update a worker computes is a function of the document it was computed from. If somebody else
edits the file while a `cp` is in flight, committing would discard their edit. So the staged write
records the generation it was created under and `flush` refuses with `ESTALE` where it no longer
matches — the same rule §5.4 states for reads, applied to a write, and the one place this design
prefers losing the caller's work to losing somebody else's.

## Consequences

- RFC 0003 §5.2's first two rows are implementable without breaking `CLAUDE.md`, and the sentence
  they claimed ("saved as §7.5.6 append") is now true rather than aspirational.
- `pdf-transform` has a fourth writer and no fourth CLI verb. RFC 0002's verb set is the owner's
  to extend, and `doc/todo/57` carries the question of whether `update` should be one.
- A page's *content* is never destroyed by this face. Anyone deleting a page or an attachment to
  remove its bytes needs a rewrite, which is RFC 0003 §9's fifth open question and still the
  owner's.
- The `pages` verb and `update` now both know how to take a page out, by two constructions. That is
  not a second implementation of one thing: one builds a document and one edits one, and the
  clause-by-clause reasoning they share (labels, destinations, `/Count`) is the same reading
  applied twice. Whether the two can be made one engine is a real question and is not this round's.
