# 481 — The mode a clause conditions its states on

**Finding.** §12.4.4.2 is a state machine written in `shall`s — "[a]n interactive PDF processor
shall maintain a current navigation node", `/PresSteps` making the primary node current on arrival,
`/NA` then `/Next` forward, `/PA` then `/Prev` backward — and nothing walked it, because the one
thing it is conditioned on did not exist here: NOTE 3 respects the nodes "only when in presentation
mode", and ADR 0135 had decided this crate keeps no such state and deduces it from `Command::Tick`.
**That deduction is wrong for exactly the case the clause is about**: a person stepping through a
slide show with an arrow key drives no clock, so it answers *no* while a presentation is running.
`Command::Present(PresentationMode)` is the amendment, and with it the states are walked, Table
165's `/Dur` is read — a `shall` nothing had ever read — and a defect fell out on the way: a page
turned **by hand** during a presentation played no `/Trans` at all, where §12.4.4.1's own words
("when moving … during a presentation") and §12.4.4.2's step (c) both say it must.

**Date.** 2026-08-13.
**ADR.** [0316](../adr/0316-the-mode-a-clause-conditions-its-states-on.md).
**Touched.** `crates/pdf-model/src/navigation.rs` (`Node::duration`, its read in `steps`, one test,
the module's expired paragraph), `crates/pdf-model/src/view.rs`
(`ViewState::optional_content_snapshot` and `restore_optional_content`, NOTE 2's two halves),
`crates/viewer-core/src/presentation.rs` (**new** — the clause's own state machine),
`crates/viewer-core/src/command.rs` (`Command::Present`, `PresentationMode`),
`crates/viewer-core/src/viewer.rs` (`Turn`, `go_to`, `arrive`, `play_transition`, `present`, the
node clock in `tick`, the arrival rule in `apply`), `crates/viewer-core/src/open.rs` (three fields),
`crates/viewer-core/src/interact.rs` (`navigate`), `crates/viewer-core/src/lib.rs`,
`crates/viewer-core/tests/sub_page_navigation.rs` (**new**, seven tests),
`crates/viewer-confined/src/protocol.rs` (the mode on the pipe, and its round trip),
`crates/viewer-ui/src/bin/pdf-viewer.rs` (`p` sends the mode; the trace line),
`crates/pdf-model/examples/presentation_fixture.rs` (a fourth slide, with states),
`doc/conformance/ledger.toml` (§12.4.4, §12.4.4.1, §12.4.4.2), `doc/todo/32-presentation-player.md`,
`doc/ui-boundary.md`, `doc/running-the-viewer.md`, `doc/HANDOVER.md`, `doc/adr/0316-*`, this file.

## The boundary message, and the argument for it

One `Command` variant, no `Event`, no `Query`. `doc/ui-boundary.md`'s rule is that a message carry a
question a host cannot answer for itself; this is that rule in the other direction — a *statement*
only a host can make, like `Restrict` and `Delegate`, because full screen is chrome and chrome is
the host's. The forward and backward requests are the `GoTo(Next)` and `GoTo(Previous)` the
vocabulary already had, because "the user requests to navigate forward (such as an arrow key press)"
is what that message already means, and the clause's own random access — "such as by clicking on a
link" — is the page change `interact::apply` already performs.

**Two consumers failed to compile**, which is the mechanism working: `viewer-confined`'s wire
protocol and `viewer-ui`'s trace line are the two that match `Command` exhaustively. The C ABI did
not and did not need to — commands there are functions, and `PDFV_EVENT_KIND_COUNT` is unmoved.

## The population, counted first

`cargo run --release -p pdf-model --example presentation_census` over every document this tree
opens: **985 documents, 1978 pages, 0 stating `/Trans`, 0 stating `/Dur`, 0 stating `/PresSteps`.**
A trap-8 round, said out loud: the corpus cannot exercise one line of this. Every witness is
hand-built, and the pair `tests/sub_page_navigation.rs` writes differs in the single entry
`/PresSteps`, so the test is about the entry rather than about a page turn.

## Two silences that are choices, and are labelled as such

- Running off either end of the node list leaves **no current node**, so the next request turns the
  page. The clause says `/Next` "(if present)" becomes current and is silent about a node with none;
  the alternative re-executes the last node's `/NA` for ever and no page could be turned.
- Entering the mode on a page nobody navigated to makes that page's primary node current — the
  clause names an *arrival* as what sets the node, and a processor with none would not be
  maintaining one.

## Where the reading came from

`doc/md/` **and** the standard's own PDF, because Table 165's first row is garbled in the
conversion (the `Type`/`name` cells are transposed). `pdftotext -layout` confirms the conversion
drops nothing else here, which is the caveat `doc/HANDOVER.md` states about that directory met once
more and answered in the safe direction.
