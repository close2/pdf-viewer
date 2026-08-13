# Session 494 — the line that says which place the note is about

**Finding:** Table 177's `/CL` was refused for want of a colour, and the colour was never the
thing — a construction that names none paints in Table 51's initial black, and what actually
separated the callout from the `/BS` border beside it is that Table 166 gives every annotation a
border by default and no annotation a callout.

Date: 2026-08-14. Argued in [ADR 0329](../adr/0329-the-line-that-says-which-place-the-note-is-about.md).
Closes the first of the two items `doc/todo/33` still carried, and extends
[ADR 0193](../adr/0193-a-construction-is-not-a-form-xobject.md)'s table with a fifth entry.

## Files touched

- `crates/pdf-model/src/appearance.rs` — `callout`, the `CALLOUT_SHAPE` refusal, `free_text`'s
  ordering, `undrawn_decoration` losing its `/CL` branch, and `FreeText` joining the unbounded
  subtypes.
- `crates/pdf-model/tests/variable_text.rs` — the five callout tests, replacing the one that
  asserted the refusal.
- `doc/conformance/ledger.toml` — §12.5.6.6.
- `doc/adr/0329-…`, `doc/todo/33-annotation-editing.md`, this file.

## What the round was

Spec-driven, and the only track available: `examples/free_text_census` counts **0 of 73** free
text annotations in the corpus stating a `/CL`, on every page rather than on first pages. So both
gates were expected to reproduce line for line, and reproducing is the check rather than an
absence of one.

The clause turned out to say more than two ADRs had taken from it. Table 177 states the geometry
completely — two points or three, one of Table 179's ten shapes at (x1, y1), and Figure 79 draws
the result — and it states the condition to draw on *another entry's value*: "meaningful only if IT
is FreeTextCallout", with `FreeTextTypeWriter` saying outright that "no callout line is drawn". So
the two intents that are not this one draw nothing **and report nothing**, because a report fired
outside the table's own condition names a gap the clause does not have. That is trap 11's shape,
and it is the fifth time in this project's history that the honest condition turned out to be
narrower than the reflex one.

Three smaller things came out of reading the entries together rather than one at a time.

`/RD` is what makes a `/Rect` hold both the note and the line: "[t]he inner rectangle is where the
annotation's text should be displayed", so `/Rect` on a callout annotation is not the text's box
and what occupies the difference is the callout. Read apart, each entry looks under-specified.

`/BS` is not the callout's width, because the same `/RD` row binds it to "the border of the inner
rectangle" — so the width is a choice, taken at §12.5.4's one point, and a `/BS` of `/W 0` means
"no border" rather than "no callout".

And `/CL`'s coordinates are "in default user space" — the words ADR 0193 found on four other
entries when it decided that a construction is bounded by `/Rect` only where its clause bounds it.
A free text annotation belonged on that list and could not have been noticed while the entry was
refused. Joining costs the text nothing: §12.7.4.3's own example puts the clip inside the
construction, so `variable_text::lay_out` has clipped the value to its own box since before
`bounded` existed.

## The pictures

Trap 1, at four times scale on a 200×100 page with `/Rect [20 40 180 70]`, every callout below the
rectangle: the two-point `/CL` draws the diagonal; the three-point one bends at its knee; `/LE
/ClosedArrow` puts an arrowhead at (x1, y1) pointing away from the note, **unfilled**, because
Table 179 fills it "with the annotation's interior colour, if any" and Table 177 gives this subtype
no `/IC`; `/RD [40 0 0 0]` moves the text forty points and the line not at all; and `/IT
/FreeTextTypeWriter` draws the note alone. Figure 79, drawn.

Every fixture is a pair differing in one entry, which is the only thing that separates a test of
the rule from a test that something was drawn — the two-point line is inked at (40, 20) and not at
(30, 20), and the three-point one is the exact reverse.

## An environment note that cost time

`/tmp` was at 100% on this machine, and `sccache`'s temporary directory is under it — so every
build failed with "No space left on device" from inside a wrapper nothing in the invocation names.
`RUSTC_WRAPPER=` and a `TMPDIR` inside the worktree got past it. It is a fact about the machine
rather than about the tree, and `doc/environment.md`'s note that sccache "costs little" is true of
its cache and not of a full `/tmp`.
