# 800 — §7.3.3's numeric forms, and what the lexer actually admits

Date: 2026-08-28. Branch `round-800`, from `main` at `46289075`.
ADR: [0733](../adr/0733-what-the-lexer-admits-that-the-clause-does-not.md).
Touched: `crates/pdf-syntax/src/lexer.rs`, `crates/pdf-model/src/function.rs`,
`doc/conformance/ledger.toml` (§7.3.3, §7.10.5.2), `doc/traps/parsers-and-streams.md`, and two
new files — `doc/adr/0733` and `crates/pdf-model/examples/numeric_form_census.rs`.

## The finding, in one sentence

Round 796's note was right and was the smaller half: the lexer reads §7.3.3's forbidden
exponential form, which the clause forbids the **writer** and says nothing to a reader about — so
it stays, now stated where it is granted — and beside it, on the same line, a magnitude no double
can hold was read as **zero**, which is a *conforming* number silently replaced by the smallest
one available.

## What was read

§7.3.3, and the grammar behind it. The two sentences state the forms; **Errata Collection 3's
Issue #327 closes them**, adding a railroad diagram above each of the clause's EXAMPLEs — an
optional sign, decimal digits, one PERIOD for the real form, and no other production in either
figure. Neither errata instrument put that in front of the round: `check` compares quotations
against struck text and #327 struck nothing, and `emit` files it on **page 1004** under a heading
of its own, as two `FileAttachment` annotations whose printable content is their titles. The
figures are images; they were decoded out of `doc/md/` and looked at.

