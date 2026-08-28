# ADR 0718 — The prompt the confined window owed its first verb

Status: accepted, 2026-08-28. Session 781. Amends ADR 0713's Decision 2 by argument: the sharpest
of `pdf-viewer-confined`'s refusals-by-name — an encrypted document — becomes ISO 32000-2
§7.6.4.1's prompt. Clause touched: §7.6.4.1 (its ledger row's host count is brought along; the
row stays `partial` for the reasons it already states).

## What was owed

ADR 0713 scoped the first confined host to the smallest complete set — open, arrange, turn,
scroll, zoom, report, abort — and refused everything else by name. For an encrypted document that
refusal sat in front of the scope's *first verb*: §7.6.4.1 says an interactive processor should
prompt when the default user password fails, and a window on a screen is an interactive processor
(the clause's NOTE 2, read properly in the six-hundred-and-ninety-fifth session, is about
processors with nobody to ask — not about windows launched without a terminal). So the prompt
**completes** the scope rather than extending it: without it, *open* is unreachable for a whole
population of documents, and the window's charter promised open. The level-hosts argument of ADR
0713 §2 is untouched — this adds no message and no new chrome, only wiring to chrome that already
exists.

## Decision 1: the password crosses *into* the confinement, and that is the design

The question this round was posed was whether a password should ever enter the confined process.
It should, and the boundary answered it before this round arrived: `Command::Open` has carried
`Option<Secret>` across the wire since the transport was built, with
`a_password_crosses_the_transport_unchanged` pinning byte-exactness for a non-ASCII password.
What this round adds is the argument, written where the next reader will look:

- §7.6.4's decryption happens where the document's bytes are, and the bytes are in the worker —
  that placement is the whole point of the boundary (principle 3).
- The confinement is precisely what *bounds* where the password can go next: no filesystem, no
  network, nothing but the pipe back to the host. A secret handed to a sandboxed process is
  better contained than one handed to most programs on the machine.
- The alternative — decrypt host-side so the password never crosses — would run §7.6's
  cryptography over hostile bytes in the *unconfined* process. That is the boundary defeated by
  courtesy, and it is refused.

On the host's side the password is a `viewer_core::Secret` from the card's buffer to the
command (ADR 0545's property): the trace's `brief` prints a `Secret` as no characters, and the
proof run's own log shows `Open { … }` twice with no password visible either time.

## Decision 2: the document's bytes cross again per attempt, re-read from disk

A retry is a second `Command::Open`, so the document crosses the pipe once more per attempt —
at most `viewer_host::password::ATTEMPTS` times. Two alternatives were priced and refused:

- **The worker retains the bytes and a new message supplies only the password.** Saves one
  crossing per attempt; costs a protocol message, worker-side state for a document that failed
  to open, and a divergence from what the in-process hosts do. `doc/todo/15` §5's measurement
  says what a crossing costs (the kernel's pipe is ~4 ms of a 19 MB document; the corpus's
  encrypted files are kilobytes), and that is not worth a message.
- **The host keeps the bytes in memory.** The flagship re-reads from disk instead
  (`password_answered`'s own comment: a file gone between attempts is a fact about this machine,
  said out loud), and rule 2 makes the filesystem this side's. The confined window does the
  same, for the same sentence — and holding a document the worker also holds would double the
  host-side high water for the common case of no retry at all.

The worker survives an open it could not finish (ADR 0597 made the budget refusal a message
rather than a death, and a failed authentication never left the worker at all), so the retry
goes to the **same** process: no respawn, no re-confinement.

## Decision 3: everything shared, nothing eager

The policy is `viewer_host::password`'s (`Asking`, `Ask`, `supplied` — three attempts, an empty
entry is a decline because the default user password has already been tried, `Exhausted` never
closes a window). The card is `viewer_ui::chrome::PasswordCard`, the same modal card the
flagship draws, placed over whatever the screen has — for an encrypted document that is the
surround. The fourth window's prompt therefore asks the same question in the same words as the
other three, which is what the shared module exists for.

`Chrome` — the interface's own faces — is loaded on the **first** `PasswordRequired` and never
on the launch path: a document that is not encrypted costs this window no chrome at all
(`CLAUDE.md`'s nothing-eager rule). A build whose compiled-in faces will not parse refuses by
name, still pointing at the three established windows (trap 5).

## Decision 4: Escape has two meanings, and the card decides which

While the card is shown it has the whole keyboard (the flagship's modality rule: the document
behind the card is not open, so a page key would be turning a page that does not exist). Escape
with the card up is the prompt's **decline** — it reaches
`viewer_host::password::supplied` by the same route as an empty Enter, so one place decides what
a decline means — and not `Host::abort`: "I don't know the password" and "end this worker" are
different facts about the reader. Escape with the card down remains the abort ADR 0713 built.
The proof drives both, in that order, in one sitting.

## Proof, driven under Xvfb on the release programs

`issue6010_1.pdf` (pdf.js's manifest records its password), display `:181`, 900×1100:

- Launch: worker confined in 1.4 ms, `PasswordRequired` at 0.021 s, the card presented over the
  surround at 0.043 s — *attempt 1 of 3*, the clause number in the question.
- A wrong password: `Open` re-sent (no password legible in the trace), `PasswordRequired` again,
  the card back up saying *attempt 2 of 3* with the entry cleared.
- The right password: `Opened { pages: 1 }`, `frame: 1 page(s), 1 as marks`, and the decrypted
  page — its own text says which issue it is — on the screen through the marks arm.
- Modality: `q` pressed with the card up quits nothing; the window is still there.
- The decline: Escape with the card up leaves the **worker alive** and the window open, the
  CANCELLED sentence in the title.
- The abort: Escape again — card down now — ends the worker (no zombie; ADR 0713's reaping), the
  abort sentence joins the title, and `q` then exits.

## Trap-13 calibration

Each of the four new tests in `pdf-viewer-confined.rs` watched its own defect fail before being
believed; the suite is green as committed.

| injected defect | failed |
|---|---|
| the `PasswordRequired` arm stops the document as it used to | `an_encrypted_document_is_prompted_for_not_refused` |
| `Ask::Exhausted` closes the window | `exhausted_attempts_leave_the_window_open` |
| Escape with the card up calls `abort` | `escape_declines_the_prompt_without_aborting` |
| a file missing at the retry is fatal | `a_file_gone_before_the_retry_is_said_not_fatal` |

## What this does not close

`doc/todo/15`'s remainder is unchanged in kind: the three established windows still interpret in
process, the warn-before-abort input through `viewer_host::keys` is still owed to them, and the
confined window still presents through the processor. The window's other refusals — a file the
document asks for, a URI — stay refusals, correctly: they reach outside the program, which the
prompt never does.
