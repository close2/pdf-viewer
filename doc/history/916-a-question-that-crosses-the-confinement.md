# 916 — A question that crosses the confinement, and two faces that ask it

2026-09-04. Argued in [ADR 0874](../adr/0874-the-ask-level-crosses-the-confinement-as-a-question-and-an-answer.md)
and [ADR 0875](../adr/0875-four-hosts-and-what-each-does-with-a-question.md). On round 913's
branch, because it finishes what that round found. The standing instruction it exists to satisfy is
`CLAUDE.md` principle 3's: the four levels are `off`, `on`, *ask before the operation* and *warn
before the operation*, the policy is asked once in a place a host can supply, and **"a refusal that
cannot become an 'ask' is the thing to avoid"**.

Touched: `crates/pdf-transform/src/lib.rs` (`consult`, `Consulted`, `Refusal::Declined`) and
`src/bin/pdf-transform.rs`; `crates/pdf-vfs/src/{worker,wire,confined,layout,lib}.rs`;
`crates/pdf-vfs-ffi/src/{tree,abi}.rs`, `include/pdf_vfs.h`, `c/browse_a_document.c` and three
tests; `kio/src/pdfworker.{h,cpp}`; `crates/pdf-transform/tests/verbs.rs`,
`crates/pdf-vfs/tests/{a_write,confined}.rs`; `doc/conformance/ledger.toml` (§7.6.4.2, §12.8.2.2),
`doc/todo/38`, `doc/todo/58`, `doc/state-of-play.md`; two ADRs, this file.

`pdf-model` did not change. That is worth a line, because it is the property the round was asked
to keep: `restriction::decide` is still the one place the policy is asked, and what this round
added is a way to ask it *before* committing to the operation rather than a second copy of it.

## 1. What round 913 found, and what was wrong in `doc/todo/38`

`doc/todo/38` said nothing in the core had to change for *ask*. Round 913 disproved it by building
the KIO face. RFC 0003 §6 puts every byte of parsing in a confined process; the restriction
decision is taken inside it — `pdf_transform::apply` asks `decide` at the seam and answers
`Level::Ask` with `Refusal::Unanswered`, "a pipe has nobody to put the question to" — and a
confined process has no channel to a person **by construction**. So *ask* degraded to a refusal in
every face, including the one face with a real question channel sitting unused
(`KIO::WorkerBase::messageBox`). Exactly the shape principle 3 says to avoid.

## 2. Which construction, and why the earlier recommendation stood

ADR 0869 §3 costed two ways out — make the wire a dialogue, or ask first in two round trips — and
recommended the second. **This round implemented the recommendation rather than overturning it**,
and the argument for keeping it is in ADR 0874 §Decision. In short: a dialogue on the wire makes
every worker re-entrant, including `pdf-view-worker`, which has no restriction question at all;
whereas what must not be duplicated is the *policy*, and asking first does not duplicate it. What
crosses is a question and an answer.

- `pdf_transform::consult(level, document, operation) -> Consulted` is the question, and
  `apply_borrowed` now asks it too — so a host that asks and then acts gets **one** reading rather
  than two that could disagree. `Consulted::question` words the sentence once, and answers `None`
  for the three verdicts that are statements rather than questions.
- Across `pdf-vfs`'s confinement: `Query::Consult { operation }` out, `Answer::Consulted` back, and
  then `Query::Consented(Box<Query>)`, which runs the inner query at `Level::Off`. **The wrapper
  carries the answer, never a second copy of the rule.** A `Consented` inside a `Consented` is
  refused by the decoder — a nesting depth a peer chooses is a stack a peer chooses — and
  `wire::encode_consented` writes the tag without owning the query, so consenting to an insertion
  does not cost a copy of the document being inserted.
- `Vfs::consult(path, verb)` and `Vfs::answer(proceed)`. The consent is held **beside the worker
  for that generation**, in a `Consenting` wrapper, and spent inside `Worker::ask`. Two things fall
  out of that placement: the answer dies with the generation, so a file somebody else edited
  underneath the mount cannot be acted on with a stale yes; and none of the fifteen call sites can
  forget to spend it. The first construction tried threaded a `consent` parameter to each of the
  fifteen and was thrown away for exactly that reason.
- Which operation a path's verb performs is `layout::Write::operation` and
  `layout::Generator::operation`, beside the table that already says what a write to each row
  means — and held to `Plan::operation`'s own answer by a test that walks four paths at `Level::On`
  and requires the consultation to refuse exactly where the operation refuses. Two mappings that
  must agree and are only *said* to agree is how they stop agreeing.

## 3. Which hosts ask, and what each does

| face | asks | how | its level from |
|---|---|---|---|
| KIO | **yes** | `messageBox(QuestionTwoActions, …)`; a decline is `ERR_USER_CANCELED` | `PDF_KIO_RESTRICTIONS` |
| `pdf-transform` | **yes, on a terminal** | the question on stderr, a line read back | `--restrictions` |
| a C host of `viewer-ffi` | **yes, since 885** | `PDFV_EVENT_KIND_ASKING`, `pdfv_answer` | `pdfv_restrict` |
| `pdf-fuse` | **no** | `EACCES`, the whole sentence in the log | `Config::policy` |
| the three windows | **not yet** | `viewer_host::unanswerable`, `proceed: false` | `IGNORE_RESTRICTIONS` |

