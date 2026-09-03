# 0814 — A file attached is an edit, the four levels reach a window, and the writer moves beside its reader

Session 885. Status: **accepted**.

## Context

The project owner, on 2026-09-03: *"I am missing adding embedded files."* `doc/todo/38`'s "no
user interface is to be built until the project owner asks for one" is therefore answered for
this one feature — and this round builds the **core** of it, not the gestures, because the owner
is reviewing HTML mockups of the flows and the GTK, Qt and winit gestures follow that review.
What every gesture will share is what this round had to build completely: attaching and
detaching as *edits* in `viewer-core`'s log beside the immutable document; the restriction policy
asked once at the seam, with the two levels `doc/todo/38` said were still owed — *ask* and
*warn* — as the event a window receives and the command that answers it; the files sidebar
listing the log's view; and the edit crossing the confined worker's boundary the way every other
edit does.

Four clauses decide it, and each is quoted where it binds. §7.11.4.1 gives an embedded file its
two homes — "An embedded file stream shall be included in a PDF file in one of the following
ways:" — a file specification's `/EF`, which "shall be used for file attachment annotations (see
12.5.6.15, "File attachment annotations"), which associate the embedded file with a location on a
page in the document", and the name dictionary's `EmbeddedFiles` entry, which associates one
"with the document as a whole". §12.5.6.15 makes the annotation's file part of the annotation:
"A file attachment annotation ( PDF 1.3 ) contains a reference to a file, which typically shall be
embedded in the PDF file (see 7.11.4, "Embedded file streams")." §7.5.6 is the one form of writing
`CLAUDE.md` permits — "When updating a PDF file incrementally, changes shall be appended to the
end of the file, leaving its original contents intact." — and it says what a deletion is:
"Deleted objects shall be left unchanged in the PDF file, but shall be marked as deleted by means
of their cross-reference entries." §7.6.4.2's Table 22 governs what a reader may do, and two of
its rows are the question ADR 0803's mockup round could not settle.

## Decision

### 1. The shared writer lives in `pdf-model`, beside the reader it mirrors

The instruction was to reuse `pdf_transform::attachments`' writing rather than build a second
one, exposing a function there if the seam needed it. The seam needed more than a function: the
crate graph runs `pdf-transform → viewer-core` — that crate takes `viewer_core::Secret` (ADR
0800) — so `viewer-core` cannot depend on the transform without a cycle. And the viewer's save
has to *compose* an attachment with everything else its log writes into one §7.5.6 update: a
markup and a file on the same page both rewrite that page's `/Annots`, and the tree's holder may
be the catalog, which the usage-rights withdrawal may rewrite after. Two writers building
objects into two maps cannot compose; one writer building objects for a caller's map can.

So the objects moved: `pdf_model::attachment::filing` builds §7.11.4's stream (Table 44's
`/Type`, `/Subtype` where a media type is known, Table 45's `/Size` and `/CheckSum` from the
bytes, unfiltered), §7.11.3's specification (Table 43, `/F` and `/UF` both the filing name,
`/EF` under both keys, `/Desc` where given), §12.5.6.15's annotation (Table 187's `/FS` and
`/Name`, `/Contents` for the description because that clause's one `shall` makes it the text a
reader shows), §7.7.4's tree as one `/Names` node in §7.9.6's order, the holder rewrite at the
nearest indirect object, and the "alone" condition on freeing what a removed entry reached.
`Payload` moved with them. **Nothing in the module allocates a number or writes a byte**: the
transform allocates from the file's highest number into a `BTreeMap`, and `ViewState` from a
counter it shares with the annotations it adds, into its `Update`. The transform's `attach`,
`remove` and `file_on_page` are now the verb — refusals, output, report — over those functions,
and the `md-5` dependency went with the digest. `Tree::read` takes a `current` function so that
a writer composing several changes reads the tree as its earlier changes left it.

### 2. An attachment is `ViewState`'s, like an annotation, and a page's file *is* an annotation

`ViewState` holds `filed: Vec<Filed>` — every file attached this sitting, with the two object
numbers its stream and its specification will be written under — and `unfiled: Vec<Vec<u8>>`,
the keys of the document's own tree entries detached this sitting. `attach` refuses a name the
tree (less what was detached) or this sitting already files, under §7.9.6, and it is **one
namespace across both homes**, so that a detach by name is never ambiguous. Where the home is
a page, the §12.5.6.15 annotation is pushed onto the *additions* at once, under
`filing::file_attachment_annotation` with Table 187's default `PushPin`: the page draws its icon
before anything is saved, through the same three functions every added annotation is drawn by,
and `write_additions` writes it and appends it to the page's `/Annots` exactly as it writes a
markup. The save's new half, `write_filings`, goes first in the update and reads through
`Update::current`: streams and specifications, then the tree rewritten once — the document's
entries less the detached ones plus the attached ones — with what a detached entry alone reached
handed to `incremental_update_freeing` and what another home still reaches named in
`Written::still_reached`, which the viewer reports. `withdrawn_usage_rights` now reads the
catalog as the update has it, which it did not need to before because nothing rewrote the
catalog ahead of it.

**Object numbers are never handed out twice in one sitting.** `next_free_object` counted the
additions, and a file detached again would have given its numbers to the next annotation, which
the log names by number. `allocate` counts everything handed out and the counter starts again
only when nothing allocated is held, which is exactly when a replay clears the state.

**`Query::Attachments` answers the log's view, for the tree's home.** The list is the document's
own — the tree and the catalog's `/AF` — less what was detached, plus what was attached to the
document as a whole; each attached file's record is the one `attachment::read` would build from
the objects the save writes, with the same unfiltered stream, so `Command::Extract` decodes the
same bytes before the save as after it. A file attached to a *page* is deliberately not in it:
after a save and a reopen it is in the home the tree does not list (ADR 0295), and a list that
showed it before the save and not after would change under a person who changed nothing. It is
where §12.5.6.15 put it, on the page, as its icon.

