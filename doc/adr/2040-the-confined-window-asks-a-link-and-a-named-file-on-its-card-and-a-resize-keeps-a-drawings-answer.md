# 2040 — The confined window asks a link and a named file on its card, and a resize keeps a drawing's answer

Session 1602, slot 4 of batch ninety-five. Status: **accepted** and **built** in `quorra-confined`.
Context: ADRs 1145, 1155, 1190, 1227, 1263, 1332, 1345, 1987, 2026; `doc/todo/30`'s list of what each
future feature brings to this window. Code: `quorra-confined/asking.rs`, the `Opened`,
`PasswordRequired`, `OpenUri` and `NeedsFile` arms of `Host::event`, and the offer flag in
`exchange.rs`. Drive: steps `90-link-*` and `91-remote-*`, in all four windows. Cites §12.6.4.8 and
§12.6.4.3 for the acts, and moves no ledger row: the policy is `viewer_host`'s and was reviewed with it.

## 1. What the brief supposed, and what held

ADR 1987 kept `--links=` and `--remote-documents=` pinned to `refuse` in this window "for want of a
dialogue to put the *ask* level's question in". **The password card's shape carries the ask, and
held whole**: `viewer_ui::chrome::QuestionCard` is already that shape with the entry taken out (ADR
1145), drawn by the flagship and answered by Enter and Escape, so this window takes it unchanged and
draws it where it draws the password card — over the device's frame and over the processor's.
**The hypothesis that the levels' plumbing already crosses the wire held for another reason than the
one given**: ADR 1961's operation code is a *document's* restriction, and neither of these levels is
one. Both are this side's decisions about this machine — which program a URI is handed to, which
file is read — so nothing about them crosses. What crosses is what crossed already: `Event::OpenUri`
and `Event::NeedsFile` out, and `Command::Supply` back, whose bytes the wire has carried since it had
the command. The worker parses the named file under its own limits, as it parses the first. No frame
was added, so the greeting does not change.

The brief also supposed "the drive's link steps". **There were none**, in any window: the drive
followed a link inside the document (step 12) and never one to another program or another file. The
steps are new, and judge the four windows alike.

## 2. Decision

1. **Both words are taken, at the other windows' default**, `ask` (ADRs 1155, 1227), and the four
   levels are `viewer_host::link`'s and `viewer_host::remote`'s answers unchanged.
2. **A named file opens in place, and the question says so**: this window holds one document and
   reserves no name for a second, so `beside` is asked as `false` and the worker's viewer says
   itself that a `/NewWindow true` replaced what was open (ADR 1263).
3. **A file supplied is the window's document once the worker says it opened** — the title, the
   save, the next relative link and a resume all follow it — and the prompt's file where the worker
   asks for its password, whose cancel declines it to the worker (ADR 1345). `viewer_host::Arrivals`'s
   offer, for one document: it lives for the one command that carries it, a supply with bytes or the
   prompt's next attempt, and is spent once that command's events are seen (ADR 1332).
4. **One question stands at a time**; a second is declined out loud, because a person answering a
   card is answering the words they read.
5. **The two other pins stay**, and their comments now give the reason that is left: a form needs a
   sender and `--submissions=` (ADRs 1291, 1527), and an embedded document a second view (ADR 1331).

## 3. A resize that changes no target keeps the drawing's answer

Round 1595 left this open (ADR 2026 §4). **Kept, as the toolkits keep it, and for their reason.**
`viewer-core` asks for a page again only when its target or its interpretation changed (the
`showing || asked` test in `viewer/layout.rs`), so a resize at one magnification issues no request and
`viewer_host::Oversized::asked` is not asked again; this screen's `drawn_as_windows` asks once for a
drawing, which is the same rule. The answer prices a cost and not a picture — a page drawn whole and
one drawn as its windows compose the same pixels (ADR 2026 §3) — and asking at every resize would
put a stride of the list on the window's thread at every step of a dragged edge.
