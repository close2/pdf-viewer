# 0830 — A page edit is an integer, a permutation, and a second page object

Session 893. Status: **accepted**. The tenth decision record of RFC 0002's implementation, on the
long-lived branch `round-867`, and the suite's third verb on ADR 0817's serializer.

## Context

RFC 0002 §6.2 proposes two verbs and leaves one question open about them:

> whether `pages` and `merge` are truly two verbs or one (`merge` with a single input and edit
> flags subsumes `pages`; two verbs are kept in the proposal because the common cases read better)

Session 888 landed `merge` with every document-level reconciliation derived from a clause (ADR
0821). `pages` is the other half of §6.2 — delete, insert, reorder, rotate over one document — and
almost all of its machinery is `merge`'s already. What it needs of its own is three things the RFC
names and does not decide: what a *relative* rotation composes with, what happens when one page
takes two places in the output, and where the boundary between the two verbs actually falls.

`CLAUDE.md`'s exclusion, redrawn in session 886 and amended again since, states the test this verb
has to pass: **does the operation invent marks?** It names rotation as the example of one that does
not — "it writes an integer the producer's renderer already honours".

## Decision

### 1. The boundary between `pages` and `merge` is the count of files, not the kind of edit

`pages` reads **one** document; `merge` reads several. That is what RFC 0002 §4.1 already says of
both ("one input, one PDF output" against "many inputs, one PDF output"), and it settles §6.2's
open question without either verb becoming a mode of the other.

The consequence is `--insert`. It takes a range of *this* document and puts a second copy of those
pages somewhere else in it; a path in the argument is a usage refusal naming `merge`. Inserting
another file's pages is cross-file renumbering, which is the thing `merge` exists to do, and a
second implementation of it in this module would be two answers to one question.

The engine is shared rather than duplicated: `merge::write` takes a list of `Placement`s — a
document, a page of it, and the `/Rotate` an edit decided — and writes a file. The two verbs
differ only in how they build the list. So every reconciliation ADR 0821 derived applies to a page
*leaving* exactly as it applies to a page arriving, with no second construction to keep in step.
That is why `pages` carries §8.11's groups, §12.7's form, §7.9.6's name trees, §12.3.3's outline,
§12.4.2's labels, §14.11.5's output intents and §12.8.1's unsigned signature fields without a line
of new code for any of them.

### 2. A relative rotation composes with the page's **effective** `/Rotate`, not with what it states

Table 31:

> ( Optional; inheritable ) The number of degrees by which the page shall be rotated clockwise
> when displayed or printed. The value shall be a multiple of 90. Default value: 0 .

An unsigned angle is absolute and a signed one is relative — qpdf's spelling, which RFC 0002 §6.2
adopts. The question the RFC does not answer is what the sign is relative *to*, and §7.7.3.4
decides it:

> If such an attribute is omitted from a page object, its value shall be inherited from an
> ancestor node in the page tree.

> If the attribute is optional and no inherited value is specified, the default value shall be
> used.

So `--rotate +90` composes with the value that clause gives the page: `+90` on a page that states
nothing and inherits `/Rotate 90` writes 180, and on a page that inherits nothing writes 90. This
is the only reading under which one flag means one thing — "turn this page a quarter turn from how
it is displayed" — for every page in a document. Composing with the *stated* value would make the
same flag do different things to two pages a reader shows identically, which is a difference the
user cannot see and therefore cannot have meant.

Two consequences, both documented as choices rather than derived:

- **The value written is reduced modulo a whole turn**, into 0, 90, 180 or 270. `-90` and `270`
  name the same displayed page and the clause's constraint is "a multiple of 90" rather than a
  range, so both are legal and the smallest non-negative one is written. `/Rotate 0` is written as
  no entry at all, which is what "Default value: 0" makes it.
- **A page whose effective rotation is not a multiple of 90** has broken the clause's `shall`.
  Where nothing is asked of it, it crosses as its producer wrote it — this verb does not correct a
  file it was not asked to touch. Where a rotation *is* asked of it, an absolute angle replaces it
  and a relative angle is composed against 0 with a warning naming the page and the value, because
  there is no quarter turn from 45 degrees. An angle the *caller* gives that is not a multiple of
  90 is `Refusal::Rotation`, a usage error: guessing which quarter turn was meant would be this
  program deciding what a document says.

### 3. Annotations are not touched by a rotation, and §12.5.3 says so in as many words

An annotation's `/Rect` is "in default user space units" (Table 172) and `/Rotate` does not change
default user space — it is a display instruction, "when displayed or printed". The entry that looks
like it bears on the file is Table 167's bit 5:

> (PDF 1.3) If set, do not rotate the annotation's appearance to match the rotation of the page.
> The upper-left corner of the annotation rectangle shall remain in a fixed location on the page,
> regardless of the page rotation.

and §12.5.3 states the same thing from the other side:

> Similarly, if the NoRotate flag is set, the annotation shall retain its original orientation on
> the screen when the page is rotated (by changing the Rotate entry in the page object; see 7.7.3,
> "Page tree").

