# ADR 0094 — The document inside the document

Status: accepted, 2026-08-01.

## Context

The sixty-second session's action census named `GoToE` as **the largest action nobody had
written**: 31 actions in 2 corpus documents, behind only `GoTo`, `JavaScript` (excluded) and
`URI`. It sat in `action.rs` beside `GoToR` and `Launch` under one sentence — the refusals that
"want a file system or a network, which principle 3's sandbox deliberately withholds".

That sentence was wrong about this one, and had been since the eighty-sixth session read §7.11.4's
embedded file streams (ADR 0076). §12.6.4.4:

> An embedded go-to action (PDF 1.6) is similar to a remote go-to action but allows jumping to or
> from a PDF file that is embedded in another PDF file

**A `GoToR` names a file on a disk. A `GoToE` names a file inside the one already open.** The
whole of its path is bytes this program already holds, and there is no permission to ask anybody
for.

## Decision

**`GoToE` is read whole and performed.** Tables 204 and 205, and the clause's own vocabulary
drives the shape:

> The T entry in the action dictionary is a target dictionary that locates the target in relation
> to the source, in much the same way that a relative path describes the physical relationship
> between two files in a file system. Target dictionaries may be nested recursively … As the
> hierarchy is navigated, each intermediate target shall be referred to as the current document.
> Initially, the source is the current document.

So `EmbeddedGoTo::path` is a **list of steps**, flattened out of `/T`'s nesting, and
`target_in` walks it holding a stack of opened documents. A `/R /P` step pops; a `/R /C` step
opens a child, either by its key in the `EmbeddedFiles` name tree or through the file attachment
annotation Table 205's `/P` and `/A` name — each in both of the two spellings the table gives it,
a page index or a named destination, an `Annots` index or an `/NM`.

The clause's EXAMPLE is the test: a child, the parent, and a sibling written as `/R /P` nesting
`/R /C`. The third is what makes a path a list rather than a step.

## Four refusals, each its own sentence

`TargetError` has four cases because a caller wants to say four different things, and none of
them is this program failing at something it implements:

- `/F` names another **root document**, which is a file on a disk — `GoToR`'s refusal exactly.
- A `/R /P` from the document a person opened. **An embedded file's parent is the file it was
  taken out of**, and the document at the top of the stack has none inside itself. That is why
  §12.6.4.4's sibling EXAMPLE is performable only from a document that is already a child.
- The current document embeds no such file.
- The bytes are not a PDF this reader can open.

§12.6.4.4's NOTE asks for one more thing and `MAX_TARGET_DEPTH` is the whole answer: "[i]t is an
error for a target dictionary to have an infinite cycle … Interactive PDF processors need to
attempt to detect such cases and refuse to execute the action if one is found." The path is a
finite list read once and each step opens at most one document, so a cycle in `/T` cannot become a
loop. Each opened document inherits the parent's `Limits`, which is what stops a file that embeds
itself from being a decompression bomb with extra steps.

## The destination belongs to the target, and §12.3.2.2 says so

`/D` is kept as the object the file wrote and resolved in the *opened* document. Two reasons, and
only the second is obvious. A **named** destination is looked up in the target's own `/Dests` and
name tree, so resolving it against the source would answer about the wrong document. And an
**explicit** one is not a reference at all:

> No page object can be specified for a destination associated with a remote go-to action …
> because the destination page is in a different PDF document. In this case, the page parameter
> specifies an integer page number within the remote document instead of a page object in the
> current document.

with a NOTE saying "[t]he above paragraph was corrected to also include embedded go-to actions
(2020)", and Table 203 giving the numbering: "[t]he first page shall be numbered 0."

`Destination::page_index_in_target` is that one case. `Destination::page_index` answers `None` for
an integer target — correctly, because there it is a number about a file this reader does not
have — and the new method reads it as the index it is once the target is open. Every one of
`issue17056.pdf`'s 30 actions is that form, and before this method they all resolved to nothing.

## The viewer opens it, and drops one thing on the way

`viewer-ui` replaces the open document with the target and rebuilds everything derived from it —
§12.4.2's labels, §12.3.3's outline, the `ViewState`. Table 204's `/NewWindow` is a *should* and
this program has one window, so it replaces and says so out loud rather than pretending to have
obeyed.

The one thing deliberately **not** inherited is the directory. §12.7.6.4's import-data policy
(ADR 0090) resolves a file name against the directory the open document is in; an embedded
document is not a file on this disk, so it has no siblings and is given none. A document that
reached the screen from inside another file cannot name a path.

## Consequences

`reported` falls 51 → 50 and `partial` rises to 232. 837 tests. No gate moved and none could:
`issue17056.pdf` draws its own first page completely either way, and what changed is what happens
when somebody clicks.

What this leaves is the shape §12.7.8.3.4 asked for in ADR 0090 — an FDF annotation whose `/AP`
lives in one file while the page it belongs to lives in another. That row is still `reported`, but
the machinery it named as "a second `Document` reaching the interpreter" now exists and has a
caller.
