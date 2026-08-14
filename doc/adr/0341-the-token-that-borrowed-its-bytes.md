# ADR 0341 — The token that borrowed its bytes, and the number parsed in place

Status: accepted, 2026-08-14. Session 506. Takes `doc/todo/44` §2's named candidates — the
lexer first, number parsing second — with ADR 0332's attribution as the justification and
its instruments as the measure.

## Context

ADR 0332 attributed the owner's document (`tmp/Entwurf.pdf`, one page, a 141.12 MiB content
stream of 20 834 587 lexer tokens) to `Lexer::next_token` at 63.6% of a 22 411 M-instruction
interpretation, and named the shape under it: `read_regular_run` ended in `.to_vec()`, so
~21 M tokens were ~21 M short-lived `Vec<u8>`s — the allocator's ~20.8% of the whole — and
every numeric operand went through `f64::from_str` (15.1%) after being copied a *second*
time into a `String`. The todo named two candidates and required the callers be surveyed
before an API was chosen.

## Decision 1: `Token<'a>`, with `Keyword` borrowing and `Name`/`String` still owning

The survey first, because the design follows from it. Every consumer of
`pdf_syntax::Token` in the tree:

| caller | what it does with the payload |
|---|---|
| `pdf-syntax/src/parser.rs` | matches keywords against literals; `Name`/`String` into owned `Object`s |
| `pdf-syntax/src/xref.rs` | matches keywords (`xref`, `trailer`, `n`, `f`, `obj`) |
| `pdf-syntax/src/document.rs` | integers only (object-stream headers) |
| `pdf-model/src/content/run.rs` | the interpreter: keyword = the operator, matched and formatted; operands into owned `Object`s |
| `pdf-model/src/inline_image.rs`, `appearance.rs`, `variable_text.rs` | keyword matching; names/strings read or re-serialised |
| `pdf-font/src/cmap.rs`, `tounicode.rs` | buffers whole *sections* of tokens in a `Vec` before reading them |
| three census examples, two fuzz targets | matching only |

Not one caller stores a keyword's bytes beyond the token itself; every keyword is compared
against a literal or formatted into a report. So:

- **`Keyword(&'a [u8])`, borrowing from the lexer's input.** A keyword — and a number,
  which is read through the same `read_regular_run` — is a span of the input *verbatim*,
  so a borrow is possible and exact. The lifetime is the input's, not the lexer's
  (`fn next_token(&mut self) -> Option<Token<'a>>`), which is what lets `cmap.rs` keep
  buffering tokens while the lexer advances.
- **`Name(Vec<u8>)` and `String(Vec<u8>)` stay owned.** They are *decoded* — `#xx`
  escapes, backslash escapes, hexadecimal pairing — so their bytes may not exist in the
  input at all, and a borrow is not generally possible.

**Why not the other two designs the todo listed.** A `Cow<'a, [u8]>` on all three payload
variants was rejected because the survey shows it would never be exercised: every keyword
can borrow and no name or string consumer could use a borrow without `.into_owned()` at
the same call sites that today take the `Vec` — a `Cow` that is always one variant per
constructor is an enum documenting a flexibility nobody uses, plus a discriminant on every
access. (A borrowing fast path for escape-free names remains possible behind the same
enum later, if a measurement ever names names; on this document they are noise against
21 M numbers and operators.) An internal scratch buffer reused across tokens was rejected
because a token borrowing `&mut self` forbids holding two tokens at once, and `cmap.rs`
and `tounicode.rs` hold whole sections; that design is a caller rewrite, not an API
change. The blast radius of the chosen design is wide but shallow: every
`Token::Keyword(b"x".to_vec())` comparison became `Token::Keyword(b"x")` — *simpler* at
every site — and the type annotations gained a `<'_>`.

## Decision 2: §7.3.3's two forms are parsed from their bytes, exactly

The second candidate, taken because the first landed cleanly. Two parts:

**The `String` detour is gone.** `read_number` parsed by collecting the run into a
`String` byte by byte — a second allocation per numeric token — and now parses the
borrowed bytes in place (`str::from_utf8` view for the fallback path, and a byte-wise
`salvage_number`, which walks the same prefix the `char`-wise one walked because every
byte it keeps is ASCII).

**A fixed-format fast path, in front of `from_str`, parsing exactly the grammar §7.3.3
states.** The clause writes both forms:

> An integer shall be written as one or more decimal digits optionally preceded by a sign.

> A real value shall be written as one or more decimal digits with an optional sign and a
> leading, trailing, or embedded PERIOD (2Eh) (decimal point).

`fixed_format_number` accepts precisely that — one optional leading sign, decimal digits,
at most one period, nothing else — and refuses everything outside it to the standing
`from_str`-then-salvage path, so exponent forms, over-long digit runs and every malformed
shape keep their previous reading byte for byte. That is what makes it principle-5 clean:
the fast path *is* the clause's grammar, not a heuristic over it.

