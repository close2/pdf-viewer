# 1840 — Every problem the converter knows, joined to its remedies, and a profile that answers in order

Slot 1 of batch 125, 2026-10-28, an "archive" round (Opus), the PDF/A design slot; ADR 2514, RFC 0014
and Q706 written, ADR 2515 left unused (part (c) not built). I worked in `scratchpad/r1840/`.
No ledger status moves and no source file differs from HEAD.

**Premise** (RFC 0007's format, `--config`, the sites, the `Loss` words and the blocking families as
the brief names them): held. **Hypothesis** (sites, losses and reason families are three views of one
set): held for two. Every configurable site is a `pdf_archive` requirement and every loss word is
taken at a site or by a profile-level key, but the blocking list carries problems no requirement
states (a declined source, a header, an undecodable filter, the unreadable, the pageless, dynamic
XFA), so the table has three parts (ADR 2514).

**Built.** `doc/pdfa-remedies.md`: a row per requirement a file can fail that is a site, a refusal,
a loss or an owed remedy (one line per family for the rest); the problems no requirement states,
each with its external-tool row; the owner's two first-named reasons per target; a loss index sorted
into information and presentation. Each row: the problem in `pdf_archive`'s words with its clauses,
every remedy built, buildable, external tool or rejected with its loss, each shipped profile's answer
(from `doc/profiles/`, matched at 2b and 4 against a census of the 42 profile-by-target listings), and
the two proposed profiles' lists — joined by a scratch script from four Sonnet censuses.
RFC 0014: `try = [...]` per site and target, evaluated per object; `append-statement`;
`derive` as program, then this converter, then attach or append; the operator's file checked at six
targets with `base`; `no-data-loss` and `state-every-loss` defined against the shipped seven.

**Found.** ISO 19005-2 section 6.8 admits an attachment conforming to part 1 or 2, so a derived file
converted to part 2 is a remedy at PDF/A-2, against RFC 0007 section 4.6's table (ADR 2514 item 4).
A57's own condition for a chain, a real configuration wanting two steps, is met three times; Q706
item 1 asks rather than taking it. The statement page's labels are outside ADR 1014's "no sentence
added" as written; Q706 item 2 offers the owner a sentence for `CLAUDE.md`.

**Gates.** `cargo test -p conformance --no-fail-fast` exit 101: three reds, none mine —
`every_quotation_is_the_standards_own_words` (`crates/pdf-syntax/tests/cross_references.rs:1696`),
`the_ledger_agrees_with_the_standard_and_with_the_tree` (§12.5.6.23 and §11.7.5.3 naming siblings'
ADRs 2528 and 2532 not yet written), `every_ignored_test_file_is_named_by_the_sequence_or_says_why_it_is_not`
(a sibling's `pdf-model` test); the documents gate (the `§` rule over the new files) passed.
`--bin quotations` exit 0, `--bin pointers` exit 0, `--bin cited` exit 0. The archive recipe was
not owed: no code was built.

**Unfinished.** Part (c), the two first-named reasons, is batch 126's, as `doc/pdfa-remedies.md`
section 4 lists their steps. Duration 2100 s, about 80 tool calls.
