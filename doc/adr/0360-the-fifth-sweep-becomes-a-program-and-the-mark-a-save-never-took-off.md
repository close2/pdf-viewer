# 0360 — The fifth sweep becomes a program, and the mark a save never took off

**Status.** Accepted.

## Context

`doc/todo/01`'s binding rule for a sweep round: commit one more prose sweep as a program before
running any of them. Six of the fifteen were commands — `conformance --bin entries`,
`--bin quotations`, `--bin unread`, `--bin blockers`, `--bin capabilities`, `--bin retired` — and
nine were descriptions.

The **fifth** sweep is the one that asks the question from the other end. Every other sweep here
reads what a *row* claims; this one takes every `pub fn` in `pdf-model` and asks who calls it,
because a capability can reach the crate that implements a clause and never reach a program. It
has produced a finding on most of its runs since the two-hundred-and-fifty-third session:
§12.5.6.19's `/H` was `implemented`, argued in an ADR and tested with pixels, while `viewer-core`
took the pressed annotation from `link_at`, so no host could press a widget for a hundred and
fifteen sessions; §8.11.4.3's `/ListMode` was read and asked by nothing with a layer panel on the
screen; `Signature::must_cover_whole_file` drew a `shall` no host distinguished;
`Collection::initial_document` could not be called at all.

**And it is the sweep whose *number* is the finding**, which is why it was the one to build. The
four-hundred-and-eighth's result was that a whole new host program took **zero** names off the
list; the four-hundred-and-thirteenth's, that four more hosts — one of them in C — took exactly
one. Those are deltas, and a delta cannot be read off two different instruments. Every run has
been a script written for that session: the four-hundred-and-eighth recorded 246 `pub fn`s and 85
unnamed where the four-hundred-and-fifth's script said 246 and *86* at the same commit, and the
five-hundred-and-seventeenth printed 101 and 82 against the five-hundred-and-tenth's 92 and 77
with the population unchanged at 286 and nothing in the tree having moved. `doc/todo/01` had to
write "a session-local extraction whose *level* is not comparable across runs" beside its own
numbers, which is the same failure `CLAUDE.md`'s "write down the command, not the answer" names.

## Decision

**`cargo run --release -p conformance --bin callers [crate]`** is the fifth sweep as a program
(`tools/conformance/src/callers.rs`, twelve unit tests, 0.2 s over the whole tree). Four decisions
are worth their lines.

**Who could possibly be a caller is a question for the manifests, not for a list in a script.**
The by-hand runs grepped a typed-in list of host crates, which grew from two to four to five to
eight as hosts arrived and which nothing maintained. Here a consumer is a crate whose `Cargo.toml`
names the answering crate, so the population maintains itself — fifteen of them today — and the
manifest settles a false positive no grep over directories can: `render-gpu` and `render-quorra`
name `pdf-model` in `[dev-dependencies]` only, so a match in *their* `src/` is a word rather than
a call, and the sweep does not count it.

**The three known populations become rungs.** Every by-hand run ended by sorting its unnamed names
into "functions `pdf-model` calls itself", "functions only a test or an example reaches" and
"functions nothing names at all" — prose, redone each time. `Reach` is that sorting, ordered by who
asks the question: a dependent crate's `src/`, a tool or a fuzz target, the answering crate's own
`src/`, a test or an example, nothing. The report prints the rungs from the bottom, because that is
where the findings have been.

**Two exclusions that decide what the rungs mean, and both were judgements the grep left to the
reader.** A file's own `fn NAME` is not a call, so a definition cannot answer for itself; and a
`#[cfg(test)]` item inside `src/` is scaffolding rather than use — without that split
`Collection::all_folders`, whose only caller is an example, reads as used by its own crate on the
strength of two lines in `collection.rs`'s test module. A **comment** is not a caller either, which
is the same rule one step further: a doc comment naming a neighbouring function is how this tree
explains itself.

**It stays loose in the direction that costs a hit rather than a claim.** A name is matched as an
identifier, so a short name shared with another type's method (`read`, `new`) reads as named, and a
name reached through a wrapper reads as unnamed — the four-hundred-and-second recorded
`document_part::first_page` as exactly that. The half a reading acts on is the *unnamed* one, and a
name reported there is genuinely absent from every file that could call it.

**Not a gate**, for ADR 0249's ratio reason and one of its own: most names below the top rung are
helpers that happen to be `pub`, and which of them is a clause's noun is the reading.

## Consequence

First run: **289 distinct `pub fn` names in `pdf-model`, 15 crates naming it in a manifest; 174
named by a dependent crate, 19 by a tool or a fuzz target, 73 only inside `pdf-model` itself, 21
only by a test or an example, and 2 by nothing at all** — so 115 names no crate under `crates/`
asks, which is the number to compare the next run against.

**`ViewState::additions` was one of the two, and its own doc comment named the caller it did not
have**: "what a save writes **and what a host asks to know whether there is anything to save**".
No host asks it. `viewer_core::Open::dirty` answered that question from `cursor > 0` — the length
of the undo log — and **nothing ever put it back to `false`**, because a save does not shorten a
log. So a document saved and left open went on saying it had unsaved work: `Event::Dirty` never
came back false and `viewer-ui`'s title kept its mark for the rest of the session.

The fix is the distinction the model already draws and the host had flattened. What is unsaved is
the cursor's distance from the last save, not from zero: `Open::saved_at` records where the cursor
stood when §7.5.6's update was written, `dirty()` compares the two, and `Viewer::save` says so with
an `Event::Dirty { dirty: false }` on the same pass that emits `Event::Saved`. An undo back across
the saved point is dirty again and a second save announces nothing, which is what a cursor means.
`a_save_takes_the_unsaved_mark_off_and_an_edit_puts_it_back` pins all four states.

**This is the sweep's own shape at one remove**, and worth naming as such: the finding was not that
the model lacked something. The model had the answer, in a function whose doc comment *predicted*
its caller — the strongest form of this failure, and the same one `requirements::unmet` showed in
the two-hundred-and-twenty-first session — and the host answered a different question that agrees
with it everywhere except at the moment of saving. **A `pub fn` nobody calls is sometimes a
question somebody else is answering wrongly.**

The second name nothing calls is `structure::Namespace::is_standard`, and it is the other
disposition this sweep has: §14.8.6.2's "all structure elements shall be in at least one of the
standard structure namespaces" is addressed to a *document*, so asking it is validating a file
rather than drawing one. The caller it waits for is `doc/todo/48`'s second owed item, which needs
exactly this predicate; both the function and the todo say so now, and nothing was built — which is
what this sweep got right about `Query::Find` in the four-hundred-and-thirteenth.

## What this does not do

It does not read a *type* or a variant. The four-hundred-and-thirteenth ran the sweep a second way,
over `viewer-core`'s own vocabulary — every `Command`, `Query`, `Answer`, `Event` and `Edit`
variant against the crates that speak it — and that is where `Query::Find` and
`Query::LogicalSelection` turned out to reach no program. This program answers the function
question only; the vocabulary question is a second population and is still a by-hand run.

It does not know what a name *means*, which is the whole reason it is a reading list. 72 names sit
on the rung where only `pdf-model` names them and most are helpers; the ones worth opening are the
ones whose name is a clause's noun, and no program can tell which those are.
