# 469 — The annotation the producer wrote

**Finding.** This program could put §12.5.6.6's free text annotation on a page and type into it, and
refused to touch one the *file* states, on the grounds that "replacing an object the producer wrote
is a decision nobody has made". §7.5.6's own list of what an incremental update carries is "objects
that have been changed, replaced, or deleted", and the producer's bytes survive a replacement byte
for byte — so the refusal named an architecture that did not exist, and this tree had been replacing
a field's `/V` and a page's `/Annots` that way since the hundred-and-thirty-sixth session. Twenty-
seven corpus documents state such an annotation and a person can now retype any of them, in the
running program, through messages that already existed.

**Date.** 2026-08-13.
**ADR.** [0304](../adr/0304-the-annotation-the-producer-wrote.md).
**Touched.** `crates/pdf-model/src/view.rs` (`retyped`, `set_free_text`, `free_text_at`,
`write_retypings`, `Written::unappeared`), `crates/pdf-model/src/appearance.rs` (`construct` takes
the view; `free_text_layout` takes the retyping), `crates/pdf-model/src/annotation.rs` (the stored
stream set aside), `crates/pdf-model/src/restriction.rs` (Table 167 bit 10),
`crates/pdf-model/examples/free_text_census.rs` (new), `crates/pdf-model/tests/variable_text.rs`
(four tests), `crates/viewer-core/src/{notes,open,viewer}.rs`, `crates/viewer-ui`'s doc comments,
`doc/conformance/ledger.toml` (§12.5.6.6, §12.5.3, §7.5.6), `doc/todo/33`, `doc/todo/README.md`,
`doc/HANDOVER.md`, `doc/adr/0304-*`, this file.

## The population, before believing anything

Trap 11, and it decided which of the three capability items the round's brief offered was worth
taking. `examples/free_text_census`, over every page of the 971 documents that open:

```text
971 document(s) opened, 27 stating a free text annotation
  73 free text annotation(s)
  54 state /Contents with something in it
  67 carry an /AP /N stream, 11 of which hold a BMC marked-content region
  Table 167: 1 ReadOnly (bit 7), 3 Locked (bit 8), 1 LockedContents (bit 10)
  0 state Table 177's /CL callout line
```

The other two items measure zero: `RadiosInUnison`'s corpus population is 0 by ADR 0197's own table,
and `presentation_census` finds no `/Trans`, `/Dur` or `/PresSteps` anywhere. Table 177's `/CL`, the
other half of `doc/todo/33`, is stated by none of the 73.

## What the census refuted

The line worth keeping. §12.7.4.3's closing paragraph updates an existing appearance stream by
splicing between `/Tx BMC` and its matching `EMC`, **appending** where there is no such region, and
§12.5.6.6 sends this subtype to that subclause — so reusing `appearance::regenerate` looked like the
derived answer rather than a convenient one. The third line of the census kills it: **56 of the 67
streams have no marked-content region**, so for five annotations in six the parenthesis applies and
the new note would have been appended to a stream still drawing the old one.

The reason it is right for a widget and wrong here is the two subtypes' streams: a widget's is
artwork *with* the variable text in a region, and a free text annotation's whole appearance is its
text. So the appearance is replaced, and the stream dictionary rebuilt rather than kept — the marks
are in the page's own space with `/BBox` the `/Rect`, and the producer's `/Matrix` would move them.

## Table 167's two lock flags

Bit 8 is called `Locked` and its own row ends by saying it "does not restrict changes to the
annotation's contents"; bit 10, `LockedContents`, is the one that does. A reader consulting the first
would refuse an edit the table permits. Both are read through `restriction::asserted` rather than at
the point of the edit — `CLAUDE.md`'s rule that a refusal which cannot become an *ask* is the thing
to avoid — which makes this that module's fifth clause and the second addressed to a named object
rather than to the document. The corpus states bit 10 once and bit 8 three times and never both on
one annotation, so the pair is pinned by two hand-built fixtures differing in that bit alone: trap
8's shape.

`doc/md/`'s conversion splits *changes* into "chang es" in bit 8's row, so that sentence is prose
rather than a quotation, checked against `doc/`'s PDF with `pdftotext -layout` first. The handover's
rule — when a gate accuses the standard of a gap, suspect the conversion — held again.

## What it cost the boundary

Nothing. `Query::FreeTextAt` and `Edit::SetFreeText` already said everything a host needed, so the
six consumers on `viewer-core`'s boundary gained this by being recompiled. Three `pdf_model`
signatures changed and each failed every consumer's build until it said what it does.

## Proved in the running program

Under `Xvfb`, on `freetexts.pdf`, whose page one carries six notes by six producers. A click on the
one Firefox wrote printed `note: typing into the free text annotation 32 0`; typing changed what the
page draws; `s` wrote 45 447 bytes. `cmp -n 44273` says the producer's file is byte-identical
underneath; object 32 is the annotation with its new `/Contents` and object 33 is Firefox's own
appearance stream, replaced under its own number. `pdftotext` and `mutool` read the new text back.

## Gates

`fmt`, `clippy --workspace --all-targets` silent, `nextest --workspace` 1679 passed, doctests,
corpus, oracle (905 agree / 68 contradicted / 786 ambiguous / 18 no render), text extraction 99.3%,
dates, xmp, jpeg2000, quorra, conformance. Nothing moved but the four tests added.
