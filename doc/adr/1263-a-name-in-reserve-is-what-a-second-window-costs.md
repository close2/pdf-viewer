# 1263 — A name in reserve is what a second window costs

Session 1213. Status: **accepted**.
Context: `crates/viewer-core/src/command.rs` (`Command::Beside`),
`crates/viewer-core/src/viewer.rs` (`Viewer::beside`, `Viewer::adopt`, `Viewer::apply`),
`crates/viewer-core/src/interact.rs` (`Outcome::beside`, `resume_remote`, `jump_into`),
`crates/viewer-confined/src/protocol.rs` (command kind 35),
`crates/viewer-ffi/src/{abi.rs,session.rs}` and `include/quorra.h` (`quorra_beside`),
`crates/viewer-core/tests/{remote_go_to.rs,embedded_go_to.rs}`,
`doc/conformance/ledger.toml` §12.6.4.3 and §12.6.4.4.
Builds: ADR 1227 section 5 (what a round that builds tabs would have to re-state), ADR 1190 (what
the fourth window owes), ADR 1228 and ADR 1106 (the pattern a host-supplied policy value takes),
`doc/ui-boundary.md`'s test for a message.
Clauses: ISO 32000-2 §12.6.4.3 (Table 203), §12.6.4.4 (Table 204), §12.3.2.2.

## 1. The entry was read and the decision was taken in the wrong place

ADR 1227 obeyed Table 203's `/NewWindow` by reading it and then answering the `true` case with a
sentence: *this view has one, so the remote document replaces what was open*. That was right about
the clause — not one of the entry's three sentences carries a `shall` — and it put a claim about
the **window** inside `pdf_model`'s and `viewer_core`'s reading of the **file**. The proof that it
was in the wrong place is that ADR 1227 had to write down what a later round would have to change:
"a round that builds tabs or a second `DocumentId` changes what this program's preference *is*
without re-arguing the clause."

So the reading stays where it is and the decision moves out. `interact` sets one flag —
*the action asked for a window of its own* — and `Viewer::apply` decides what that comes to,
because that is the one place this crate holds the host's answer.

## 2. `Command::Beside`, and why it is a message rather than a field on `Command::Supply`

The value is a name this host has free for a second document. It is the eleventh host-supplied
policy value and takes `Command::Trust`'s shape for `Command::Trust`'s reason: whether this program
has a second place to put a document is a fact about the *window* and about no file, and the name
it would be called by can only come from out there, because `DocumentId` is documented as the
host's word.

**The first design put it on `Command::Supply` as an `into` field**, which is the shape
`doc/ui-boundary.md` prefers — a variant's shape changed rather than a message added. It was
discarded for one reason, and the reason is the clause: §12.6.4.4's embedded go-to reaches its
target *inside the document already open*, so no file is ever asked for and no `Supply` is ever
sent. A design that cannot reach Table 204 is a design that answers the weaker of the two entries —
Table 203 states the `true` case with nothing and Table 204 states it with a `should` — and leaves
the stronger one unimplemented on a path no message passes through.

Three properties, each chosen and each testable:

- **An offer, not an instruction.** The name is used only where an action states `/NewWindow true`.
  One offered and not used opens nothing;
  `a_new_window_with_nowhere_to_put_it_replaces_and_says_so` is the other half.
- **Consumed.** `Viewer::apply` *takes* the reserve. A name left standing would have the second
  remote go-to open under the identity of the first, which `Command::Open`'s own rule makes a
  replacement of a tab somebody is reading —
  `a_reserved_name_is_spent_by_the_document_that_opens_under_it`.
- **Nothing changes for a host that never sends it.** `None` is the reserve empty, which is every
  host on this boundary before this round, and the sentence it gets is the one ADR 1227 wrote.

## 3. Both outcomes are said, and the new one is focused

A person who asked for a second window and got one view is owed the difference (trap 5), and so is
one who got a second tab they did not ask *this program* for. `apply` pushes one note or the other
into the same `Event::Reported` the click already raises.

**The document that opens beside is focused**, which is `Command::Open`'s own rule rather than a
second decision: a person who followed a link asking for a new window asked to be reading the
destination, and `Event::Opened` carries the name so a host knows which of the two happened. A host
that wants the source back in front says so with `Command::Focus`.

## 4. A document an action opens is a document this reader has answered for

`Viewer::adopt` hands a replacement every host-supplied value the reader has already given —
§8.10.4's target documents, §8.11.4.4's audience, Table 166's clock, §10.8.3's simulation and
§12.4.4's presentation. **This was already owed and nobody had noticed**: `Open::around` is not
`Viewer::open`, so every document reached through §12.6.4.3's or §12.6.4.4's action since those
clauses were built has been shown under the defaults rather than under this reader's answers. Each
of those five is documented as applying to "every open document and to every one opened
afterwards", and a document shown in this window by this reader is one of those whether they named
it or a link did.

## 5. What this does not close

Nothing here opens a second document from a *person's* gesture. A file chooser is a toolkit's and
`quorra` has none, so the second document a reader can reach today is the one an action names;
several files on a command line is the smaller of the two remaining routes and is
`doc/todo/30`'s. §12.6.4.7's `Thread` action still refuses its `/F`, unchanged.