**Why the fast arithmetic is exact rather than approximate.** The digits accumulate into
an integer mantissa `m`, bounded to at most 15 digits (else: fallback), and the value is
`m / 10^f` for `f` fractional digits. `m < 10^15 < 2^53`, so `m` is exactly representable
in an `f64`; `10^f` with `f ≤ 15 < 23` is exactly representable; and IEEE 754 division
rounds its exact quotient once, to nearest — so the result is the correctly rounded value
of the decimal the bytes state, which is the same value `f64::from_str` is specified to
return. Not approximately: bit for bit, which
`lexer.rs::the_fixed_format_parse_agrees_with_the_standard_library` asserts over every
digit string to five digits in every sign/point position, and the readback gate asserts
end to end. (This is Clinger's classic exact case; the standard library's parser spends
its cost being correct for the general case a PDF number never uses.)

## What it buys — callgrind, three arms from one tree, `RAYON_NUM_THREADS=1`

The habit's rule twice over: the pool is pinned because callgrind counts every thread and
rayon's steal spin drowned a smaller change than this one once before (ADR 0335), and all
three arms were built and run in one sitting. The instruments are ADR 0332's
(`callgrind_interpret` on the owner's document) and ADR 0330's exact `find_cost`
invocation (a corpus-normal cold sweep, ISO 32000-2's 1023 pages) — the outlier and the
normal path, because a win on the outlier must not cost the normal path. The before arm
reproduces ADR 0332's attribution: 22 398 M against that session's 22 411 M, 0.06% apart,
which is the check that the arms are comparable at all.

| `callgrind_interpret tmp/Entwurf.pdf 1` | instructions | vs before |
|---|---|---|
| before | 22 397 708 918 | |
| Decision 1 (borrowed token) | 17 045 615 941 | **−23.9%** |
| Decisions 1+2 (number parsed in place) | **13 487 421 385** | **−39.8%** |

| `find_cost ISO_32000-2 zzzqqqxyzzy 0 split 100000` | instructions | vs before |
|---|---|---|
| before | 37 180 286 243 | |
| Decision 1 | 35 582 613 482 | −4.30% |
| Decisions 1+2 | **34 961 497 466** | **−5.97%** |

Where it came from, on the owner's document, inclusive unless said otherwise:

| | before | Decision 1 | Decisions 1+2 |
|---|---|---|---|
| `Lexer::next_token` | 14 257 M (63.7%) | 9 074 M (53.2%) | **5 516 M (40.9%)** |
| `<f64 as FromStr>::from_str` | 3 379 M (15.1%) | 3 379 M | below the profile's threshold |
| `Lexer::read_regular_run` | 3 229 M (14.4%) | 1 836 M | 1 836 M |
| `malloc`, self | 1 187 M (5.3%) | 186 M | 186 M |
| `free`, self | 1 651 M (7.4%) | 370 M | 370 M |

The todo priced the two candidates at "up to a fifth" and "up to 15%"; they delivered
23.9% and a further 15.9 points. The corpus-normal sweep — whose profile is mostly not
lexing (ADR 0335: `interpret_with` at 93.5%, spread across fonts, filters and drawing) —
improves 5.97% rather than paying: nothing on the normal path was spent on the
outlier's win.

## Readback is byte-identical, which is the gate

The standing method, ADR 0335's, on both documents and all three arms:

- `readback` over all 1023 pages of ISO 32000-2, concatenated: 2 730 201 bytes,
  SHA-256 `ed074b1c00292534…` on every arm — the same digest ADR 0335 recorded, so the
  readback has not moved since either.
- `readback` on the owner's document: 67 bytes,
  SHA-256 `dbc3e0e78ce41b2d…` on every arm.
- `find_cost`'s own split line agrees at 2 658 697 bytes on every arm, and
  `callgrind_interpret` prints the same 58 009 commands on every arm.

## What it costs

- **A lifetime parameter on a public type.** `Token<'a>` now names the input it borrows
  from, which is the truth the old type hid behind a copy. Callers that held a token
  while replacing the buffer it came from could no longer compile — and the survey found
  none; the borrow checker now enforces what was previously an unstated obligation.
- **`fixed_format_number` is 60 lines where `parse::<f64>()` was one**, carrying its
  exactness argument as a comment. `CLAUDE.md`'s tension rule prices this: a hot path
  measured at 15.1% of a real document's interpretation, justified by the table above,
  explained beside the code.
- **Nothing on memory**: the change removes allocations and adds none.

## Fuzzing

Both `pdf-syntax` targets ran clean on the changed lexer:
`cargo +nightly fuzz run lexer -- -runs=50000` — 50 000 runs, coverage 319 edges, no
crashes, the target's own progress assertion (the cursor advances on every token)
holding throughout — and `cargo +nightly fuzz run object -- -runs=50000` — 50 000 runs,
coverage 529 edges, no crashes.

## Consequences

- Both of `doc/todo/44` §2's candidates are **taken**; what remains of that file is §3's
  encode-cache half, which is an upstream ask first.
- The owner's ten-second draft loses a large piece of its largest component: the ~7.0 s of
  interpretation was 63.6% lexing, and the lexer's two measured costs — the allocation per
  token and the library float parse — are gone from it, 39.8% of the whole. What remains
  on that document is real work (`points_from`/`numbers_from` operand marshalling, the
  one-time inflate, the interpreter's own dispatch, and the lexer's residual 40.9%), each
  already named in ADR 0332's table or this one's.
- Any future caller that wants to hold token bytes beyond the input's life now states
  that by copying, visibly, instead of receiving a copy it may not have needed.
