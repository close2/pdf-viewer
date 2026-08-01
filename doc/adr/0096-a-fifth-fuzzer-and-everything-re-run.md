# ADR 0096 — A fifth fuzzer, and everything re-run

Status: accepted, 2026-08-01.

## Context

Six sessions have added two things this project's own principle 3 says must be fuzzed from the
first commit and were not: §7.9.4's date parser (ADR 0092) and §12.7.8's Forms Data Format reader
(ADR 0090). The second is the sharper case — **an FDF file is a second file a document asks a
person to open**, so its bytes are exactly as untrusted as a PDF's and rather more surprising,
because nothing else in this tree opens a second file at all.

The date parser is the sharper case for a different reason. It carries two
`#[expect(clippy::arithmetic_side_effects)]` blocks, each argued from bounds the parse
establishes — a sign in −1..=1, hours in 0..=23, a four-digit year through `days_from_civil`. A
sentence in a `reason =` is a claim, and a fuzzer is what checks it.

## Decision

**A fifth fuzz target, `forms_data`, covering both.**

The bytes go in twice. First as a §7.9.2.2 text string handed straight to `Date::parse`, because
reaching it through a document would test the document reader instead; the target then asserts
every range the clause states — `MM (01-12)`, `DD (01-31)`, `HH (00-23)`, the offset within
±23:59 — and calls `instant()` and `to_string()`, which are the two places the parse's bounds are
relied upon rather than re-checked.

Then as a whole FDF file. That exercises §12.7.8.2.2's header (`xref::read` looks for `%FDF-`
where `%PDF-` is absent, and §7.5.2 makes every byte offset relative to whichever it found — an
input putting one marker inside the other is what this notices), §12.7.8.3.2's `/Kids` tree and
its concatenated fully qualified names, and Table 249's flag arithmetic. Then
`ViewState::import` against the FDF file *as its own target*, which is a document with no
`/AcroForm` — the pairing's commonest real answer.

## What was run

Everything, rather than inherited. This is the third such session (79, 99, 106) and the interval
is now six sessions.

| | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets` | clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects` |
| `cargo test --workspace` | 838 tests, 0 failures |
| `cargo deny check` | advisories, bans, licenses, sources — all ok |
| `forms_data` fuzzer | clean at 50 000 **and at 2 000 000 runs** |
| `lexer`, `cmap`, `crypt`, `variable_text` | clean at 50 000 runs apiece |
| corpus gate | 86 incomplete, 0 unopenable, 0 slow |
| oracle gate | 839 agreeing, 65 contradicted, 750 ambiguous |
| text gate | 97.9%, 44 below the floor |
| date census | 1511 of 1542 conform |

Two million runs on the new target rather than fifty thousand, because it is the one whose
`#[expect]`s are an argument rather than a bound the compiler checks. It found nothing, which is
the evidence those two `reason =` strings did not have before.

## The speed comparison, remeasured

`hayro-speed` over the whole corpus, in one sitting:

| | hundred-and-sixth session | ninety-ninth | seventy-third |
|---|---|---|---|
| total, ours | **6.99 s** over 862 complete pages | 7.08 s over 859 | 6.91 s over 858 |
| total, `hayro` | 39.59 s | 49.03 s | 41.87 s |
| **median page** | **2.14×** slower | 2.15× | 2.14× |
| worst page | 68×, `issue19176.pdf` at 642 µs against 9.4 µs | 50× | 63× |

**Our own total fell while the page count rose** — three pages joined the complete set in the
hundred-and-third session (ADR 0093) and the total still went down, which is that session's 2.5%
showing up in aggregate.

And `hayro`'s total moved **49.03 s → 39.59 s** with nothing in this tree touching it, which is
the third time this file has recorded that number swinging by more than 20% between sessions. The
handover's rule stands and is worth restating: **the number to trust across sessions is our own
total.** The median is a ratio and its denominator is not ours.

`issue19176.pdf` remains the worst page at 68× and remains meaningless: a 9×11-point page where
the absolute numbers are 642 µs against 9.4 µs, and the second is below the resolution at which
anything can be concluded.

## Consequences

Five fuzz targets. Nothing else changed and nothing was found, which is the outcome a
verification session should usually have — and the reason to date it: every claim in
`doc/HANDOVER.md`'s first bullet now names the session it was last run in rather than inheriting
one.