The prohibition — "A PDF writer shall not use the PostScript language syntax for numbers with
non-decimal radices (such as 16#FFFE) or in exponential format (such as 6.02E23)" — binds the
**writer**, and the same sentence glosses the value it forbids the spelling of. Annex C, which
§7.3.3 points at for the ranges, is *informative* and gives no figure at all: Table C.1 says only
that integers "can often be expressed within 32 bits" and that reals are often IEEE 754 single or
double.

## What the lexer admits, by experiment

Enumerated by running it rather than by reading `str::from_str`'s documentation: the exponent in
every spelling (`1e2`, `1E2`, `6.02e23`, `1e-2`, `1e+2`, `.5e1`, `1.e2`); `inf`, `Inf`,
`infinity` and `NaN` **not at all** — they hold no decimal digit and ADR 0303's condition returns
each as the keyword it lexically is; no underscores, no radix prefix, `1e` salvaged to 1. So the
exponent is the *only* Rust-ism that reaches a value, which is narrower than "we use the standard
library's parser" suggests.

And, the finding 796's note did not contain: `1e400` **and four hundred decimal digits**, which
is a perfectly conforming §7.3.3 integer, both came back as `Integer(0)`.

## What changed

- **The exponent stays and is now stated in `read_number`**, with the clause quoted and the party
  it binds named. Refusing would add a requirement no sentence places on a reader and would lose a
  mark; reporting would be false under `Unsupported`'s own definition, which is trap 11's test and
  ADR 0152's price. The instrument that replaces the report is the new census.
- **A magnitude beyond the representation is the representation's limit**, signed, rather than
  zero. Zero is the smallest magnitude where the largest was written, it inverts every comparison
  the number takes part in, and — unlike a refusal — it draws.
- **`function::compile_token` no longer compiles `{ inf }` to a pushed infinity.** §7.10.5.2 sends
  the operand syntax to §7.3.3, whose forms are decimal digits, and Table 42 names no such
  operator, so the token is neither and the function is refused and reported. A magnitude past
  `f32` is bounded there for the same reason it is here.

## The neighbours, swept

Every `parse::<…>` and `from_str` in `pdf-syntax`, `pdf-model` and `pdf-font` read against the §7
grammar it stands for. **All but the two above already gate the grammar themselves** — §7.9.4's
date fields, `startxref`, §12.3.5.2's folder key, `Hn`, and `fragment.rs::number`, which spells
§7.3.3 out and refuses the exponent *deliberately* because "[n]othing has to draw here". §7.3.4.3's
hexadecimal strings and §7.3.5's `#`-escapes have their own `hex_value` and never see a Rust parse.
Nothing is left named-but-unfixed.

## The instrument

`pdf-model/examples/numeric_form_census` splits every regular run in every stream a lexer is ever
pointed at — page `/Contents`, form `XObject`s, CMaps, calculator functions, §7.5.7's object
streams — into six forms, classified against §7.3.3's grammar written out in the example rather
than by asking the lexer (trap 8). Its first draft scanned the whole file and reported 51 956
"non-decimal radix" runs, every one a byte of compressed image; the population is the point of it.

## What it cost, measured

The exact figures are the run's; what the reading found is that **the exponential form is real and
vanishingly rare** — one run in the whole pdf.js corpus, `sci-notation.pdf`'s `/F1 1e2 Tf`, and a
handful more in an 8300-document sample of the crawl, all inside `govdocs1-error-pdfs` — and that
**no document anywhere states a number in the clause's own forms that overflows a double**.
`display_list_digest` over all 974 first pages is byte-identical across the change, which is what
makes this a defect nothing could have found by counting.

## Trap 13

Four new tests, each calibrated in both directions above a saved copy of the file and then
restored byte-identically (`md5sum` on both). The code-side plants: the old `0.0` returned for a
non-finite value; a refusal of any run containing `e` or `E`; the digit guard replaced by `if
false` in each of the two files. The test-side plants: `Integer(0)` expected for four hundred
nines, `Real(1.0)` for `1e2`, `Real(0.0)` for `inf`, `Real(inf)` for `1e40` in a function. Every
one failed as intended.

## Gates

Run whole, in the worktree, with sibling rounds 799, 801 and 802 live on the same machine — load
average passed 60 during the run — which `doc/todo/02` §2 says inflates every line that spawns a
reference renderer. The wall clocks are of a loaded machine; the verdicts are not.

| line | what it printed |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | clean |
| `cargo nextest run --workspace` | 2759 tests run, 2759 passed, 18 skipped |
| `cargo test --workspace --doc` | green |
| `cargo check --manifest-path fuzz/Cargo.toml --bins` | clean |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow |
| oracle | 1945 pages: 983 agrees, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render |
| text extraction | 10971/11163 matched words in bounds (98.28%), 487 of 508 documents fully in bounds |
| selection census | 1000/1011 words selected (98.91%) over 453 documents |
| accessibility census | 102 853 elements, 7413 clickable, 57 116 a caret moves through |
| dates | 1545 date strings, 1514 conform to §7.9.4 (97.99%) |
| XMP, JPEG 2000 | green |
| quorra corpus | 957 pages: 932 agree, 22 differ, 3 refused, 17 not comparable; median 2.24× the CPU backend |
| fixed documents | 41 checked, 0 absent, 41 rows |
| `cargo test -p conformance` | green |

**Every verdict is identical to the round before this one**, which is the claim the change makes:
`display_list_digest` over all 974 first pages is byte-identical across it, so nothing the corpus
holds reaches either of the two behaviours that moved.

### Fuzzing

`fuzz/` is not in the workspace, so these are separate. The lexer is what changed, and it got the
long run: **50 000 000 executions in 792 s**, no crash, no timeout, no leak. Beside it, **`object`
at 5 000 000** (§7.3's object grammar, 162 s) and **`document` at 2 000 000** (§7.5's file
structure, 9 s), both clean. `cargo clippy --all-targets` inside `fuzz/` is clean.

**`page` is the only target that reaches `function::compile_token`, and the first attempt at it
measured nothing** — a fresh worktree gets an empty `fuzz/corpus/`, so libFuzzer ran 86 912
iterations and reported `cov: 0 ft: 0 corp: 0`, which is `doc/verify.md`'s warning arriving as an
exit status of zero. Seeded from the pdf.js and `doc/corpora` populations with `fuzz/seed_page.py`
(1153 seeds, 33 MB) it reports **35 249 covered edges over 21 901 executions in fork mode, 0
oom/timeout/crash**. The lesson generalises past this round: **a fuzz target's exit status says
nothing about whether it fuzzed anything**, and `cov:`/`corp:` on its last line are what to read.

**One pre-existing finding, not this round's and not fixed here**: `rustfmt --edition 2024 --check
fuzz/fuzz_targets/*.rs` — which `cargo fmt --all` does not reach — reports two files unformatted,
`display_list.rs` (an `assert!` on one line) and `x509.rs` (a `use` out of order). Both are on
`main` and neither is in a file this round touched; `fuzz/confined_wire.rs` is a parallel round's
subject, so the whole directory was left alone.

## Sweeps

§4's sweeps over `crates/`, `tools/`, `fuzz/`, `doc/adr/` and the ledger, before against pristine
`main` at `46289075` in this same worktree and after. **No finding appeared and none
disappeared**; every delta is accounted:

- `blockers`, `callers`, `entries`, `overstated`, `parts`, `retired`, `unread` — identical.
- `capabilities` — one line number moved under the edit to `function.rs`.
- `counts`, `tables`, `pointers`, `quotations` — the populations grew by this round's own
  sentences and files. `quotations` gains no *diverging* quotation, which is the number that would
  have meant a misquotation, and the conformance gate agrees.
- `inapplicable`, `owed` — every changed line is a "named by N file(s)" count rising by one for the
  new example, or §7.3.3 entering a "cousin row(s)" list because its note now names more words.
- `overtaken` — one more page-list note and one more decision record, both this round's. It first
  reported **59** notes citing no ADR against a baseline of 58, because the census's `NAMES`
  partition has the shape the sweep's population is defined by; citing ADR 0733 in that doc
  comment is the sweep's own rule and took it back to 58.

## One thing this round got wrong, recorded because a sibling paid for it

Stopping its own gate run, this round ran `pkill -x cargo`. `doc/environment.md` names `pkill -x`
as the *safe* form and it is not — not for a program every round runs under its own name.
`-x` bounds the match to the executable's name and says nothing about whose process it is, so it
took **every** `cargo` on the machine, including round 799's mid-gate build, which was then
observed rebuilding its `gates` profile from near scratch. The rule that survives is the one in
the same paragraph and it is the stronger half: **kill by PID, from your own process group** —
`kill -- -$(ps -o pgid= -p "$pid")` against the pid of the script this round started. A pattern is
a namespace shared with every parallel round, and `-x` narrows the *pattern* rather than the
namespace.

The same shape caught this round twice more, harmlessly: a `pgrep -f 800-sweeps.sh` wait-loop
matched **its own command line** and reported the sweep still running after it had finished.
