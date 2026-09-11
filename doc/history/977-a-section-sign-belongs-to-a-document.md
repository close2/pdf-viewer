# 977 — A section sign belongs to a document, and the scanner now asks which

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `bb77086d`. ADRs: 0987, 0997.
Stream: the citation scanner — a gate that had been claiming more than it enforced. Four sibling
rounds were live in the same working tree throughout (`pdf-archive`, `pdf-transform` and
`pdf-font/restate.rs`, the viewer crates, and general improvement).

Files: `tools/conformance/src/citation.rs`, `tools/conformance/tests/conformance.rs`,
`tools/state.sh` (one line: the new count reaches the state report),
`doc/todo/56-a-script-engine-that-is-memory-safe.md` (one paragraph: the rule now says what is
enforced), `doc/adr/0987`, `doc/adr/0997`, `doc/history/977`. **No citation site was rewritten**,
in any crate or any document — that was the point of the round.

## The number

ADR 0984 section 6 reported the defect and priced it at "roughly a hundred and thirty". Off a run
of the changed scanner over `crates/`, `tools/` and `fuzz/`:

```
142 `§` naming a section of one of this project's own documents rather than a clause,
142 of which resolve against ISO 32000-2 and were counted as citations of it
```

All 142 resolved, which is why the gate had been green about them for hundreds of sessions: a
section number of ours lands on a clause number of theirs. Twelve more are in the ledger's notes.
Twenty-two of the 142 landed on clauses 6 to 11 — inside the conformance ledger's population — so
the coverage instrument was counting citations nobody had made. On one tree state the citation
total goes 15 491 → 15 349; across the round it moved for a reason of its own, four siblings
writing into the same tree, which is why the 142 is the measurement and the subtraction is not.

## What changed, in one paragraph

A `§` is now one of three things. With nothing in front of it, a clause of ISO 32000-2, checked as
before. With another **standard** in front of it, a finding, as before — and now recognised
through backticks, so `` `RFC 3986` §5.2 `` fails where it used to pass. With one of **this
project's own documents** in front of it, or with a number no clause can have (`§3a`), a
`ProjectSection`: counted, printed per document, checked against nothing. A wrapper is not
distance; a comma, a full stop, a closing bracket and a possessive all are, and the last of those
is the one the tree itself proves — `` `ledger.toml`'s §8.7.4.1 `` is the standard's clause,
`` `doc/todo/02` §2 `` is ours.

The gate gained a ratchet, `UNNAMED_SECTION_CEILING`, at 46: the `§3a` shape, where a letter
suffix proves it is not a clause and nothing on the line says whose section it is.

## Two things the round did not plan

- **`` `§` `` is the character, not a citation.** Four `pdf-archive` module headers written by a
  sibling this same round say "every `§` in this file is an ISO 32000-2 number", and the checker
  failed the build over all four. `tools/conformance` is the one directory the scan does not read,
  which is why the convention had never been explained anywhere the checker could see it. ADR 0997
  section 1.
- **A standard cited with its year still passes as ours**, because the colon in `ISO 32000-1:2008`
  defeats the number test. Three sites, all in `crates/pdf-archive`, measured and left where they
  are: the fix is one character class in the scanner plus three rewrites in a crate this round did
  not own. ADR 0997 section 2 names each.

## Gates

Core four green (`fmt --all --check`, `RUSTFLAGS="-D warnings" clippy --workspace --all-targets`,
`nextest run --workspace`, `test --workspace --doc`), both `fuzz/` lines green,
`cargo test -p conformance` green. `--bin quotations`: 49 diverging in documents and 5 in ledger
notes, unchanged either side. `--bin pointers`: 185 absent and 14 undefined symbols, unchanged
either side.

`every_quotation_is_the_standards_own_words` failed for most of the round on two files a sibling
was editing — `crates/pdf-syntax/src/write.rs`, where a new paragraph citing §7.5.7 was inserted
between a §7.5.6 citation and its blockquote, and `tools/pdfref/src/undrawn.rs`, a new file whose
blockquote cited no clause. Neither was this round's, neither was touched, and both were green
again by the end of it — which is the gate working, on somebody else's line, exactly as intended.
