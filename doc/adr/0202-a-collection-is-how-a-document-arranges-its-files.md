# ADR 0202 — A collection is how a document arranges its files, so it is not a seventh tab

Status: accepted, 2026-08-06 (session 352).

## Context

§12.3.5 is a `shall` on a viewer:

> If this dictionary is present in a PDF document, the interactive PDF processor shall present the
> document as a portable collection.

`pdf_model::collection` has read Table 153 whole since the clause was implemented — the schema, the
folder tree, the sort, the navigator, §12.3.5.1's three fallbacks for `/D` — and **nothing called
any of it.** `doc/todo/01`'s fifth sweep produced eight of its `pub fn`s in the
three-hundred-and-thirteenth session, and the row explained itself with "which needs a file
browser", a reason that expired in the hundred-and-sixty-sixth when a sidebar arrived.

`doc/todo/36` had scoped it and named the one decision nobody had taken: **whether the container's
own pages stay on the screen.**

## Decision

### Not a seventh tab — the files tab *becomes* the collection

A collection is not a new population of things. It is the **same** embedded files, plus a statement
about how they are arranged. So the tab a person already looks in for §7.11.4's embedded files is
where the arrangement belongs, and `Content::collection` decides which of two shapes that tab
draws:

- `None` — the flat list of the `/EmbeddedFiles` tree, which is every document anyone has opened;
- `Some` — §12.3.5.2's folder tree as indented rows, each file under the folder its name-tree key
  names, with the schema's visible columns as the row's detail.

There is a second reason and it is arithmetic. Six tab labels share 300 logical pixels; a seventh
gets 43 each, and a label that does not fit says less than no label at all. ADR 0200 already paid
that cost once for §12.4.3.

### The container's pages stay on the screen

The clause says to present the document as a collection and does not say *instead of what*.
§7.6.7's unencrypted wrapper is what settles it: a wrapper's whole purpose is a page saying the
payload is encrypted, and Table 153's `/View H` — "initially hidden", with the processor providing
"means for the user to view the collection by some explicit action" — is how such a document asks
for the file list to start closed. **A viewer that replaced the page with a file browser would hide
the sentence the wrapper exists to show.**

So a collection is a panel over a page, like every other tab, and `/View H` is honoured by the
sidebar simply not being open — which is already true of every document that does not state
Table 29's `/PageMode /UseAttachments`.

### A key that names no folder is a file at the root

§12.3.5.2 files a document's embedded files by the *shape of the name-tree key*: `<3>report.pdf` is
*report.pdf* in folder 3. `collection::folder_of` reads it, and the clause states what a
non-conforming key means — such files "shall be treated as associated with the root folder". They
are drawn at depth zero, above the folders, which is where the root's own files belong.

The click still sends `Command::Extract` with **the tree's key**, folder number and all. That is
not a detail: `/EmbeddedFiles` is keyed by the whole string, so a host that stripped the folder
prefix to make the row look tidier would ask for a file the tree does not have.

### The schema is read for its columns and its *visibility*

Table 155's `/O` is "[t]he relative order of the field name in the user interface" and `/V` is
"[t]he initial visibility of the field". Both are obeyed: hidden columns are not drawn, and the
visible ones sort by `/O` with the ones that state none after them.

**Only the file-related kinds are answered.** Table 155's `/Subtype` divides the fields in two —
the first three keep their data in §7.11.6's `/CI` collection item and the rest read the file
specification — and `Attachment` carries the second group and not the first. That is a gap this
panel records rather than papers over: a text, date or number column drawn from the item needs
`/CI` on the file specification, which `pdf_model::attachment` does not read.

## Consequences

- **§12.3.5's `shall` is paid** and `doc/todo/36` closes. The row stays `partial` for what a panel
  of this width does not *offer*: Table 153's `/View T` tile mode, `/Sort`'s order, `/Colors`,
  `/Split` and §12.3.6's `/Navigator` layouts are presentation alternatives this host answers with
  one presentation — read, carried, and not offered.
- **Not one of the 974 corpus documents states a `/Collection`**, so the panel is defended by a
  hand-built folder tree in `viewer-ui/tests/panel.rs` and nothing else. Trap 8's converse for the
  third round running, and the same position §12.4.3's articles are in.
- **The `/CI` gap is now nameable**, which it was not while nothing drew a column: a schema whose
  fields are `S`, `D` or `N` will draw empty columns until `Attachment` carries §7.11.6's item.
  That is one entry on a file specification and is the natural next step if a witness ever arrives.
