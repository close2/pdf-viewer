# 1190 — The fourth window is not a fourth toolkit, and a control is owed by the operation rather than by the window

Session 1176. Status: **accepted**.
Context: `crates/viewer-ui/src/bin/quorra-confined.rs` and its two sibling modules,
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-host/src/policy.rs`,
`crates/viewer-host/src/restriction.rs`, `doc/todo/30`, `doc/state-of-play.md`.
Amends: ADR 0713 Decision 2 — the ground it gave, not the conclusion it reached.
Builds: ADR 0718 (the password prompt), ADR 0725 (the device), ADR 0737 (`Query::View`),
ADR 0814 (an unanswerable question is refused, not granted), ADR 1144 and ADR 1145 (the four
levels and the menu that sets them), ADR 1155 (links, pinned here to `refuse`).
Clauses: ISO 32000-2 §7.6.4.1, §12.6.4.8, §12.7.6.2; `CLAUDE.md` principle 3.

## 1. The premise expired and the conclusion did not

ADR 0713 Decision 2 declined to extend `doc/todo/30`'s level-hosts rule to `quorra-confined` on a
statement of fact: "This window adds no message and no chrome; what it carries is the *other*
boundary." The first half of that sentence is no longer true, and three ADRs made it untrue:

- **Chrome.** ADR 0718 put §7.6.4.1's `PasswordCard` in the window. Decision 2's own scope
  sentence had said "no password prompt", and 0718 retired it.
- **A message.** ADR 0737 added `Query::View` and `Command::View` to the boundary **for this
  window alone** — its own "What this does not close" says the three established windows do not use
  them at all. Decision 2's sentence about exactly this hazard is "a feature living in one host is a
  message nobody has tested".
- **A device.** ADR 0725 gave it `render-quorra`, a render thread, a `--cpu` fallback and a
  device-refusal path of its own.

Each was argued on its merits and none re-read the rule it was an exception to. An exemption
written on *smallness* has to be re-argued every time the window grows, which is a licence to
accumulate exceptions rather than a decision. Decision 2's conclusion stands; its ground is
replaced here.

## 2. The ground: levelness is parity between interchangeable hosts

`doc/todo/30`'s rule says what it is for: "this file's proudest claim is that **seven consumers have
never asked for a new message**, and that claim is only evidence while the consumers are actually
made to carry what is added. A feature living in one host is a message nobody has tested."

So levelness is an *instrument*, and what it measures is whether the boundary is complete: three
different toolkits are made to carry every feature, and if the boundary were wrong one of them would
need a new message. That instrument needs its three subjects to be **interchangeable** — three ways
of drawing the same host — and `quorra-confined` is not a fourth one of those. It is the only host
that holds no `Viewer`, no document bytes and no parser: the viewer lives in another process, and
this window is what is left outside it.

Two consequences follow, and they are the ground:

- **A feature that lands in `quorra` and not here costs the levelness claim nothing**, because this
  window was never one of the three the claim is made about. Its absence measures nothing.
- **A message that lands here and nowhere else is the boundary being tested, not the rule being
  broken.** `Query::View` exists because this host *can lose the viewer* and the others cannot;
  that is a property of the confinement, and finding it is what a fourth, differently-shaped
  consumer is for.

This ground survives the window's growth because it does not rest on the window's size. It would
expire on one event, which ADR 0713 already named and which this ADR keeps: the moment an
established window moves onto the confined boundary, the two stop being differently shaped and this
binary's reason to exist starts to expire.

## 3. What it owes is decided by the operation, not by the window

Exemption from levelness is not exemption from `CLAUDE.md` principle 3, which says a document's
restrictions "shall always be possible to turn off", gives them four levels, and says a refusal that
cannot become an *ask* is the thing to avoid. The rule this ADR sets for the fourth window is:

> **A host owes the control for an operation exactly when it can perform that operation.**

Measured against it, today's window owes nothing, and the three gaps a reader would notice are each
answered by the rule rather than excused:

- **No `--restrictions=`, no restriction menu.** The window performs none of the operations a
  restriction decides: no selection and no copy, no annotation, no form fill, no print, no save, no
  extract. A menu of levels for six operations none of which it can perform would decide nothing.
  It runs at `RestrictionLevel`'s default, `Off`, which is also the reader-favouring end, so nothing
  is imposed on the reader either.
- **`Event::Asking` answered `proceed: false`.** ADR 0814's decision, and it is the conservative
  direction: a window that cannot put the question says out loud that nobody could answer rather
  than letting *ask* behave like *on*. It should never arrive while the bullet above holds.
- **`--links=` absent, pinned to `Links::Refuse`.** ADR 1155's reason — no dialogue, so the question
  cannot be put — and the window has no click handling, so a link is never followed from here in any
  case. `refuse` is the safe end of that scale, not the restrictive end of a document's.

And the rule says what arrives with each future feature, in the same round as the feature rather
than as a later argument: selection or copy brings `--restrictions=` and the ask dialogue (the modal
machinery is already there, lazily loaded, since ADR 0718); click handling brings `--links=` with a
real `Ask` arm; anything that writes a file brings the save question. `doc/todo/30` carries that
list as what is owed *when*, not as what is missing now.

## 4. A correction: the confinement does not forbid this window a filesystem

It is easy to reason that the fourth window cannot open a link or save a file because it has no
filesystem. That is the wrong process. The **worker** has no filesystem and no network — that is the
confinement, and `crates/viewer-confined/src/lib.rs` states it. The **host** holds the filesystem by
design: `quorra-confined` opens the document and passes its descriptor across, which its own source
calls rule 2, "the filesystem is this side's, never the worker's".

So this window declines a file the document asks for, and the bytes of an extraction it never
requested, as **decisions** rather than as incapacities, and the rule in section 3 is what they
answer to. An argument that excused a missing control by the confinement would be excusing it with
somebody else's constraint.

## 5. What this does not close

The window is still the only consumer of two boundary messages, and that is a fact about `Query::View`
rather than a defect; if an established window ever needs to restore a view it did not hold, the two
stop being one host's. Nothing here builds UI: the owed pieces are listed in `doc/todo/30` and the
hosts are another round's.
