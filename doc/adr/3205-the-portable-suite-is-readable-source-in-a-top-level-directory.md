# 3205 — The portable suite is readable source in a top-level directory, and its PDFs are generated

Session 2126. Status: accepted for the first slice; where the suite finally lives is the owner's
(`doc/questions/Q948` question 1). Context: RFC 0016 sections 4 and 6; CLAUDE.md principle 5.

## Decision

The portable conformance suite is `portable-conformance/` at the repository's root. Each test is one
TOML manifest holding the clause, its verbatim sentences, the kind, the file as readable PDF source
(its indirect objects in order, each body written by hand) and the checks with their derivations.
`portable-conformance/tools/generate.py` writes the PDFs, adding only the header, each `/Length`,
the cross-reference offsets and the trailer. No PDF is committed.

## Why

- **A test file someone cannot read cannot be reviewed against its clause.** The precedents that
  ship PDFs (veraPDF's corpus, Isartor, pdf20examples) are reviewed by opening the file. Here every
  byte is in the manifest beside the sentence it tests. pdf-differences' rule that a file is "pure
  text except for the 2nd line comment with 4 binary bytes" is the same idea, applied by hand.
- **Offsets and lengths are what a person gets wrong**, and a wrong one makes the file a test of
  §7.5.4's recovery rather than of its clause. The generator computes them, so `qpdf --check`
  finds no syntax error in any fixture.
- **A top-level directory, not `crates/` or `tools/`**: the suite is not part of this program. It
  must run against processors that have never seen this tree, and moving it to its own repository
  later is then a `git subtree split` of one directory. The adapter for this program
  (`tools/portable-adapter`) stays in the workspace, because it is this program's code.
- **Not its own repository yet**: until the owner answers Q948, the slice is built and reviewed with
  the code it tests, and a reviewer sees both in one diff.

## Cost

A test's PDF exists only after the generator runs, so a consumer needs Python 3.11 or later
(`tomllib`). A file too large or too binary to write by hand (an embedded font program, an image)
cannot be a manifest body; RFC 0016 section 4 gives such a file a source-plus-recipe form,
not yet built.
