# 0803 — Every bit named, the four levels in one place, and a file on a page or taken out

Session 872. Status: **accepted**. The fourth decision record of RFC 0002's implementation, on
the long-lived branch `round-867`.

## Context

`doc/todo/57` §3 held one debt over from ADR 0802: `pdf_transform::Operation` — `Print`,
`Extract`, `Modify` — lived beside `pdf_model::restriction::Operation` — `FillInForm`,
`Annotate` — so two enums in two crates each read a share of Table 22, and only the viewer's
share was read against §12.8.2.2's certification. `CLAUDE.md` principle 3 states the shape the
policy must have — four levels, asked once, where a host can supply the answer, never decided
inside `pdf-model` — and `doc/todo/38` recorded that two of the four were absent by design
rather than stubbed. Moving the enum is a first-row change, and this round runs the whole gate
sequence, so this is where it goes. The other two items are the two halves of `attachments`
that RFC §6.6 named and 870 did not take: a file filed on a *page* rather than in the tree, and a
file taken *out*.

Three questions, each answered from the standard before any code:

1. Which of Table 22's bits does this program consume, which does it not, and what does a bit
   nothing consumes say about itself? Where do the four levels live, and what does a caller that
   cannot ask do with *ask*?
2. What does a §12.5.6.15 annotation consist of when a writer files a file by it, and where does
   it go when nobody says?
3. What does §7.5.6 owe a deleted object, and which of §7.5.4's two ways of marking an entry free
   can an incremental update use?

## Decision

### 1. `pdf_model::restriction`: every bit named, five operations, four levels, one verdict

**Every position of Table 22 is named**, in `Bit`, whether or not anything consumes it. The
clause's own sentence is the list — "PDF readers shall ignore all flags other than those at bit
positions 3, 4, 5, 6, 9, 10, 11, and 12" — and seven of those eight are variants: 10 is listed
by the sentence and retired by its own row ("PDF readers shall ignore this bit"), and the row is
the later and more specific statement, so it has no variant and the enum's comment says why.
Positions 1–2, 7–8 and 13–32 are the reservations, stated on the enum. **Two bits have no
consumer and say so** (trap 11): bit 11's assembling names an operation this program does not
have until `doc/todo/57`'s `split`, `merge` and `pages` exist; bit 12's faithful-copy printing
rests on "an implementation-dependent algorithm" this tree has not chosen — a page raster at any
resolution is a "representation of the appearance", and whether a given resolution is degraded
enough is a question the clause hands to the processor. Naming them is what makes each a lookup
rather than a reading on the day it is needed; consuming them now would be a report firing on a
condition the clause does not state.

**`Operation` has five arms**: the viewer's two and the transform's three, moved in. `Print`
reads bit 3 (a raster is what a print driver produces — a choice, written as one); `Extract`
bit 5; `Modify` bit 4, the residual ADR 0802 argued. `Operation::bit(revision)` is where the
revision rule lives, and it is now one function for all five: bit 9 is a grant from revision 3
and a reserved position before it, so filling in reads bit 6 at revision 2 — the clause's own
−44 example — and bit 9 *or* bit 6 from 3, because bit 9's row says "even if bit 6 is clear".
The other three have one bit at every revision; their rows carry no revision condition.

**§12.8.2.2's certification is read against all five**, which the old split could not do.
Table 257's three levels are all about "changes to the document": rendering a page and copying
a file out change nothing, so no level withholds `Print` or `Extract`; writing a file *in* is a
change none of the three permits — it is not form filling, a page template, a signature or an
annotation — so `Modify` is withheld at every defined level. The corpus's one `/Perms /DocMDP`
(`xfa_filled_imm1344e.pdf`, `/P 2`) is the witness: attach refused, save-all not.

**The four levels are `Level`, in `pdf-model`, and the one place they are applied is
`Level::verdict`** — a pure function from the list `asserted` returns to a `Verdict` the caller
matches exhaustively: `Proceed`, `Warn(reasons)`, `Ask(reasons)`, `Refuse(reasons)`. Nothing
in `pdf-model` refuses; `Refuse` is a value. `decide` is `asserted` then `verdict`, asked once
per operation. Two things about the shape are deliberate. `Level` has no `Default`: a default
is a policy, and each consumer states its own where it makes it (`Off` in `pdf-transform`, `On`
in `viewer-core`). And `Verdict` is not `#[non_exhaustive]`: a consumer that cannot ask has to
write the arm for `Ask` itself, and the compiler is what holds it to that.

**What each consumer does with the two it cannot answer, stated rather than degraded silently.**
`pdf_transform::apply` answers `Ask` with `Refusal::Unanswered`, its own variant so a caller can
tell "refused by policy" from "a question went unanswered" without parsing a sentence — because
going ahead on an unanswered question would be `Off` under another name, and not going ahead is
what a closed dialogue means everywhere else. The command line goes one step further and makes
`--restrictions=ask` a usage error before the file is opened, so that `ask` never looks like a
level the program has. `viewer_core::RestrictionLevel` stays two-valued — the two a window can
answer — with `level()` mapping into `Level`; `Viewer::refusal` asks `decide` and matches the
verdict with `Refuse | Warn | Ask` in one arm that refuses visibly, because neither of the last
two can arrive from this crate today and the arm is what the day they can will split in three.
This is the shape `doc/todo/38` asked for and not the feature: the event a host answers and the
command that answers it are still that file's, and now they are the *only* thing it holds.

