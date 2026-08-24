# 721 — The way out two windows named and did not have, and the count that was a suffix

Date: 2026-08-25. ADRs [0603](../adr/0603-the-count-that-was-a-suffix-and-the-reading-beside-it.md),
[0604](../adr/0604-the-way-out-two-windows-named-and-did-not-have.md).

Touched: `tools/state.sh`; `crates/viewer-host/src/{policy.rs,lib.rs}`;
`crates/viewer-host/tests/host_mappings.rs`; `crates/viewer-gtk/src/{host.rs,bin/pdf-viewer-gtk.rs}`;
`crates/viewer-qt/src/{host.rs,bin/pdf-viewer-qt.rs}`; `crates/viewer-qt/cpp/window.cpp`;
`crates/viewer-ui/src/bin/pdf-viewer/{arguments.rs,files.rs}`;
`doc/conformance/ledger.toml` (§7.6.4.2, §12.8.2.2); `doc/todo/30-a-native-host.md`,
`doc/todo/38-a-documents-restrictions-have-levels.md`, `doc/state-of-play.md`,
`doc/traps/instruments-and-reports.md`.

The **eighth** round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*. ADR 0509's ordering was spent in
the seven-hundred-and-seventeenth and `doc/todo/30` named two items in its place; this round took
the second — **sort `tools/state.sh windows`' unreached list into debts and non-debts, and record the
reading beside the count**.

`viewer-core` was not touched. No message was added and no variant changed shape — the thirteenth
consecutive round since the six-hundred-and-seventh in which that has been true.

## Why this item and not the other

`AccessibilityNode::lines` across the C ABI is two accessors and no decision, and it would have been
a good afternoon. The sort ranks above it on ADR 0509's own third criterion — what makes the
level-hosts decision *checkable* — and on a stronger argument the briefing named: an uninterpreted
count is what lets a parity claim decay quietly, and this one had been read by two rounds as "eleven
queries" and left. The choice paid: the list could not be sorted as it stood, and the sorting found
a debt that outranks both items on criterion 1.

## The list could not be sorted, because two of its entries were the instrument's

`names_in_code` grepped `Command::[A-Za-z]+` with **no word boundary**, so it matched the tail of
`PathCommand::Close` — `pdf_render`'s *path* close, which `viewer-ui` writes on every rounded
rectangle of its own chrome. The question *"does this window ever close a document?"* was answered by
a piece of chrome geometry.

And the population included `viewer-ui`'s **trace formatter**, which matches `Command` exhaustively
in order to print a command's name. `section_hosts`' own comment gives that as its reason for asking
`viewer-ffi` alone — *"`viewer-ui` names all of them in `trace.rs` … trap 11's shape, a count whose
condition is not the question"* — and `section_windows` was written sixty lines below it over that
crate anyway, citing the same comment for excluding `viewer-confined`. **The condition was documented
and not applied, in one file, by one round.**

So the section reported `viewer-ui reaches 25 of 25` and `every Command reaches at least one window`.
Both false: no window reaches `Command::Close` or `Command::Focus`, and `viewer-ui` reaches 22 of 25.
Trap 11 has a seventh and an eighth instance, with the rule that generalises past greps: a count you
wrote is a population you have not audited, and the audit is *list everything that satisfies the
condition and does not satisfy the question*.

## The reading, and where it lives

Fifteen rows in `tools/state.sh`, one per unreached variant, each saying **debt** or **not a debt**
with the reason — a reading rather than a count, which is what `CLAUDE.md` permits to be written
down, and it lives beside the number because that is the only place it will be read at the same
moment. The non-debts turned out to be four kinds rather than ADR 0577's two: learned another way
(`Dirty`), the tier (`Frame`), a **delegation** (`Caret`, `Offset`, `FieldSelection`, `FieldAt` —
four of the eleven, not "most"), and a message whose precondition no window meets (`Close`, `Focus`,
which address a second open document).

Five debts are left and they rank: §14.7's tree with §9.10.2's readback beside it, §12.5.6.14's
popups, §12.3.5's collection, §12.5.6.5's link cursor, and §12.5.6.6's free text — which is a debt
two hosts **refuse by name**, and the verdict column says so, because a table calling that the same
thing as one nobody has noticed would lose the distinction this project spends its care on.

It is checked in both directions: `UNREAD` for a gap with no reason, `SPENT` for a reason whose
variant every window now reaches. Both were run against an injected defect (trap 13), and the second
had a customer in this round.

## What the reading found, on the row nobody had been asked to read

`Command::Restrict` reached one window of three. `CLAUDE.md` says of a document's restrictions that
**"it shall always be possible to turn them off"** — and in `pdf-viewer-gtk` and `pdf-viewer-qt` it
was not possible at all, because neither sent the command from anywhere.

Both, meanwhile, answered every refusal with a sentence naming `--ignore-restrictions`, each having
written it for itself, and both argument parsers answered that same word with *"--ignore-restrictions
is not an option this program has"* and exit 1. **Each told a person the way out and then refused the
way out.** It is invisible from inside either host: the sentence is right about what it means, the
parser is right about words it does not know, and nothing read both.

Closed here. `viewer_host::IGNORE_RESTRICTIONS` is the word, `viewer_host::refused` the sentence that
interpolates it, and they are in one module because apart is exactly how they came to disagree.
`viewer-ui` adopted both and lost its third copy.

**Three tests, because neither end alone would have caught it** — the sentences agreed with each
other and with nothing else, and so did the parsers: `viewer-host` holds the sentence to the
constant, each native binary holds its **parser** to it, and `viewer-qt` walks the **whole chain**
headlessly on `issue17215.pdf`, one of ADR 0212's seven corpus witnesses, pressing `a` then `h` and
reading the window's own status line back. That last is possible only because C++ owns the `Host` as
a plain struct, which is ADR 0246's ownership inversion paying a dividend. All three were run against
injected defects.

Under `Xvfb` on the same document, with §5's binaries: obeying, the status bar carries the refusal
and the title has no dirty mark; with the flag, nothing is refused and the title gains its `•`.

## And the screenshot found the same defect one layer out

The GTK status label is `EllipsizeMode::End`, so the longest sentence this window says loses its
tail — and the tail of the longest sentence is the way out: the screen read *"… — it was not done —
this reader i…"*. Qt's `QLabel` in a status bar is clipped rather than elided, identically. Both set
a tooltip with the whole text now. Trap 1's rule applied to a window rather than a page: the count
and three tests all said the fix had landed, and the photograph said the sentence did not fit.

**The tooltip itself is not photographed and is not claimed to be.** There is no window manager
here, and a GTK tooltip wants a crossing event and a timer this environment cannot deliver
convincingly; what was checked is the call and the text handed to it.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctests, `check` over the fuzz targets, and `cargo test -p conformance` — the core plus the
conformance gate, which is `doc/todo/02` §2's row for a change to three host crates, a tools script
and the ledger. §5's binaries were rebuilt and installed before the measurement.

Nothing here can change a pixel a corpus gate rasterises: no `pdf-*` crate, no rasteriser, and
`viewer-core` untouched.

## What is left, named rather than left silent

`AccessibilityNode::lines` still does not cross the C ABI. The five debts above are in
`tools/state.sh windows`' own output with their reasons, which is where a next UI round chooses from
— and ADR 0603 §5 declines to write a second numbered list in `doc/todo/30`, because what the first
one bought was three rounds not spending themselves on a survey each, and the survey is now a command
that reruns itself.

Nothing is queued for the owner's measurement loop: every number here is `Xvfb`, a corpus document
and a status label.