**The default is `off` everywhere and did not move.** `--restrictions=ask` stopped being a usage
error; a non-interactive run still degrades to `Refusal::Unanswered`, which is the honest answer
and not a silent proceed. `Refusal::Declined` is new beside it, because "this program is obeying
the document", "a reader decided" and "nobody was asked" are three events and were two — the old
`no` would have ended "and --restrictions is on", a statement about a level nobody chose. And
`Unanswered`'s own sentence stopped naming a command-line flag at four hosts of which one has a
command line.

**The three windows are the one entry in that table this round chose not to move**, and the
distinction it acted on is between a face whose *channel* does not exist — a design defect, fixed —
and a face whose *widget* does not exist. `doc/todo/38` carries the owner's instruction that no
user interface is built until it is asked for, and a modal question in three windows is three
pieces of user interface. `viewer-gtk`'s password prompt is the shape one would copy.

## 4. End to end, twice by gate and once by hand

- **`pdf-vfs`, in process and confined.** `a_write.rs::a_question_crosses_the_confinement_and_both_answers_are_obeyed`
  and `confined.rs::a_question_and_a_consent_cross_the_confinement`, over `bug1815476.pdf`
  (`/P −1084`, so §7.6.4.2's Table 22 bits 4, 5 and 11 are all clear). The question names the
  operation and the bit; **a `no` refuses by name and leaves the file byte for byte what it was**;
  a `yes` performs it; the consent is spent once and is not a blanket over other operations. The
  confined half puts all of that across `confined-transport`'s frames.
- **From C, through the confined worker.** `c/browse_a_document.c` gained `restricting()`, which is
  what a face does: consult, decide, answer, perform.

  ```
  consulted: verdict 2, question 'This document restricts assembling a document out of these
    pages: Table 22 bit 11 is clear. Do it anyway?'
  after a no: EACCES — this document restricts assembling a document out of these pages: Table 22
    bit 11 is clear; the restriction level is ask and there was nobody to ask, so it was not done
  after a yes: 139455 byte(s) of page
  ```

- **The command line, by hand under a pseudo-terminal**, because a gate cannot allocate one:

  ```
  $ pdf-transform split bug1815476.pdf --restrictions=ask -o 'page-%d.pdf'
  This document restricts assembling a document out of these pages: Table 22 bit 11 is clear. Do it anyway? [y/N] n
  error: this document restricts assembling a document out of these pages: Table 22 bit 11 is clear, and the question was answered no
  ```

  exit 4, nothing written; answering `y` exits 3 (the split's own warnings about an encrypted
  source) and writes `page-1.pdf`.
- **KIO.** The harness drives the round trip's plumbing — every verb consults first — and the
  level's channel twice: a level the plugin knows changes nothing about a document that restricts
  nothing, and a word it does not know is refused by name. **The dialogue itself is driven by
  nothing**, and cannot be here: `messageBox` needs a client with a UI delegate and
  `drive_the_worker.cpp` is a `QCoreApplication` with no session. That is ADR 0869's own honest
  limit, unchanged, and it is in `doc/todo/58` beside Dolphin's entry and look.

## 5. What the gates found

Four core lines and the two `fuzz/` lines green; `cargo nextest run --workspace` 3294 passed.
Three lint findings on the new code, all fixed rather than allowed: two match arms with identical
bodies in `Query::operation` (merged, with Table 22 bit 11's sentence saying why the three verbs
are one operation), a `..self.policy` struct update with no effect (written out, so a second field
added to `Policy` fails to compile rather than being silently dropped for a consented operation),
and three functions over the hundred-line lint, split. The corpus gates' figures are in the commit
message.

## 6. One finding this round did not chase

**A page of an encrypted document kills the confined worker with `SIGSYS` when it is rendered.**
`bug1815476.pdf` answers `Consult`, `ExtractPage`, `ExtractImages` and the attachment questions
through the confinement and dies on `RenderPage`; every committed document in `tests/confined.rs`
renders. Encryption is the difference, which makes it the same shape as round 911's `openat`
finding — a library sizing something from a file the filter forbids — rather than anything about
this round's work. It is in `doc/todo/58` §4 with `strace -ff` named as what settles it. The test
that met it was rewritten to prove the same property a different way rather than to work around it.

## 7. What principle 3 still does not have

- **A way for a person to choose a level.** `--restrictions` is one; `PDF_KIO_RESTRICTIONS` is a
  placeholder for a configuration page and says so; the three windows have `IGNORE_RESTRICTIONS`
  and nothing else. The per-document override the viewer-wide value cannot express is unbuilt.
- **A dialogue in the three windows**, held by the owner's instruction rather than by a design.
- **Table 22 bit 5's copy**, which nothing here can name: what a window hands a host is the same
  readback a drag asks sixty times a second, and the bit carves itself for §14.9's tree.
  `doc/todo/38` has it unchanged.
- **Bits 11 and 12's remaining operations**, and Annex O's `ef` prompt, likewise unchanged.
