# A document's restrictions are the reader's to set, and they have levels

Status: **the reading, the four levels, the verdict, the events, the command, the question, a level
*per restriction*, a per-document scope, the copy operation, a command line, a menu in all three
windows and the prompt the *ask* level needs are built** (ADR 0212, session 373; ADR 0803, session
872; ADR 0814, session 885; ADRs 0874 and 0875, session 916; ADR 1144, session 1147; ADR 1145,
session 1155). **The owner lifted the no-interface deferral on 2026-09-16.** §12.11.6's requirements processing
joined the six as a seventh operation in session 1165 (ADR 1167), which is the first one that is
not a verb a person presses: what its level decides is whether a document is opened at all. What is
left is the *attach and detach gestures*, which wait on the owner's HTML mockups, and `Assemble`,
which awaits a verb this program does not have. `Print` has one since session 1171 (ADR 1180):
`Command::Print` is the operation and `viewer_host::WindowAct::Print` is the key that sends it.

**A second policy with the same four levels sits beside this one and is not part of it**, because
the direction is the other one: `viewer_host::Links` is what this machine does when §12.6.4.8's
link asks it to start another program on a string the *document* chose (`--links=refuse|ask|warn|
open`, `ask` by default, ADR 1155). The levels here are a reader deciding how much of what a
document asserts *over them* to obey; there they are a reader deciding what a document may ask
their machine to do. One vocabulary for both would have made `off` mean the permissive end in one
and the restrictive end in the other.
Priority: 38 — capability, and low priority by the owner's own words
Clauses: §7.6.4.2 (Table 22's `/P`), §12.8.2.2 (`/DocMDP`), §12.8.6 and Table 258 (usage rights),
§12.7.5.5 (Table 236's signature field lock — the one restriction addressed to a *named field*
rather than to the document, ADR 0284), §12.7.6.2
Code: `crates/pdf-model/src/restriction.rs`, `crates/viewer-core/src/viewer.rs`,
`crates/viewer-core/src/command.rs`, `crates/viewer-core/src/notes.rs`,
`crates/viewer-host/src/restriction.rs`, `crates/pdf-syntax/src/crypt.rs`,
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
sentence: `quorra-gtk` and `quorra-qt` sent `Command::Restrict` nowhere and could not be in
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
C ABI as `QUORRA_RESTRICT_ASK`, `QUORRA_RESTRICT_WARN`, `quorra_answer` and three event kinds that moved
`QUORRA_EVENT_KIND_COUNT` 16 → 19.

**A window with no dialogue answers *ask* with `viewer_host::unanswerable` and `proceed: false`** —
out loud, the same closed-dialogue choice `pdf-transform` makes with `Refusal::Unanswered` — which is
what keeps the level from behaving like *on* in silence. One face is still in that sentence and it
is the one that cannot be asked at all: `quorra-confined` performs no restricted operation. **A C
host of `viewer-ffi` was never in it**: `QUORRA_EVENT_KIND_ASKING` and `quorra_answer` are a channel
*and* an answer.

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
| a C host of `viewer-ffi` | **yes, since session 885** | `QUORRA_EVENT_KIND_ASKING`, `quorra_answer` | `quorra_restrict` |
| `pdf-fuse` | **no** — a mount has no dialogue | `EACCES` and the sentence, in full, in the log | `Config::policy`, default `off` |
| the three windows | **yes, since session 1155** | a modal window apiece, worded by `viewer_host::asked` | `viewer_host::RESTRICTIONS`, and the menu |
| `quorra-confined` | **no** — and no question can reach it | `viewer_host::unanswerable`, `proceed: false` | `viewer_host::IGNORE_RESTRICTIONS` |

**The default did not move anywhere**, which is the owner's rule: every face still opens at `off`.
And `Refusal::Declined` is a third sentence beside `Restricted` and `Unanswered`, because "this
program is obeying the document", "a reader decided" and "nobody was asked" are three events and
were two.

## What the one-thousand-one-hundred-and-forty-seventh session built

**A level per restriction, `off` as the default, a way to set one, and the copy operation.** ADR
1144 has the argument and the census; the shape is:

| | where |
|---|---|
| `RestrictionPolicy` — one `RestrictionLevel` per `Operation`, total over all six | `viewer_core::command` |
| every entry `Off` by default, in the viewer as in every other face | `RestrictionPolicy::default` |
| `Command::Restrict` carries the policy; `uniform` is the one value it used to carry | `viewer_core::command` |
| `Command::Copy` / `Event::Copied` — §7.6.4.2 bit 5 as an *operation* | `Viewer::copy`, `Viewer::standing` |
| six level bytes on the wire, in `RestrictionPolicy::OPERATIONS`'s order | `viewer-confined` |
| `quorra_restrict_operation`, `quorra_copy`, `quorra_event_copied`; kinds 20 → 21 | `viewer-ffi` |
| `--restrictions=copy:ask,annotate:on`, one parser for three windows | `viewer_host::restrictions` |

**The census this rests on**, over the 90 763 files of `doc/pdf.js`, `doc/corpora/` and
`corpus-cache/`: 2 186 state an `/Encrypt` this reader reads, 4 open as the owner, **1 539 withhold
copying** and **1 901 withhold copying or annotating**. Bit 5 was consulted by `pdf-transform` and
by nothing a person could press.

**`Assemble` is an entry with no verb behind it in this crate**, deliberately: a policy with a hole
in it would have to grow a message to fill it, and the day a window gains the verb the level is
already the reader's to set. That is what happened to `Print`: the level was settable before there
was anything to set it about, and when `Command::Print` arrived it was already the reader's.

## What the one-thousand-one-hundred-and-fifty-fifth session built

**The menu, the prompt and the scope a viewer-wide policy could not express.** ADR 1145 has the
argument; the shape is:

| | where |
|---|---|
| `Command::Restrict(RestrictionScope)` — the window's levels, or the focused document's departures | `viewer_core::command` |
| `RestrictionOverride` — one *optional* level per operation, `NONE` being a document at the window's | `viewer_core::command` |
| `RestrictionPolicy::under` — the whole of the layering, asked in `Viewer::standing` | `viewer_core::viewer` |
| the departure held beside the document, so it ends when the document does | `viewer_core::open::Open` |
| the menu's rows, the two scopes, the state it edits, the question and the decline | `viewer_host::restriction` |
| `Key::R` and `WindowAct::Restrictions` — the menu's key in all three windows | `viewer_host::keys` |
| a `gio::Menu` behind `set_create_popup_func`; a modal `gtk4::Window` for the question | `viewer-gtk` |
| a `QMenuBar` refilled on `aboutToShow`; a `QDialog` for the question | `viewer-qt` |
| the same rows drawn flat on a card, answered by the arrows and two keys | `viewer-ui`'s `quorra` |
| a scope byte beside the six level bytes, `NO_DEPARTURE` being the one that is not a level | `viewer-confined` |
| `quorra_restrict_document_operation` and `QUORRA_RESTRICT_INHERIT`; `QUORRA_ABI_VERSION` unmoved | `viewer-ffi` |

**Every window builds its menu when it is opened and never before.** `CLAUDE.md` section 2's rule,
and the only way the ticks can be right: the levels change while the window is up.

**§12.2's `/HideMenubar` is read, answered and deliberately not obeyed.** The only menu bar these
windows have is the one holding the reader's levels, and a document that could hide it would be
taking away the control over itself; `viewer_host::restriction::NOT_THE_DOCUMENTS_TO_HIDE` names the
clause, the entry and the reason. `/HideToolbar` and `/HideWindowUI` are obeyed, and Table 29's full
screen still takes the bar — that sentence is the reader asking rather than the document.

## What is left

- **A pointer on the menu `viewer-ui` draws.** Its rows are answered by the arrows and Enter, which
  is what every other card in that window is answered with — there is no window manager behind them.
  A click model exists in that crate (`ChoiceList`) and is deliberately not used here: it is there
  because §12.7.5.4's list is a control the *document* placed at a point on a page, and a menu is
  not (trap 17 — this is a choice about what the window is, not a claim about winit).
- **The gestures that send `Edit::Attach` and `Edit::Detach`, and they wait on the mockups the
  owner asked for on 2026-09-03** — HTML, per platform, demonstrating the functionality rather than
  the look. None has been supplied, so no drag-and-drop, no command palette and no file dialog has
  been built, which is the eight-hundred-and-eighty-fifth session's ruling standing unchanged. What each window gained is the *display* half:
  the files tab is rebuilt from `Query::Attachments` when `Event::AttachmentsChanged` says the list
  moved. The C ABI has `quorra_attach` and `quorra_detach` already, because an ABI has no gestures.
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
- **Table 22's bit 5 is consulted since session 1147, and it took a command rather than a query.**
  The entry above said a host had to say *this is a copy* and guessed at `Query::LogicalSelection`;
  the guess was wrong for a reason it named and one it did not. A readback cannot be refused without
  refusing the highlight a drag draws — and a query raises no events, so nothing can wait on one and
  the *ask* level cannot exist over it at all. So `Command::Copy` is the gesture, `Viewer::standing`
  is the consultation, and `Event::Copied` carries **both** of §14.8.2.5's orders because
  `viewer_host::copied` already chooses between them once for three windows and a C caller. §14.9's
  tree is still a query and is gated by nothing, on Table 22's own carve-out. ADR 1144.
- **§12.7.5.5's Table 236 `/P` is the sixth reason `asserted` returns**, since ADR 1156 discharged
  ADR 0502's deferral: what made the reading safe to take is this file's own default, because a
  level every face opens at *off* withholds nothing from a reader who did not ask for it. It is the
  standing answer to a two-voiced entry — read it, route it through the levels, and let the reader
  decide.
- **§12.11.6's requirements processing is the seventh operation**, since session 1165: the penalty
  a document's unmet requirements total, past §12.11.3's threshold, reaches the same four levels as
  the six Table 22 positions do. It is the one asked where a document *opens* rather than at a
  gesture, so `on` raises no `Event::Opened` at all, `ask` holds the whole open until
  `Command::Answer` — the one question whose `no` has something to undo, because a document held
  before it was processed was never opened — and `warn` opens it and says so afterwards. `off` is
  still the default. ADR 1167.
- **Annex O's `ef`, which is the same four levels arriving from `doc/todo/39`.** "[S]ecurity should
  be strongly considered when opening an embedded file … a PDF processor may choose to prompt the
  user or even prevent opening of the file" — a *prompt*, which is exactly the ask level, over an
  operation (`Command::Extract`) that no document restricts today. The level exists now; what is
  missing is `Operation` reaching that path at all.
- **Assembling and faithful printing** (Table 22 bits 11 and 12) are named in `restriction::Bit`
  and consumed by nothing, each saying why; bit 3 is consumed by `pdf-transform`'s page render and,
  since session 1171, by a window's own `Command::Print`. `Operation` gets an arm for 11 the day
  `split`, `merge` or `pages` exist (`doc/todo/57`), and for 12 only if this tree chooses the
  "implementation-dependent algorithm" the row leaves to the processor.

## What not to do

- **No attach or detach gesture ahead of the mockups.** The owner's word on 2026-09-03 asked for
  HTML mockups of *adding embedded files* in the GUIs, per platform, before those flows are built,
  and none has been supplied — so no drag-and-drop, no command palette and **no file dialog**, which
  is session 885's ruling unchanged. It binds that feature and not this one: the restriction menu
  and the *ask* prompt are chrome for a policy the reader sets, they name no file and open no file
  dialog, and the interface deferral over them was lifted on 2026-09-16 (ADR 1145).
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
