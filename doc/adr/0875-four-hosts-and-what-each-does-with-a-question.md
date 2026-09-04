# 0875 — Four hosts and what each does with a question

Session 916. Status: **accepted**. The second of this round's two records: which faces can now put
`CLAUDE.md` principle 3's *ask* question to a person, how each one does it, where each gets its
level from, and what the face that cannot ask says instead. ADR 0874 is the construction underneath.

## Context

ADR 0874 made the question crossable. It does not by itself make any face ask: a question that can
be asked and is not is the same refusal it was before. This record is the four faces, one by one,
and the honest limit on each.

The rule they are all measured against is `CLAUDE.md`'s: the four levels are the reader's, the
default stays `off` because "it shall always be possible to turn them off", and **a host with no way
to ask must still give a named refusal that says why** — never a silent proceed, which would be
`off` wearing another name.

## Decision

### 1. KIO — a modal dialogue, which is why this face was singled out

`PdfWorker::mayProceed` runs before `get`, `put` and `del`. It calls `pdfvfs_consult` with the verb
it is about to perform; on `PDFVFS_VERDICT_ASK` it puts `pdfvfs_consultation_question`'s sentence
through `KIO::WorkerBase::messageBox(QuestionTwoActions, …)`, carries the answer back with
`pdfvfs_answer`, and then performs the verb unchanged.

Three decisions inside that:

- **The other three verdicts are not shown as a dialogue.** `PROCEED`, `WARN` and `REFUSE` are
  statements, and the verb itself is what acts on them — `WARN`'s sentences arrive as the commit's
  warnings through `warning()`, `REFUSE`'s as the verb's own refusal in the core's words. A dialogue
  for any of the three would be asking somebody to decide something already decided.
- **A person who declined gets `ERR_USER_CANCELED`, not the boundary's refusal.** Issuing the verb
  after a `no` would produce "the restriction level is ask and there was nobody to ask", which is a
  sentence about a question they *did* answer.
- **`put` asks before it reads a byte off the socket**, so a person declining is not first made to
  wait for a file they are about to refuse to write.

**The level this face opens mounts at is `PDF_KIO_RESTRICTIONS`**, taking `off`, `on`, `ask`, `warn`,
defaulting to `off`. An environment variable is a poor interface and it is the only channel a KIO
worker has: it is started by `kioworker`, has no configuration dialogue, and RFC 0003 §7 forbids it
from knowing anything both faces do not. A word the plugin does not know is a refusal naming the
word rather than a guess — the same rule the boundary applies to an unknown level number.

**The honest limit is ADR 0869's, unchanged.** `messageBox` needs a client with a UI delegate, and
`kio/test/drive_the_worker.cpp` is a `QCoreApplication` with no session, so the dialogue itself is
not driven by any gate. What the harness does drive is the round trip's plumbing — every verb now
consults first, and the level reaches the mount from the environment, checked twice: a level the
plugin knows changes nothing about a document that restricts nothing, and a word it does not know is
refused by name. The dialogue is owed a session, like Dolphin's own entry and look (`doc/todo/58`).

### 2. The command line — a terminal, and only a terminal

`--restrictions=ask` **used to be a usage error**, refused before the file was opened, on RFC 0002
§13's fourth open question: "a pipe cannot 'ask'". Half of that is still true and is unchanged; the
other half was wrong, and the suite already drew the distinction for §7.6.4.1's password —
`--password-prompt` is described there as "interactive, the default when a document refuses and
stdin is a tty".

