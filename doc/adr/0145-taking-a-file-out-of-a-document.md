# ADR 0145 — Taking a file out of a document

Status: accepted, 2026-08-02. Session 169. §7.11.4 closes, and a checksum gets a place to be
checked.

## What was owed

§7.11.4's embedded file streams are, as the ledger row has said since they landed, "the one part
of §7.11 that needs no filesystem: the bytes are inside the document". This tree listed them
from the eighty-sixth session and, since the hundred-and-sixty-seventh, showed them in a panel.
A person could see that a document carried `foo.txt` and could do nothing whatever with it.

## The decision

`Command::Extract { name }` → `Event::Extracted { document, name, bytes }`, and the **host**
writes them. This is `Command::Save` → `Event::Saved` with a different subject, and it is rule 2
in the same direction: the core produces bytes, the host owns the filesystem. A confined renderer
with no filesystem at all can still hand a person their attachment.

Three smaller choices, each with its reason:

- **Named by the tree's key, not by an index.** §7.11.4.1 makes `/EmbeddedFiles` "map name
  strings to file specifications", so the key is the document's own identifier and
  `Query::Attachments` already answered with it. An index into a list would be a numbering the
  host and the core would both have to derive from the same walk.
- **Decoded here.** §7.4's filters are undone before the bytes cross, because a host that had to
  undo them would be a second reader of the document — and because two of the corpus's
  twenty-three attachments are in encrypted documents whose streams refuse, which §7.6.6 makes
  the stream's answer. That refusal is now *reported* rather than being an entry in a list nobody
  could act on.
- **The host writes only the last component of the name.** The file's name is a string *the
  document wrote*. `viewer-ui` takes `Path::file_name` of it and joins that to the directory the
  open document sits in — the mirror of §12.7.6.4's import policy — so a name that is a path, is
  empty, or is `..` is refused rather than followed. §7.11.4 states no policy at all, because a
  policy is a property of the processor.

## The checksum, and the reason that expired

Table 45's `/CheckSum` was read and not checked, and the row said why: "checking this would mean
inflating every attachment for a value the clause says is strictly a checksum". True of a
*list*, and false the moment anything decoded one stream.

The clause's subject decides where the check belongs:

> A 16-byte string that is the checksum of the bytes of the uncompressed embedded file.

"Uncompressed" — so the question cannot be asked until something has decoded the stream, and
extraction is the one path in this program that does. `Attachment::checksum_matches` is the
predicate and `Command::Extract` is the caller.

A mismatch is **said and not acted on**. The same table's NOTE is explicit: "This is strictly a
checksum, and is not used for security purposes." Withholding a file whose digest differs would
tell a person less than handing it over with a sentence beside it.

One edge is decided rather than defaulted: a `/CheckSum` that is not sixteen bytes answers
`Some(false)` and not `None`. The file stated a checksum and stated it wrongly, which is a
different thing from stating none — and `None` is reserved for the *optional* entry being absent,
which is most documents.

## A stale claim found on the way

§7.11.4's row said "`/RF`'s related files (§7.11.4.2) are unread". §7.11.4.2's own row says
`implemented` and `attachment::related` has read `/RF` since the hundred-and-fourteenth session.
The second was right. This is the class the handover names — a note claiming an entry is unread
where the tree reads it — and it was found by reading the family rather than the row, which is
the defence that keeps finding them.

§7.11.4.2's note carried a wrong *reason* too: "Nothing extracts them … writing a file is what
the sandbox exists to prevent." The sandbox prevents the renderer writing; the host writes, which
is exactly what now happens for `/EmbeddedFiles`. The true reason nothing extracts a related file
is that nothing asks — a related-files set belongs to whoever holds the specification, and no
corpus file specification carries an `/RF`.

## §7.11.4 and §7.11.4.1 close

Both move to `implemented`. Verified in the window as well as in a test: `attachment.pdf` opened
on `Xvfb`, `o`, the Files tab, a click on `foo.txt`, and nine bytes of `bar baz` land beside the
document.