### 3. Two edits, one answer, three events, and the level a page home reads

`Edit::Attach { bytes, name, description, mime, home }` and `Edit::Detach { name }`, with
`AttachHome::Document` and `AttachHome::Page { at }` — a viewport point in device pixels, mapped
like `Edit::FreeText`'s corners through the transform the frame was drawn with, to the page
under the point rather than the current page, because Table 29's continuous arrangements put
several on the screen and a person dropped the file on one. The icon is a 20-unit square centred
on the point (`filing::rect_around`), a stated choice: a person who drops a file on a spot means
the spot. `Done::Attach` records the resolved `Filing`, so a replay hands `ViewState::attach` the
same page and rectangle; `Done::Detach` the name. The bytes are a `Payload` that prints as its
length, for `Secret`'s reason: two hosts trace a command by printing it.

**Which bit governs a file attached on a page, decided from the text.** ADR 0802 read bit 4 for
the tree: "Modify the contents of the document by operations other than those controlled by bits
6, 9, and 11." is the residual no other bit names, and an embedded file in the tree is none of
the three. The same row settles the page: bit 6 is "Add or modify text annotations, fill in
interactive form fields, and, if bit 4 is also set, create or modify interactive form fields
(including signature fields)", and bit 4's own text hands whatever bit 6 controls to bit 6. A
file attachment annotation is one of Table 171's annotations, and §12.5.6.15 makes its file part
of it — "contains a reference to a file, which typically shall be embedded in the PDF file" — so
the annotation and the file it carries are one act of annotating, as a markup and the appearance
stream it carries are. Two readings were weighed and refused: bit 4 alone (the stream is a
modification), which would make adding an annotation fall under a bit whose own row excludes
what bit 6 controls; and both bits at once, which `Operation` has no arm for and which would make
`Event::Refused` carry two operations. So `Edit::Attach` to a page is `Operation::Annotate` and to
the document `Operation::Modify`; `Edit::Detach` is `Modify`, because taking an entry out of the
tree is the residual again. The consequence is stated rather than hidden, and pinned by a test: a
certification at §12.8.2.2's level 3, which permits "annotation creation", or a Table 22 word
granting bit 6 and not 4, admits a file on a page and withholds one in the tree. The row's words
are "text annotations", and this decision inherits the reading `operation_of` already makes for
§12.5.6.10's markups and §12.5.6.6's free text rather than making a new one: Table 22's row
predates Table 171's family and names the family by its oldest member.

**The four levels.** `RestrictionLevel` gains `Ask` and `Warn`, mapped onto
`pdf_model::restriction::Level` one to one. `Viewer::standing` asks `decide` once per edit and
matches all four verdicts, with the notes worded for each by `notes::restricted` — a sentence
that ends "was not done" is a lie under *warn* and premature under *ask*, so `Standing` chooses
the tail. *Refuse* is `Event::Refused`, unchanged in shape, the answer of exactly one level; the
bit is named in its notes. *Warn* commits the edit and then sends `Event::Warned`, after the
`Dirty` it caused, so a host reading events in order sees the state and then the sentence. *Ask*
resolves the edit, holds it as `Open::asking`, and sends `Event::Asking`; `Command::Answer {
document, proceed }` takes it — `true` commits exactly as `Off` would have, `false` forgets it
and says nothing, because a question declined is neither the document doing something nor this
program refusing. One question is outstanding per document, and a new edit that needs asking
replaces it. Every window has an arm for both events; none has a dialogue yet, so each answers
*ask* with `viewer_host::unanswerable` and `proceed: false`, out loud — the same closed-dialogue
choice `pdf-transform` makes with `Refusal::Unanswered` — rather than letting the level behave
like *on* in silence. The four levels reach the C ABI as `PDFV_RESTRICT_ASK` and `_WARN` with
`pdfv_answer`, and the wire as codes 2 and 3 with `ANSWER`.

**`Event::AttachmentsChanged { document }`** is the one message this vocabulary gained that is
not one of the three named above, and it passes `doc/ui-boundary.md`'s test: a host sent the
edit, but it did not send the *verdict* — an attach under `On` moved nothing — and an undo names
no edit at all. `viewer-ui` re-queries the one list it cached at open, `viewer-gtk` rebuilds the
files tab's slot, `viewer-qt` rebuilds its panels, and the confined window, which shows no panel,
says so in an arm rather than a wildcard. That is the whole of what a frontend gained: nothing
that is a gesture.

### 4. The bytes cross the confined boundary whole

Round 883's descriptor route for a document did not exist on `main` when this round branched, so
`Edit::Attach` ships its bytes on the wire, as `Command::Open` ships a document's. `doc/todo/38`
records the descriptor route as owed the day one exists for a source.

## Consequences

- `pdf_transform::attachments` is the verb over `pdf_model::attachment::filing`; its output is
  unchanged, and `writer.rs`'s and `writer_corpus.rs`'s tests are the evidence.
- `viewer-core` gained two `Edit` variants, one `Command`, three `Event`s and two levels; every
  consumer failed to compile and now says what it does, and `PDFV_EVENT_KIND_COUNT` moved 16 →
  19. `doc/ui-boundary.md` records it.
- `Written::still_reached` is a fourth thing a save says out loud, beside the two it already
  did.
- What the gestures still owe, and where the mockups decide it: a drop or a dialog that sends
  `Edit::Attach`, a row action that sends `Edit::Detach`, and a prompt for `Event::Asking` in
  each window — and a way to set `Ask` and `Warn` at all, since the command line is not one.
  `doc/todo/38` holds the list.