So `ask` is a level here now. `pdf_transform`'s `ask_before_the_operation` runs before anything is
written: where standard input is a terminal it consults every document the plan reads, puts
`Consulted::question` on **stderr** (this program's rule: stdout carries bytes), reads a line, and a
`yes` lowers the run to `Level::Off`. Where standard input is not a terminal the level is left
alone and `apply` answers it with `Refusal::Unanswered` — the honest degradation, and the same one
every non-interactive caller gets.

**A `no` is its own refusal.** `Refusal::Declined` was added rather than reusing
`Refusal::Restricted`, whose sentence ends "and --restrictions is on" — a statement about a level
nobody chose. `Restricted` is this program obeying the document; `Declined` is a reader deciding;
`Unanswered` is nobody having been asked. Three different events, three sentences, one exit status
(RFC 0002 §4.4's 4, refused).

**Driven by hand, because a gate cannot allocate a terminal.** Under a pseudo-terminal, over
`bug1815476.pdf` (`/P −1084`, Table 22 bit 11 clear), `pdf-transform split --restrictions=ask`:

```
This document restricts assembling a document out of these pages: Table 22 bit 11 is clear. Do it anyway? [y/N] n
error: this document restricts assembling a document out of these pages: Table 22 bit 11 is clear, and the question was answered no
```

exit 4, nothing written; and with `y`, exit 3 (warnings) and `page-1.pdf` on disk. The transcript is
in `doc/history/916-*.md`.

### 3. The file system — the face that cannot ask, and says so

`pdf-fuse` is unchanged and deliberately so. A mount has no dialogue: RFC 0003 §5.3 already records
that FUSE "returns … `EPERM` with no message channel". It does not call `Vfs::consult` at all, and
an operation under `Level::Ask` is `EACCES` with `WorkerError::Unanswerable`'s sentence, logged in
full to the mount's own stderr as well as returned as a number.

**What changed is the sentence.** It read "--restrictions=ask was given and this program cannot ask"
— a command-line flag, named at four hosts of which one has a command line. It now says the
restriction level is ask and there was nobody to ask.

**A face that never consults is not broken**, and that is a property of the construction rather than
an indulgence: the operation refuses exactly as it did before ADR 0874. Not proceeding is what a
closed dialogue means everywhere else in this tree.

### 4. The viewer — the channel exists, the windows still have no dialogue

`viewer-core` has had the whole round trip since session 885: `Event::Asking` out,
`Command::Answer { document, proceed }` back, the edit resolved before it is held, one question
outstanding per document. It crosses `viewer-confined`'s wire and the C ABI (`pdfv_answer`,
`PDFV_EVENT_KIND_ASKING`), **so a C host of the viewer can already ask and answer**. Nothing about
that needed this round, and nothing in it changed.

What the three windows do is still `viewer_host::unanswerable` and `proceed: false`, out loud. This
round did not build a dialogue in `viewer-gtk`, `viewer-qt` or `viewer-ui`, and the reason is not
that it could not be done — `viewer-gtk`'s password prompt is a modal `gtk4::Window` a yes/no could
be modelled on in an afternoon. It is that `doc/todo/38` records the project owner's instruction
that no user interface is to be built until it is asked for, and a modal question in three windows
is three pieces of user interface. **The distinction this round did act on is between a face whose
*channel* does not exist and a face whose *widget* does not exist**: the first was a design defect
and is fixed; the second is a piece of work waiting on a word.

## Consequences

| face | can it ask | how | where its level comes from |
|---|---|---|---|
| KIO | **yes** | `WorkerBase::messageBox`, `QuestionTwoActions` | `PDF_KIO_RESTRICTIONS`, default `off` |
| `pdf-transform` | **yes, on a terminal** | a line on stderr and a line read back | `--restrictions=off\|on\|ask\|warn` |
| a C host of `viewer-ffi` | **yes, since 885** | `PDFV_EVENT_KIND_ASKING` and `pdfv_answer` | `pdfv_restrict` |
| `pdf-fuse` | **no** | — | `Config::policy`, default `off` |
| the three windows | **not yet** | `viewer_host::unanswerable`, `proceed: false` | `viewer_host::IGNORE_RESTRICTIONS` |

- **The default did not move anywhere.** Every face still opens at `off`, which is what `CLAUDE.md`
  requires and what makes the whole feature invisible to a reader who never asks for it.
- **`doc/todo/38`'s "what is left" list is shorter by its first entry's harder half** — a dialogue to
  answer with exists in two faces — and unchanged in the other: a way for a person to *choose* a
  level is still owed everywhere but the command line, and `PDF_KIO_RESTRICTIONS` is a placeholder
  rather than an interface.
- **One finding this round did not chase**: rendering a page of `bug1815476.pdf` inside the confined
  generator kills the worker with `SIGSYS`, while the same document's page extraction, listing and
  attachment questions all answer. Encryption is the only thing that document has that the
  committed documents in `tests/confined.rs` do not. It is recorded in `doc/todo/58` §4 as owed
  work rather than folded into this round.