`Restriction::AccessDenied` carries a `Bit` rather than a number. The transform words its
reasons for a pipe (`describe_restriction`) and `viewer_core::notes` words the same list for a
window; neither is in `pdf-model`, for the reason that module's comment gives.

### 2. `attachments --attach --to-page N`: §12.5.6.15's annotation, with this tree's own icon

The same embedded file stream and file specification as 870's, and instead of a tree entry an
annotation on the page: Table 187's `/Subtype /FileAttachment`, its required `/FS`, and its
`/Name`, which the table asks writers to include ("PDF writers should include this entry") and
which is therefore always written, `PushPin` — the table's default — where nobody chose; Table
166's `/Rect` and `/P`; and the description in `/Contents`, because the clause's one `shall`
makes that the text a reader shows: "Interactive PDF processors shall use this entry rather than
the optional Desc entry". The page's `/Annots` is rewritten where it is — the array's own object
where the page references one, the page otherwise — for the reason `ViewState::save` gives, and
appended to, because §12.5.2 makes the array's order the drawing order.

**No `/AP` is written.** This tree already synthesises the four icons (`pdf_model::icon`,
ADR for session 266) as its own documented choice, and a stream this crate drew would be a
second artwork for the same clause; the annotation is drawn by the same code the viewer draws
the corpus's annotations with. **Where the annotation sits when nobody says is a choice the
standard leaves open**, and it is stated as one: a 20-unit square 20 units in from the crop box's
upper-left corner — the side this tree draws a text annotation's icon at, and the corner a
reader's eye starts at on a left-to-right page. `--icon` outside the four names is a usage
error, because the viewer would refuse to draw it by name (trap 5) and a file this program
wrote should not be one it then reports.

The test attaches with a stated rectangle, reopens, finds the file listed under its page and the
tree empty, and renders the page with and without its annotations: every differing pixel lies
inside the rectangle's device box, which is what Table 166's `/Rect` promises. The icons were
looked at (trap 1).

### 3. `attachments --remove NAME`: the tree without the entry, and the objects marked free

The tree is rewritten as one node without the entry, the same shape `--attach` writes. What
the entry alone reached — the file specification where the leaf held a reference, and the
streams under its `/EF` — is **marked free in the new section**, because §7.5.6 is a `shall`
about it: "[d]eleted objects shall be left unchanged in the PDF file, but shall be marked as
deleted by means of their cross-reference entries". *Alone* is the condition, and it is
§7.11.4.1's: an embedded file has more than one home, and a stream the catalog's `/AF` or a
page's annotation still reaches is not deleted by the tree letting go of it, so it stays in use
and the report says so.

**Which of §7.5.4's two mechanisms, and why it is not a choice.** The clause gives a
cross-reference section two ways to hold a free entry: the linked list headed by object 0,
which a deletion "shall be added to" with its generation "incremented by 1"; and "other free
entries that link back to object number 0 and have a generation number of 65,535, even though
these entries are not in the linked list itself". An incremental update cannot use the first,
and the clause says so in its own NOTE 3: "cross reference subsections of incremental updates
can never have an object number of zero" — the head of the list is not an entry the new
section may rewrite, and an entry chained to a head that does not name it is not in the list.
So `pdf_syntax::write::incremental_update_freeing` writes the second: `0000000000 65535 f` in
a table, Table 18's type 0 with fields 0 and 65,535 in a stream. A generation of 65,535 is the
one the clause says "shall never be reused", which is also the right statement about a number
whose object's bytes are still in the file under it. This tree's reader has recorded free
entries as deletions since ADR 0100, so the reopened document answers `null` for the number and
`qpdf --check` accepts the file — evidence about the reading, never its definition.

## Consequences

- `doc/todo/57` §3's first item is done; §1 loses `--to-page`; `--remove` is added and done.
  What waits on RFC §13 is exactly the serializer and the four verbs on it.
- `doc/todo/38` narrows to what it always meant: the event a host answers and the command that
  answers it, in `viewer-core` and its three windows. The reading, the levels and the verdict
  are no longer on its list.
- `pdf_transform::Restrictions` is gone; `pdf_transform::Level` is `pdf_model::restriction::Level`.
  `Policy` states its own `Default`.
- `viewer-confined`'s protocol carries three more `Operation` codes; nothing else in the hosts
  changed, because `RestrictionLevel` did not.
- `incremental_update`'s signature is unchanged; `incremental_update_freeing` is beside it.
- A `/Names []` root is what a tree with its last entry removed becomes — Table 36's form with
  no pairs — rather than the name dictionary losing its `/EmbeddedFiles` key, which would be a
  second holder rewrite for the same result.
