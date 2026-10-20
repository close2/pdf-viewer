# 2076 — The confined window sends a form, and the server's answer crosses to its worker

Session 1620, slot 6 of batch ninety-seven. Status: **accepted** and **built** in `quorra-confined`.
Context: ADRs 1062, 1291, 1327, 1527, 1738, 2040; `doc/todo/30`'s list of what each future feature
brings to this window. Code: `quorra-confined/sending.rs` (new), `quorra-confined/asking.rs`'s two new
questions, the `Submit` and `NeedsFile` arms of `Host::event`, and `viewer-confined`'s
`command_kind::RESPOND`. Drive: steps `92-submit-*` and `93-fetch-*`, in all four windows. Cites
§12.7.6.2 and §12.7.8 for the acts, and moves no ledger row: the policy and the transmission are
`viewer_host`'s and were reviewed with it.

## 1. What the brief supposed, and what held

Round 1602 kept `--submissions=` pinned at `refuse` here for want of a sender on this side. **The card
and the policy carry the ask, and held whole**: `viewer_host::may_submit` answers `Sending::Ask` with a
`Question`, which is what `QuestionCard` takes for a link (ADR 2040). **The transmission is one call,
and held**: `viewer_host::submit::Submitter::send` is the thread every window puts
`viewer_host::submit::transmit` on, and this window polls its `interval` as the toolkits do, having no
event-loop proxy to be woken by. **"The worker's answer crosses as `Command::Respond` does" did not
hold**: `Respond` did not cross at all — the wire named it `Uncarried` because this window sent no form.
A command discriminant (43) is added and the greeting moves to `PDFVCF18`, for `MAGIC`'s own rule: an
older worker cannot answer the new command. And **the drive had no submission steps** in any window:
it drove Annex O's fetched `fdf` (step 31) and a signature policy's copy (ADR 1738), never a
submit-form. Steps 92 and 93 are new, with a server of their own, and judge the four windows alike.

## 2. Decision

1. **The word is taken at the other windows' default**, `ask` (the owner's, `doc/questions/Q98`), and the four
   levels are `may_submit`'s answers unchanged; `ask` puts the question on the card, answered by Enter
   and Escape, one question at a time (ADR 2040).
2. **The request is composed in the worker and sent from this side.** The worker holds the form and
   the composition; this process holds the network the worker never acquires (`CLAUDE.md` principle 3),
   and sends through the same `transmit` as every window, with its scheme check and URL check before the
   TLS stack (ADR 1327). The binary already linked `ureq` through `viewer-host`; the worker's system-call
   set and its filter are untouched.
3. **The answer crosses as `Command::Respond`**, and the worker imports it under its limits: this side
   reads the answer's status and media type (`submit::reply`) and never parses the body, exactly as it
   reads and never parses a file a document named (ADR 2040).
4. **An import named by an absolute URI is fetched under the same level**, as in the other windows
   (ADR 1527): it is the same act class, and once this window has the sender, refusing it at every
   level would put one act at two levels in one window. Its answer crosses the same way.
5. **A PDF answer is kept and not opened.** The other windows open it beside the form; this window
   holds one document, and opening the answer in its place would put away the form the person just
   filled in, unsaved, for a document they did not ask for. `submit::keep_answer` keeps it for every
   window, and the sentence says where.
6. **A signature policy's copy is still declined at every level**, for the reason that is left: what a
   fetched copy is for is a document opened beside the one that named it (ADR 1738).

## 3. What it costs, and what is left

- **An answer that arrives after the worker died is handed to the next one**, which reopened the
  document from its file: what the person typed and did not save is gone, and the answer is imported
  into the form as reopened. It is the residue ADR 2040 names for a question standing through a death.
- **A host and a worker from different builds now refuse each other at the greeting**, which is the
  greeting's whole purpose; they are built and shipped together.
- What this window still does not serve is a second document beside the one it holds — the policy's
  bound copy and a PDF answer — said by name both times.