The clause names *this very edit* — changing `/Rotate` — and puts the whole of its consequence on
the **viewer**: the stored `/Rect` does not change, the compensating transform is applied at
display time, and the annotation pivots around the rectangle's upper-left corner.
`pdf_model::annotation` already implements that side of it (`Page::rotate` normalised to 0, 90, 180
or 270, and the `NoZoom`/`NoRotate` transform in default user space). So a `pages --rotate` that
rewrote an annotation's `/Rect` would be moving an annotation the standard says stays where it is.
Nothing is written to any annotation.

### 4. A page in two places is two page objects, with its annotations copied and its content shared

Table 31:

> ( Required; shall be an indirect reference ) The page tree node that is the immediate parent of
> this page object

One `/Parent`, so one place in the tree. `merge` answers a page named twice with
`Refusal::PageTwice`, because a merge that named one page twice is a plan whose author meant
something the verb cannot distinguish from a mistake. `--insert` is the case where duplication is
what was *asked for*, so the second and later placements cross as their own page objects
(`Duplicates::Copy`). The first placement still stands in for the source page, so every reference
into the document — a destination, an outline item — reaches a page rather than §7.3.10's null.

Everything below the page is shared by reference: the content stream, the resources, the fonts.
Nothing in a page's closure points back at the page **except its annotations**, and Table 172's
`/P` is "[a]n indirect reference to the page object with which this annotation is associated" —
one page. So a duplicated page gets its own annotation objects. They are numbered before any of
them is built, so that an annotation naming another — §12.5.6.14's `/Popup`, §12.5.6.10's `/IRT` —
names the copy on this page rather than the original on the other one.

**A page carrying a §12.7 widget is refused by name.** §12.7.4.2 makes the fully qualified field
name a field's identity and closes with:

> In addition, actual field dictionaries with the same fully qualified field name shall have the
> same field type ( FT ), value ( V ), and default value ( DV ).

A duplicated widget is one of two things: a second field under a name that clause governs — a
field this program would have invented — or a second representation of the same field, which needs
an entry written into that field's own `/Kids`, in an object the plan never asked this verb to
touch. Both are a form edited rather than a page duplicated, so `Refusal::DuplicateWidget` names
the page and the clause and exits 4. It is the same shape as `merge`'s `FieldCollision`: the
request is well formed, the file is readable, and this program declines to write a document the
clause forbids.

### 5. A deletion is the reconciliations running the other way, and §12.3.2.2 makes it safe

§12.3.2.2 settles both the reorder and the delete in one sentence:

> In each case, page is an indirect reference to a page object (except in a remote go-to action;
> see 12.6.4.3, "Remote Go-To actions", or an embedded go-to action; see 12.6.4.4, "Embedded
> Go-To actions").

A destination in this document is a *reference*, so it follows its page through any permutation
with nothing to rewrite, and a reference to a page the output does not hold is §7.3.10's null,
counted and warned. An outline item keeps its place in the chain with a null destination rather
than being removed, because removing it would rebuild a chain the source stated — ADR 0821's rule,
unchanged.

The one destination form that does not follow is the clause's own exception: a remote or embedded
go-to states "an integer page number within the remote document instead of a page object in the
current document". That integer names a position in *another* file and is carried unchanged,
because it is not this document's to fix.

§12.4.2's labels are the positional half:

> the indices shall be fixed, running consecutively through the document starting from 0 for the
> first page, but the labels may be specified in any way that is appropriate for the particular
> document

A deletion or a reorder moves every later index, so no labelling range of the source's tree
survives; the merged construction — one entry per output page, reproducing the label that page had
— is what keeps a surviving page's own identification. The corpus walk asserts exactly that, page
by page, over every document that states `/PageLabels`.

### 6. Operations compose left to right over the running page list

RFC 0002 §6.2 states the rule and this verb keeps it: each range is resolved against the list as
the operations before it left it, so `--delete 3 --delete 3` takes out the third page and then the
page that moved into its place. `--help` says so, because the alternative — every range against
the source's original numbering — is equally defensible and only the documentation decides between
them. Label addressing (`@iv`) resolves against the page standing at that position, for the same
reason.

## Consequences

- `pages` is `crates/pdf-transform/src/pages.rs`, about four hundred lines, of which the module
  comment is more than half. The engine it runs on is `merge.rs`'s, changed from a tuple of
  `(document, page)` to a `Placement`, plus the duplication path and the `/Rotate` override.
- Three new refusals: `Rotation` and `Position` exit 1 (a wrongly written argument), and
  `DuplicateWidget` exits 4 (well formed, and declined by name).
- `Origin::Edited` is the report's word for what this verb writes, beside `Piece` and `Merged`.
- Table 22's bit 11 covers it with no new reading at all: the bit's sentence is "[a]ssemble the
  document (insert, rotate, or delete pages and create document outline items or thumbnail
  images)", which names three of this verb's four operations in order.
- The structure tree is still not carried, by this verb or any other. ADR 0831 is the honest
  statement of what that costs and why a half-carry would be worse.
