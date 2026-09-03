# 0839 — The foreign readback, and what a second reader is asked

Session 898. Status: **accepted**. RFC 0002 §9's fourth layer, owed since the suite's first writer
and named as a gap by four consecutive rounds' history files.

## Context

Four writers exist — `attachments --attach` (session 885), `split` (886), `merge` (888) and
`pages` (893) — and every gate over them checks their output with **this tree's own parser and this
tree's own rasteriser**. That answers "did we write what we meant to". It cannot answer "did we
write what the format says", because a misreading on the way out and the matching misreading on the
way back in agree with each other perfectly.

Session 897 named the sharpest reason: §14.7's structure tree is read only by an assistive
processor, so a tree only this program can make sense of is exactly what a raster gate is least
placed to notice. ADR 0838 is that prediction coming true — a parent tree naming nothing, on 70 of
83 tagged corpus documents, invisible to every instrument this crate had.

## Decision

### 1. The comparison is foreign-to-foreign, and nothing else

`crates/pdf-transform/tests/foreign_corpus.rs`. poppler draws the source page; poppler draws the
derived page; the two are compared. A difference is **ours**: one reader, one page, two files this
suite says carry the same content stream, the same resources and the same boxes.

**No foreign reader is ever compared against this tree's renderer here.** That is the oracle's
question with the oracle's tolerances, and mixing the two makes a disagreement unattributable —
traps 3 and 9 are both about exactly that confusion. The consequence is that this gate can assert
**bit identity**, which the oracle cannot: two runs of one program over one page have no
antialiasing argument to have.

### 2. Three readers, three questions

- **`qpdf --check`** — structural soundness, as a *change of verdict* rather than a verdict. qpdf
  answers 0 for a sound file, 3 with warnings and 2 with errors; the walk fails only where the
  source was not 2 and the derived file is. A corpus document qpdf already complains about says
  nothing about what we wrote (trap 11).
- **`pdftoppm` and `mutool draw`** at 48 dpi, page 1 of each — split's piece, merge's first page,
  pages' surviving first page, and attach's whole document, all of which put the source's page 1
  at page 1 by construction. That is what lets one comparison serve four writers.
- **`mutool show`** — §14.7's parent tree, which is the only one of the four writers' outputs that
  nothing rendered can see. mupdf resolves the entry for the page's own §14.7.5.4 key in both
  files and the walk compares the **shape**: one character per array member, `r` for an indirect
  reference and `-` for null. The members themselves are different objects in two different files
  by construction; what the clause makes comparable is the *index*, because an `/MCID` is "a
  zero-based index into the array" and the content stream crosses byte for byte. `pdfinfo`'s
  `Tagged:` line is asked beside it, because it is the one thing a *second* foreign reader will
  say about tagging at all.

Where the walk needs to know *which* key to ask about it uses this tree's own reader. That is not a
correctness claim — both sides' content comes from mupdf — and it is the only way to compare a
source's key with the output's renumbered one.

### 3. The population is two samples with two costs

Every document stating `/StructTreeRoot`, because the structure lane needs a tagged population and
the corpus has 90; plus every eighth document by sorted name, because a document costs up to
fourteen foreign process invocations. 203 documents, 78 seconds, and the same sample on every
machine.

### 4. A timeout takes a document out of the comparison, and does not fail it

Learned on the second run rather than designed in. `issue19517.pdf` — a 6 MB page — costs poppler
**23.9 s** and mupdf **17.6 s**, measured on the source and on the derived file and identical to
within a second on both; a 20 s budget therefore decides that document by which side of the bound
the machine happened to land on, and the walk failed in the gate sequence having passed alone
twenty minutes earlier. The budget is a bound on this walk's wall clock and not a claim about a
document, so a reader that outruns it has said nothing: the document leaves *that reader's*
comparison and is counted under its own heading. It is the treatment a timed-out **source** render
already got by construction, made symmetric. `doc/todo/02` §2's paragraph about a gate that spawns
another program is the same rule; this is one more instance of it.

The classification is by **elapsed time**, not by the error's text: `pdfref` reports a killed
renderer as an ordinary `RendererFailed`, and matching on its message would be an assertion on a
substring (trap 27).

### 5. It skips where the readers are absent, and says so under its own prefix

The one gate in this crate where a skip is right rather than a failure: with no foreign reader
there is no foreign readback, and every other property of these four writers is asserted by the
four walks beside it. The skip line carries the `transform-foreign:` prefix so that
`tools/state.sh`'s filter matches it and the section stays legible instead of reporting that the
gate said nothing.

### 6. The scan of mupdf's output is itself tested

Everything the structure lane asserts is read out of text another program printed, so a scan that
misread it would make the lane say whatever it liked (trap 27).
`the_scan_of_mupdfs_output_reads_both_forms_of_a_parent_tree_value` is an ordinary test over
literal strings in both of §7.3.10's forms, including the `[null]` ADR 0838 was about, and it runs
in the workspace suite rather than under `--ignored`.

## Consequences

- **One defect, fixed**: ADR 0838.
- **Three renderings held, one reading.** `bug854315.pdf`, `issue16553.pdf` and `issue17069.pdf`
  differ under mupdf and not under poppler, and the cause is ADR 0821 §2: §12.8.1's `/V` is dropped
  from a signature field that crosses into a merged document, because the signature is over bytes
  the new file does not have. mupdf draws an *unsigned* signature widget as a placeholder over its
  `/Rect` rather than from its `/AP /N`. That is a decision of this suite showing up in another
  program's output, which is what this gate is for; the entries record it as such rather than as
  the document's.
- **Two structure shapes held, one reading, and a warning added.** `bug1365930.pdf` and
  `paragraph_and_link.pdf` state parent-tree entries naming structure elements their **own**
  hierarchy does not reach: §14.7.2 makes the hierarchy what `/StructTreeRoot`'s `/K` reaches and
  Table 355 makes `/P` required, and in both files the subtree the parent tree names hangs off an
  element that states no `/P` at all. This suite carries the hierarchy, so the array position goes
  to null — and it now **says so**, which it did not: `Carry::report` counts the members a source
  filled and the output could not, and names §14.7.5.4 and §14.7.2 in one sentence. A carried tree
  that quietly loses a marked-content sequence's structure is the shape of failure this whole ADR
  exists to prevent, whoever caused it.
- **`qpdf --check` never lost a sound file**, on any of the four writers, over the whole sample.
  That is the strongest single sentence available about the serializer ADR 0817 introduced.
