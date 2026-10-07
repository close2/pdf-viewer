# 1624 — A comment saying `doc/md/` lacks something is read against `doc/md/`

Session 1394. Status: **accepted** and **built**. Extends ADR 1273's sweep family (a doc comment's
name exists) to a comment's claim about the standard's conversion. Supersedes nothing.
Context: `CLAUDE.md` principle 5 ("'The specification defines nothing here' is itself a claim
about the specification, and it decays"); ADRs 0252, 0253, 1273.
Code and test: `tools/conformance/tests/doc_md_absences.rs`.

## The incident

`pdf-model`'s `attachment.rs` said Errata Collection 3's Table 409a was not in `doc/md/`, so
`MCAF` could not be checked against this project's copy and the function would narrow "when it can
be read". The table was there all along, under its `## Issue #374` heading. Its one row is `MCAF`.
Nothing compared the sentence with the directory it described.

## The shape

1. **The population** is every comment paragraph (`//`, `///`, `//!`) under the workspace's source
   roots. A paragraph is a run of comment lines, and a blank comment line ends it.
2. **The unit is a segment, not a sentence.** A sentence is cut at `;`, `:`, a parenthesis and a
   dash as well as at its end. The first version read whole sentences and charged `xmp.rs`'s
   *(paraphrased, since the caret's text is not in `doc/md/`)* with the `§14.3.2` named outside
   the parenthesis. A segment is the smallest unit that keeps a claim and its object together in
   every case the tree holds.
3. **A claim** is a phrase of absence (`absent from`, `not in`, `missing from`, `has neither`,
   `holds no`, `carries none` and their kin) within 40 bytes of `doc/md`.
4. **Its subjects** are a `Table` number, an `Issue #` number, a `§` clause and a backticked PDF
   name (`/BrotliDecode`). A bare backticked word is not a subject: `` `None` `` and `` `ja` `` are
   Rust and BCP 47, not claims about the standard. A claim that says *the table*, *the clause*,
   *the erratum* or *the name* also takes the last subject of that kind earlier in the paragraph.
   That back-reference is how the incident's sentence was written.
5. **Present** means: a table, an issue or a name appears as that label with nothing alphanumeric
   after it, or a clause is a Markdown heading that opens with its number.
6. **Held at zero.** A claim whose subject is present fails, naming the file, the line and the
   segment. There is no version of the tree in which that sentence is acceptable. A claim with no
   searchable subject ("the caret's text", "the standard's equations") is counted and printed,
   because the directory cannot answer it. Today that is 5 of 7.
7. **Calibrated** by the second test. It runs the sweep's functions over the incident's shape and
   over a true claim (a Table 9999 and an Issue #99999), and requires that the first is found and
   the second is not.

## What it does not do

It reads `doc/md/` only, which is what the claims name. An absence claimed of another standard's
text ("that document is not in `doc/md/`") has no searchable subject and is printed. It does not
read Markdown documents: `doc/errata-read.md` is the record of what the conversion dropped, and
that record is written to be re-read rather than swept.
