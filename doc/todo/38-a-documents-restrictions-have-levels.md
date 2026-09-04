# A document's restrictions are the reader's to set, and they have levels

Status: **the reading, the four levels, the verdict, the events, the command and — since session
916 — the *question* are built (ADR 0212, session 373; ADR 0803, session 872; ADR 0814, session
885; ADRs 0874 and 0875, session 916). Two faces put the question to a person today: KIO through
`WorkerBase::messageBox`, and `pdf-transform` on a terminal. What is left is a way for a person to
*choose* a level, and a dialogue in the three windows.** No user interface is to be built until
the project owner asks for one.
Priority: 38 — capability, and low priority by the owner's own words
Clauses: §7.6.4.2 (Table 22's `/P`), §12.8.2.2 (`/DocMDP`), §12.8.6 and Table 258 (usage rights),
§12.7.5.5 (Table 236's signature field lock — the one restriction addressed to a *named field*
rather than to the document, ADR 0284), §12.7.6.2
Code: `crates/pdf-model/src/restriction.rs`, `crates/viewer-core/src/viewer.rs`,
`crates/viewer-core/src/notes.rs`, `crates/pdf-syntax/src/crypt.rs`,
`crates/pdf-transform/src/lib.rs`

## The policy, in the owner's words

> DRM restrictions are low priority and we should always have the possibility to turn them off. We
> should not implement a UI for them right now, but our finishing product will have a few different
> DRM levels: off, on, ask before operations, warn before operation. I tell this, so that, when we
> encounter them for any reason, they are now implemented in a way, which allows such levels later.

**This is not about the sandbox.** Principle 3's confinement runs the other way — it protects the
reader from the document — and nothing here is negotiable in that direction.

## What the three-hundred-and-seventy-third session built

| clause | what it does now | where |
|---|---|---|
| §12.8.2.2's `/DocMDP` | states which of Table 257's levels withholds which operation | `restriction::asserted` |
| §7.6.4.2's Table 22 | **consulted at last**, for bit 6 and bit 9, with `/R` deciding bit 9 | `restriction::withheld` |

**The population that reaches `withheld` grew in the eight-hundred-and-eighty-seventh session**, which is worth a line here because this file is about what a *document* asserts over a reader: `/R` 5 was refused by `pdf-syntax` and its Table 22 flags therefore reached nothing at all. 33 of the 41 `/R` 5 documents among the 90 535 in `doc/pdf.js`, `doc/corpora/` and `corpus-cache/` now open, 19 of them withholding at least one of the two operations this program has — and their flags arrive through §7.6.4.4.9's encrypted `/Perms` block, which outranks the plaintext `/P`, so what is obeyed for them is the copy a producer could not silently edit. ADR 0820.
| §12.8.6's `/Perms` | composes the two, because the clause says a permission needs *each* handler | `restriction::asserted` |
| §12.8.2.3's `/UR3` | unchanged, and deliberately: a grant is not a restriction | `view::withdrawn_usage_rights` |

The reading is in `pdf-model` and decides nothing; the policy is one value a host supplies
(`Command::Restrict(RestrictionLevel)`), asked **once per `Edit`**; the refusal leaves as
`Event::Refused { document, operation, notes }`, which carries the operation precisely so that it
can become a question. The argument, the corpus measurement and the Table 22 revision finding are
in ADR 0212.

**All three windows supply the value since the seven-hundred-and-twenty-first session, and this
paragraph said `viewer-ui` did and stopped there** (ADR 0604). It was true and it was half the
sentence: `pdf-viewer-gtk` and `pdf-viewer-qt` sent `Command::Restrict` nowhere and could not be in
any level but `On` — while both answered every refusal with a sentence naming
`--ignore-restrictions`, and both argument parsers answered that word with *"is not an option this
program has"* and exit 1. `CLAUDE.md`'s "**it shall always be possible to turn them off**" therefore
held in one program of three, and the sentence promising otherwise was the thing that made it look
closed. `viewer_host::IGNORE_RESTRICTIONS` is the word now and `viewer_host::refused` the sentence
that names it, in one module so that they cannot drift apart again; three tests hold the sentence,
each parser and the whole chain, because the defect lived exactly between two links that each had
one.

## What the eight-hundred-and-seventy-second session built

`pdf_model::restriction::Level` is the four levels; `Level::verdict` is the policy applied, a
pure function to an exhaustive `Verdict` (`Proceed`, `Warn`, `Ask`, `Refuse`, each carrying
every reason); `decide` is the whole question in one call. Every Table 22 position is named in
`Bit`, two of them as consumed by nothing, and `Operation` has the transform's three arms beside
the viewer's two, so §12.8.2.2's certification is read against a page render, an extraction and
a file written in, not only against the viewer's edits. `pdf-transform` consumes all four levels
— a pipe answers *ask* with `Refusal::Unanswered`, and its command line refuses the word before
opening the file — and `viewer-core` supplies `Off` and `On` through `RestrictionLevel::level`
and matches every verdict, the two it cannot produce in one arm that refuses visibly. ADR 0803.

## What the eight-hundred-and-eighty-fifth session built

**The event and the command, which is what the section below called the whole of what was left.**
`RestrictionLevel` has all four levels; `Viewer::standing` asks `decide` once per edit and matches
all four verdicts, none of them quietly:

| level | what a window receives | what happens to the edit |
|---|---|---|
| `Off` | nothing | done |
| `On` | `Event::Refused { document, operation, notes }` | not done |
| `Warn` | `Event::Warned { document, operation, notes }`, **after** the `Dirty` it caused | done |
| `Ask` | `Event::Asking { document, operation, notes }` | held, until `Command::Answer { document, proceed }` |

*Ask* is the `Event::PasswordRequired` shape the section below asked for: the edit is **resolved**
before it is held, so what goes ahead on a `yes` is what was asked for at the moment it was asked
(`open::Done`'s rule); one question is outstanding per document, a second replaces it, and a `no`
forgets the edit and says nothing, because a question declined is neither the document doing
something nor this program refusing. `notes::Standing` chooses the tail of every sentence, because
a reason ending "was not done" is a lie under *warn* and premature under *ask*.

**All four cross every boundary this tree has**: `viewer-confined`'s wire (`ANSWER` as command 26,
`ASKING`/`WARNED`/`ATTACHMENTS_CHANGED` as events 16–18, `RestrictionLevel` as codes 0–3), and the
C ABI as `PDFV_RESTRICT_ASK`, `PDFV_RESTRICT_WARN`, `pdfv_answer` and three event kinds that moved
`PDFV_EVENT_KIND_COUNT` 16 → 19.

**No window has a dialogue yet**, by the owner's word that the gestures follow the HTML mockups, so
each of the four answers *ask* with `viewer_host::unanswerable` and `proceed: false` — out loud, the
same closed-dialogue choice `pdf-transform` made with `Refusal::Unanswered`. That is what keeps the
level from silently behaving like *on*, and it is the one thing a window still owes. **A C host of
`viewer-ffi` is not in that sentence and never was**: `PDFV_EVENT_KIND_ASKING` and `pdfv_answer` are
a channel *and* an answer, so a host on that boundary has been able to ask since this round.

**§7.11.4's attach and detach are the levels' second consumer**, and the third if `pdf-transform`
counts. `Edit::Attach { bytes, name, description, mime, home }` and `Edit::Detach { name }` are
edits in `viewer-core`'s log beside the immutable document, replayed by undo and redo, written by
§7.5.6's incremental update at `Command::Save` and at no other time. **Which of Table 22's bits
governs one depends on §7.11.4.1's home, decided from the table's own words** (ADR 0814, and the
§7.6.4.2 ledger row): a file filed in §7.7.4's `/EmbeddedFiles` tree is bit 4's residual —
"[m]odify the contents of the document by operations other than those controlled by bits 6, 9, and
11" — so `Operation::Modify`; a file filed by §12.5.6.15's annotation is bit 6's "[a]dd or modify
text annotations", because that clause makes the file part of the annotation and bit 4's own row
hands whatever bit 6 controls to bit 6, so `Operation::Annotate`. The consequence is pinned by a
test: a certification at §12.8.2.2's level 3 admits a file on a page and withholds one in the tree.

## What the nine-hundred-and-sixteenth session built

**The *ask* level became askable, and this file's own claim that "nothing in the core has to change
for it" was the thing that was wrong.** Round 913 found it by building the KIO face: RFC 0003 §6
puts every byte of parsing in a confined process and the restriction decision is taken *inside* it,
so the level degraded to a refusal in every face — including the one face with a real question
channel. ADR 0869 §3 costed two ways out; ADR 0874 implemented the one it recommended.

**Two round trips.** `pdf_transform::consult(level, document, operation) -> Consulted` is the
question — the four verdicts, the operation's word, the document's reasons, and
`Consulted::question` for the one verdict that is a question — and `apply` itself now asks it, so a
host that asks and then acts is answered by one reading rather than by two that could disagree.
Across `pdf-vfs`'s confinement it is `Query::Consult { operation }` out and `Answer::Consulted`
back; the operation afterwards is `Query::Consented(Box<Query>)`, which runs the inner query at
`Level::Off` — **the answer crossing, never a second copy of the policy**. `Vfs::consult(path, verb)`
and `Vfs::answer(proceed)` are the broker's shape; the consent is held beside the *worker for that
generation*, so it is spent once, spent only by the operation it was given for, and gone when the
document moves underneath the mount.

| face | can it ask | how | where its level comes from |
|---|---|---|---|
| KIO | **yes** | `WorkerBase::messageBox`, `QuestionTwoActions`; a decline is `ERR_USER_CANCELED` | `PDF_KIO_RESTRICTIONS`, default `off` |
| `pdf-transform` | **yes, on a terminal** | the question on stderr, a line read back; `--restrictions=ask` is a level rather than a usage error | `--restrictions=off\|on\|ask\|warn` |
| a C host of `viewer-ffi` | **yes, since session 885** | `PDFV_EVENT_KIND_ASKING`, `pdfv_answer` | `pdfv_restrict` |
| `pdf-fuse` | **no** — a mount has no dialogue | `EACCES` and the sentence, in full, in the log | `Config::policy`, default `off` |
| the three windows | **not yet** | `viewer_host::unanswerable`, `proceed: false` | `viewer_host::IGNORE_RESTRICTIONS` |

**The default did not move anywhere**, which is the owner's rule: every face still opens at `off`.
And `Refusal::Declined` is a third sentence beside `Restricted` and `Unanswered`, because "this
program is obeying the document", "a reader decided" and "nobody was asked" are three events and
were two.

## What is left

- **A way to choose a level, and a dialogue in the three windows.** A menu with four entries and,
  probably, the per-document override the viewer-wide value does not express today — plus a prompt
  for `Event::Asking` in each window. **A command line is not one**, which is worth restating
  because it is what kept two hosts without any way out for the whole of their lives: nothing in
  the owner's instruction was blocking the flag, and nobody checked. **Nor is an environment
  variable**: `PDF_KIO_RESTRICTIONS` is the only channel a `kioworker` has and it is a placeholder
  for a configuration page, said so in `pdfworker.cpp` rather than left to be discovered.
  The *dialogue* half of this entry is done in two faces (ADR 0875); the *choosing* half is owed
  everywhere but the command line.
- **The gestures that send `Edit::Attach` and `Edit::Detach`.** No drag-and-drop, no command
  palette, no file dialog was built in the eight-hundred-and-eighty-fifth session, by the owner's
  word that the mockups are being reviewed first. What each window gained is the *display* half:
  the files tab is rebuilt from `Query::Attachments` when `Event::AttachmentsChanged` says the list
  moved. The C ABI has `pdfv_attach` and `pdfv_detach` already, because an ABI has no gestures.
- **The payload's descriptor route across the confinement, and the route now exists.**
  `Edit::Attach` ships its bytes on the wire today, as `Command::Open` shipped a document's. Round
  883 made the *document's* descriptor cross with `SCM_RIGHTS`, and that branch was not on `main`
  when 885 branched — which is why the attach was built against the byte route. **Both are on
  `main` since round 889's merge**, so the sentence that used to read *the day a source descriptor
  route exists* is answered — it is `viewer-confined`'s `write_frame`, sending the descriptor as
  `SCM_RIGHTS` beside `open_kind::ON_DISK` — and an attach is the second thing that should take it:
  the host opens the file, the worker never sees a path, and a large attachment stops being copied
  through a pipe. `encode_edit`'s arm 4 is where it lands, and what it needs is a `Payload` that can
  name an open file rather than a `Vec`.
- **Table 22's bit 5, and the copy operation nothing here can name.** The bit is "[c]opy or
  otherwise extract text and graphics from the document", and this crate hands a host a *readback*
  — the same `Query::Selection` that a drag asks sixty times a second in order to draw a
  highlight. Refusing that would refuse the highlight. The bit also carves itself: "for the limited
  purpose of providing this content to assistive technology, a PDF reader should behave as if this
  bit was set to 1", so §14.9's tree must never be gated by it. What is needed is a host saying
  *this is a copy* — plausibly `Query::LogicalSelection`, whose own doc comment already says "[a]
  host asks this when a person presses copy", made to answer differently under the policy. It
  wants an `Answer` variant rather than `Answer::None`, because a copy that came back empty would
  be a lie about the selection.
- **Annex O's `ef`, which is the same four levels arriving from `doc/todo/39`.** "[S]ecurity should
  be strongly considered when opening an embedded file … a PDF processor may choose to prompt the
  user or even prevent opening of the file" — a *prompt*, which is exactly the ask level, over an
  operation (`Command::Extract`) that no document restricts today. The level exists now; what is
  missing is `Operation` reaching that path at all.
- **Assembling and faithful printing** (Table 22 bits 11 and 12) are named in `restriction::Bit`
  and consumed by nothing, each saying why; bit 3 is consumed since session 872, by
  `pdf-transform`'s page render. `Operation` gets an arm for 11 the day `split`, `merge` or
  `pages` exist (`doc/todo/57`), and for 12 only if this tree chooses the "implementation-
  dependent algorithm" the row leaves to the processor.

## What not to do

- **No user interface**, by the owner's instruction, until it is asked for.
- **No level enum shipped with one caller**, which is why two of four were absent rather than
  stubbed for five hundred sessions and arrived in the eight-hundred-and-eighty-fifth *with* the
  event and the command. ADR 0178's lesson, and it is discharged rather than retired: the next
  level-shaped thing here — a per-document override, `Operation` for bit 5 — is under it too.
- **No weakening of what is written.** §7.5.6's incremental update, `Document`'s immutability and
  the signature-withdrawal rule are correctness, not policy: a save that exceeds a usage-rights
  grant must still remove the `/UR3`, because §12.8.6 makes the signature a claim about the file
  and leaving it would make the file lie. Turning the *restriction* off is the reader's; making
  the file assert something untrue is not. The types keep the two apart —
  `RestrictionLevel` reaches `Viewer::refusal` and nothing else, and `withdrawn_usage_rights` is
  reached from `ViewState::save` with no policy in scope at all — and it must stay that way.
- **§12.7.6.2's submit is still not one of these**, re-checked in the three-hundred-and-seventy-third
  session: it is refused because it needs a network this program does not have (principle 3), which
  is a capability rather than a permission, and no level would turn it on.
