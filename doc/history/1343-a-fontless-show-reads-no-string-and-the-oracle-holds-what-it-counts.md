# 1343 — A fontless show reads no string, and the oracle holds every page it counts

Robustness slot of batch fifty-three. ADRs 1521, 1522. No ledger row moved; no question written.

**The slow units (ADR 1521).** Premise checked and partly wrong: `fuzz/artifacts/page/` holds two
units with text shown under no font, `46c2ca1b` (553 483 shows) and `021ace25` (16 716 558), and
both are Type 3 glyph descriptions invoking themselves until `MAX_OPERATIONS`; a fontless show
already returned at once and was reported once a page. §9.3.1 Table 103: "they shall be specified
explicitly by using Tf before any text is shown". Where the instructions went (callgrind, release):
in the first, 3 794 874 notes of a few hundred items, each formatted, cloned and compared; in the
second, every string lexed into a `Vec`, copied into an `Arc` and freed. Built: `Notes` (an ordered
set asked before it changes, and a keyword memo so a repeated sentence is never built) and
`ContentReader::with_operand(Strings::StepOver, …)`, which steps a string over by §7.3.4's own
grammar (`LiteralStringEnd`, now shared with `drop_token`) while no font is in force — exact,
since every operator clears its operands. A/B against the base commit with only these hunks:
15.34 → 12.61 G and 46.64 → 40.62 G instructions; reports byte-identical over all 70 units; an
ordinary page +0.16% (ISO 32000-2 p. 101) and +0.05% (`tracemonkey.pdf`) after the scan was moved
out of line, where inlined it had cost +0.18%. Left: lexing and `NestedContent::of` per glyph run.
Fixtures: `a_string_stepped_over_ends_where_the_lexer_ends_it` (twelve hard string shapes, token
for token) and `text_shown_with_no_font_is_counted_once_a_page_and_reads_none_of_its_strings`
(310 KB past the window: 12 048 shows counted, one report for 10 000 unknown keywords, every fill
after a string drawn). No clock asserted, and why, in ADR 1521 section 5.

**The 26 (ADR 1522).** The run lists them: 26 ambiguous pages we report, every one a page of a
document on `corpus.rs`'s `INCOMPLETE` (25 first pages and `issue6127.pdf` page 2); and the same
shape for 3 our-geometry pages with a `/MediaBox` the standard does not admit. Held by name in
`AMBIGUOUS_ON_A_PAGE_WE_REPORT` and `GEOMETRY_ON_A_PAGE_WE_REPORT`, both directions; a test reads
`INCOMPLETE` out of `corpus.rs` so the mechanism is one fact. `oracle-held`: 836 ambiguous and 3
our geometry, the walk's totals. No sixth verdict: `ambiguous` is what the references did.

**`WHOSE_DEPARTURE`.** The run: 47 contradicted, 2 ours (`issue4436r.pdf`, `issue7891_bc1.pdf`),
12 references', 33 choice — unchanged. The next page to take is `issue7891_bc1.pdf` page 1, 1.11×
on the worst tile, `CONTRADICTED_TIGHT_CONSENSUS`, §10.7.4.

**Gates.** `rustfmt --check` on my five Rust files exit 0; `RUSTFLAGS=-D warnings cargo clippy -p
pdf-model --all-targets` exit 0; `cargo nextest run -p pdf-model` 1772 passed; `cargo test -p
conformance` 361 passed, 1 failed (`records`, on 1341's record in progress). Behind the lock:
`pdf-model --test corpus` exit 0 (59/59); `raster_golden` exit 0, held 974, moved 0; `--test
oracle` exit 0 (ambiguous 836, our geometry 3, contradicted 47); `cargo +nightly fuzz run page` on
both units `-runs=0` exit 0 (20 s and 70 s, instrumented, loaded).
