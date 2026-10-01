# 1464 — A section sign's document is read across its paragraph, not its line

Session 1314. Status: accepted. Amends ADR 0987's "immediately before" (kept; *before* now reaches
the earlier lines of the same paragraph) and closes the gap ADR 1452 section 3 left open.
Code: `tools/conformance/src/citation.rs` (`Paragraph`, `Prose`, `comment_prose`, `opens_a_block`;
`scan`, `scan_prose`, `read_citations`, `read_tables`). Tests: `citation::tests`'
`a_name_wrapped_onto_the_line_above_owns_the_sign`, `a_sign_that_opens_a_block_is_read_in_that_block`
and `a_comment_marker_is_not_an_acronym_and_a_hyphen_is_not_a_number`.

## 1. The defect

`another_document` decides whose a `§` or a `Table` is from the words in front of it, and both
readers handed it the text in front of the sign *on the sign's own line*. Prose wraps wherever the
line runs out, so a standard's name that ends one line and the sign that opens the next were two
unrelated things to every gate: `doc/questions/Q169` line 18's sign after ISO 19005-2 (whose name
ends line 17) passed both `tests/documents.rs` and `--bin section_signs` as a clause of ISO 32000-2.

## 2. The rule

The unit is the paragraph. `Paragraph` joins a paragraph's lines, trimmed, with spaces, and the words
in front of a sign are the paragraph so far followed by the sign's own line up to it. A paragraph ends
at a blank line, a line of code, or a change of comment kind (`//`, `///`, `//!`); a new one opens at
a list item, a heading, a table row, a fence, or the first line of a blockquote; a heading and a table
row are a unit of one line, and a line inside a fence continues nothing. In a Rust source only a line
that is nothing but a comment continues one, and its marker is not a word of it — the prose starts
after `///`, so the marker is never the "acronym" in front of a wrapped name. Every finding is still
reported on the line its sign is on. Both readers take it: `scan` (Rust sources) and `scan_prose`
(documents, ledger notes, `spec-errata`'s populations), for `§` and `Table` alike, since they share
`another_document` and the defect is the same in both.

## 3. What the sharper scan found

Every population `another_document` serves, dumped through the old and the new scanner over the same
tree at the same moment (Rust sources under `roots::source_roots`, the instruction documents
`documents::instructions` names, every ledger note): **nineteen findings changed, none in the ledger.**

- **One new foreign `§`**, the tree's: `doc/pdf-a-conversion-limits.md` line 1418, ISO 19005-2's
  6.2.4.3 wrapped after its name, with a 6.2.4.4 of the same standard on the line. Both are now
  written "section N".
- **Twelve signs that were counted as clauses of ISO 32000-2 are sections of this project's own
  documents** whose name ends the line above — `ADR 0895` §3 (twice), `ADR 0657` §3, `ADR 0775` §1,
  `ADR 0847` §1, `ADR 0865` §3, `RFC 0002` §6.1, `doc/todo/02` §2, `pdf-a-conversion-limits.md`
  §3.2, `RENDER_LIBRARY.md` §2.4, `QUORRA_FEEDBACK.md` §14.2 and §23 — and two letter-suffixed
  sections now name their document (`oracle-and-corpus.md` §3d). Correctly classified, not
  reported; the coverage instruments stop counting the twelve as citations of clauses of the
  standard.
- **Three tables that were ISO 32000-2's Table 3 are ISO/TS 32002's** (`doc/todo/51` line 82,
  `brainpool_p512.rs` line 4, `ecdsa.rs` line 561), and one foreign table's document gains its body
  (`ETSI EN 319 122-2`).

No gate changed its verdict in the worktree. In the main checkout, `section_signs` now names
Q169's line 18 twice — ISO 19005-2 and ISO 19005-4 — beside Q170's line 12; those are the owner's.

## Costs

A wrapped mention now counts as adjacency, which is what an unwrapped one always was: a line ending
"ADR 0044" followed by a line opening with a clause of the standard reads as ADR 0044's section.
Each of the twelve above was read in place and is about the ADR's or the document's own section —
three of them split the name itself across the wrap (`ADR` / `0775 §1`); a writer who means the
standard's clause there separates the two with a word, as on one line. The scan is
still linear in the text; a paragraph's joined text is held for the paragraph's length.
