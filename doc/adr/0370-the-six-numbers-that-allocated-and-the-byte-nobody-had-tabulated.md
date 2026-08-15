# ADR 0370 — The six numbers that allocated, and the byte nobody had tabulated

Status: accepted, 2026-08-15. Session 535. Retakes `doc/todo/44` §2's attribution — stale since
ADR 0341 halved the lexer and ADR 0365 changed the window the stream arrives through — and takes
what the new one names, in the order it names it.

## Why the attribution had to be retaken

ADR 0332 attributed the owner's document (`tmp/Entwurf.pdf`, one page, a content stream inflating
to 141.12 MiB, 20 834 587 lexer tokens, 3 185 295 operators, 58 009 display commands) at 22 411 M
instructions and put `Lexer::next_token` at 63.6% of it. Two rounds have moved that tree since:
ADR 0341 took the token's allocation and the library float parse (−39.8%), and ADR 0365 put the
page's `/Contents` behind a 64 KiB window, which cost this document +10.08% for 187 MB of peak
resident memory. A profile three rounds old cannot say what to optimise, and the owner's launch
table still names interpretation as the largest single item on this document's path — measured on
this tree at `interpreted, 58009 cmd (+3991.8 ms)` of a 6581 ms first present.

## The attribution, retaken

`valgrind --tool=callgrind`, `RAYON_NUM_THREADS=1` (ADR 0335's rule — callgrind counts every
thread and a work-stealing pool's spin is not deterministic), over
`examples/callgrind_interpret tmp/Entwurf.pdf 1` built `--profile gates`. One `Document::open`
plus one `interpret` of page one: **14 278 677 597** instructions, which reproduces ADR 0365's
own after-arm figure to **0.004%** and is the check that the arms are comparable at all.

Inclusive shares of the total:

| function, inclusive | Ir | share |
|---|---|---|
| `pdf_model::content::interpret` | 14 252 M | 99.82% |
| `Interpreter::run_reader` | 13 751 M | 96.31% |
| **`pdf_syntax::lexer::Lexer::next_token`** | **5 520 M** | **38.66%** |
| — of which `Lexer::read_regular_run` | 1 836 M | 12.86% |
| **`content::run::points_from`** | **2 593 M** | **18.16%** |
| — of which `Vec<f32>::from_iter` (`numbers_from`'s `collect`) | 1 714 M | 12.00% |
| — of which `Vec<Point>::from_iter` | 404 M | 2.83% |
| `Window::settle_then` → `Pump::pump` → flate | 2 401 M | 16.82% |
| — of which `zlib_rs::inflate_fast_help_avx2` | 2 209 M | 15.47% |
| `run_reader::{closure#0}` (token → operand) | 1 428 M | 10.00% |
| `RawVec::do_reserve_and_handle` → `realloc` | 1 043 M | 7.30% |
| `token_to_object` | 512 M | 3.58% |
| `Parser::parse_stream_data` (the open, once) | 497 M | 3.48% |
| allocator, self (`malloc` + `free` + `realloc`) | ~1 669 M | ~11.69% |

**The lexer's share moved for exactly one of the two reasons available, and the numbers say
which.** ADR 0341 left `next_token` at 5 516 M; it is **5 520 M** here — 0.07% apart, four
rounds and one window later. So its *fall* from 40.9% to 38.66% is entirely the denominator
growing, and ADR 0365's per-token bookkeeping is a separate item rather than a lexer cost: it is
in `run_reader`'s own 1 342 M of self instructions. That is ADR 0365's own claim
("`Lexer::next_token` and `read_regular_run` are unchanged to five significant figures")
confirmed from the other side, on a profile taken for a different purpose.

**And the remainder is named.** Of the 14 279 M: the lexer 38.66%, path-operand marshalling
18.16%, the one-time inflate of 141 MiB 16.82%, the interpreter's own dispatch and the reader's
window bookkeeping 9.65% self, the token-to-operand closure 10.00%, `token_to_object` 3.58%, and
the document's open 3.48%. Nothing above 1% is unattributed.

## What was taken, in the order the attribution named it

### 1. §7.3.3's fixed format is asked before the digit scan — −1.20%

The lexer first. `read_number` walked a numeric run **three times**: once for
`raw.iter().any(u8::is_ascii_digit)`, once in `fixed_format_number`, and once more in the
`from_str` path the fast one almost never reaches. The first two are asked in the wrong order and
that is all: the clause states both numeric forms as "one or more decimal digits", so a run
`fixed_format_number` accepts is a run holding a digit, and the scan could only ever have agreed
with it. Asked second, it runs only when the fast path refuses — which is the malformed run it
was written for.

Identical readings by construction, not by test: the two functions' domains are ordered by the
clause, and every run either function declines still reaches the same `from_str`-then-salvage
path in the same order.

### 2. §7.2.3's classification is a table — −5.84%

`read_regular_run` was **12.86% of the whole**, about seventeen instructions for each byte of a
token that is usually four characters long. The reason is what the two `matches!` predicates
compile to: the six white-space codes fit one 64-bit mask, but the ten delimiters run from 37 to
125 and need two, so `is_regular` is a dozen instructions asked once per byte.

`class::REGULAR` is `[bool; 256]`, built in a `const` block **from the two predicates**, which
are now `const fn`. The classification is still stated exactly once — this is their answer
tabulated, not a second copy of §7.2.3's sets — and `is_regular` is a load and a test. What it
costs a reader is one indirection between the clause's sets and the predicate, which is why the
table carries the measurement in its own doc comment.

### 3. An operator's operands are read into an array, not a `Vec` — −17.43%

The largest single lever, and it was invisible until this round because ADR 0332's profile was
taken before the lexer stopped dominating. `numbers_from(operands, count)` returned a
`Vec<f32>` collected from a `filter_map`, whose lower size hint is **zero** — so `collect` began
at capacity nought and grew: a `malloc` and two `realloc`s to hold six floats, three million
times over. `points_from` then collected a second `Vec`. Together, 18.16% of interpreting the
page.

Annex A gives every operator that reaches these two a *fixed* operand count — `count_of` is that
table, and the six call sites all pass a literal — so the count is a `const` parameter and the
answer is `[f32; N]` and `[Point; N]` on the stack. `points_from` reads its pairs directly rather
than through `numbers_from`, because a const parameter cannot be doubled in a type without
`generic_const_exprs`; that is two `number_at` calls per point, which is what the two-step did
anyway minus the intermediate array.

**The reading is unchanged and the argument is the semantics of `filter_map`.** The old form
collected the operands that parsed and then required `values.len() == count`, so one failure gave
`None`; the new form returns `None` at the first failure. Same answer, one branch earlier.

### What the three cost together

`callgrind_interpret tmp/Entwurf.pdf 1`, four arms built and run in one sitting from one tree:

| arm | instructions | vs baseline |
|---|---|---|
| baseline (`e2bc9f1`) | 14 278 677 597 | |
| + operand arrays (§3) | 11 789 520 532 | **−17.43%** |
| + fixed format asked first (§1) | 11 648 309 492 | −18.42% |
| + the class table (§2) | **10 968 060 675** | **−23.19%** |

And the corpus-normal control, `callgrind_interpret` with no arguments — ISO 32000-2 page 101,
interpreted fifty times, the page ADR 0365 used for the same purpose:

| | instructions | |
|---|---|---|
| before | 1 255 981 925 | |
| after | **1 235 038 472** | **−1.67%** |

So the win on a 58 000-command page is not paid for by an ordinary one: the ordinary one improves
too, by less, which is what the shape predicts — a page whose profile is mostly fonts, filters and
drawing has fewer path operators and fewer tokens to classify.

Where it came from, on the owner's document, self instructions:

| | before | after |
|---|---|---|
| `Lexer::next_token` (with `read_regular_run` inlined into it after §2) | 5 520 M | **4 698 M** |
| `points_from` + its two `from_iter`s + `do_reserve_and_handle` | ~2 593 M | **0** |
| `zlib_rs::inflate_fast_help_avx2` | 2 209 M | 2 209 M (unmoved, as it must be) |
| `Interpreter::run_reader`, self | 1 342 M | 1 342 M |
| `run_reader::{closure#0}`, self | 865 M | 865 M |

### And what the launch table says, which is structure rather than a stopwatch

The release binary on the witness under `Xvfb` on the software adapter, one launch an arm, the
before arm built from this round's own patch applied in reverse. **The machine is shared and one
sample is not a benchmark**; what it establishes is that the step that moved is the step the
instruction counter says moved, and that no other step moved with it:

| `--trace`, ms from process start | before | after |
|---|---:|---:|
| `document joined` | 72.667 | 81.457 |
| **`interpreted, 58009 cmd`** | 1015.910 (**+943.243**) | 789.309 (**+707.852**) |
| `first scene built` | 1307.482 (+291.572) | 1061.932 (+272.623) |
| `first present` | 1902.297 (+594.815) | 1715.347 (+653.415) |

The `interpreted` step falls 25.0% where the counter says 23.19%, and `scene`, `device` and the
frame's own 58 029 uploads are the same work on both arms — as a change that builds the same
display list requires.

## What was declined, each with its number

`doc/todo/41`'s precedent: a lever measured and declined is a result.

- **Scanning the regular run over the slice instead of one `peek` at a time** — built, measured,
  **+2.64%** against the arm without it (11 955 814 274 against 11 648 309 492), with
  `read_regular_run` itself 1.1% *worse*. The `peek` loop's per-byte bounds check is what a slice
  iterator was supposed to remove, and LLVM had already removed it; what the rewrite added was a
  second cursor and a closure the optimiser then had to re-fuse. Reverted. **The suspect that
  looked like the cost was the classification beside it, not the loop** — which is why §2 pays
  five times what this was expected to.
- **§7.8.2's operand slicing** — the prompt's suspect, measured by removing it:
  `operands_before` returning everything unconditionally, so `count_of`'s second match over the
  operator bytes never runs, is **10 865 831 554** against **10 968 060 675** — **0.93%**. That is
  the price of a normative sentence ADR 0302 proved load-bearing on a real document, and it is not
  a price worth reopening for 0.93%. Not taken; the probe was reverted.
- **The graphics-state clone per `q`** — `GraphicsState::clone` is **1 856 Ir** on this page,
  0.00001%. This stream nests almost nothing.
- **The resource-lookup path** — `Interpreter::resource_entry` is **334 Ir**. ADR 0332 said
  "resource lookups are nowhere"; four rounds later it is still true, and now it is true of a
  profile with the lexer cut in half.
- **The per-command allocation in building 58 009 display commands** —
  `RawVec<Command>::grow_one` is **237 613 Ir**, 0.002%. A doubling vector reaching 58 009
  elements reallocates seventeen times, which is seventeen and not 58 009. `end_path` is 69 M
  (0.48%) and `RawVec<PathCommand>::grow_one` 101 M (0.71%); neither is worth a capacity guess.
- **The reader's own per-token bookkeeping**, ADR 0365's deliberate cost, is the largest remaining
  item this round did not touch: `run_reader`'s 1 342 M of self instructions, 12.24% of what is
  left. It buys 187 MB of peak resident memory on this document and turns a decompression bomb
  from an allocation into a bounded loop, and ADR 0365 already tried three shapes of it. Left
  alone deliberately, and named here so the next round does not have to find it again.

## The display list is byte-identical, which is the gate

`CLAUDE.md` rule 1 makes interpretation a pure function of the document and the view state, so a
round that changes how a display list is *built* owes the artefact rather than a verdict. Both
arms run in one sitting, the before arm taken with `git apply -R` of this round's own patch —
never `git stash`, which is shared between worktrees:

- **`examples/display_list_digest` over every pdf.js corpus document's page one**: 975 lines,
  SHA-256 `3d82288fdf0114ff…` on both arms — and the same digest ADR 0365 recorded five rounds
  ago.

  **And the same comparison run a second time moved, which is worth the paragraph because the
  cause was the instrument.** `575f9fd730566651…` on the later pair, both arms again, with 106
  documents differing between the two pairs — every one of them a JBIG2 or JPEG 2000 carrier. The
  variable is `pdf-sandbox-worker`: the first pair ran before `doc/todo/02` §2's
  `cargo build -p pdf-sandbox --bins` line had put one beside the test binary, so those images
  were refused and their pages held no command; the second pair ran after. **A digest of a
  display list is a digest of what the program could decode**, and this project's own answer to a
  missing worker is a refusal rather than an in-process fallback — so the artefact moves with the
  directory as well as with the code. Both pairs agree arm to arm, which is the claim; a
  comparison across the two would have read as 106 documents changed.
- **`examples/readback` over all 1023 pages of ISO 32000-2**, concatenated: **2 730 201 bytes**,
  SHA-256 `ed074b1c00292534…` on both arms — the figure unmoved since session 500.
- **`examples/readback` on the owner's document**: 67 bytes, SHA-256 `dbc3e0e78ce41b2d…` on both
  arms, ADR 0341's figure.
- `callgrind_interpret` prints **58009** commands on every one of the four arms.

Every gate in `doc/todo/02` §2 was run and every one is identical to the before arm, as a change
to how a list is built and not to what it contains requires.

## Fuzzing

Both `pdf-syntax` targets and the one that reaches the interpreter ran clean on the changed code:
`cargo +nightly fuzz run lexer -- -runs=50000`, `object -- -runs=50000` and
`page -- -runs=20000`, no crashes in any of the three. `page` is the one that matters for §3 —
ADR 0264 put `pdf_model::interpret` in it precisely so that a change to the operator loop has a
target — and it is run seeded, as `doc/verify.md` requires.

## What it costs

- **A `const` type parameter at six call sites** — `numbers_from::<3>(operands)` where it read
  `numbers_from(operands, 3)`. The count was already a literal at all six; what changed is which
  side of the compiler reads it.
- **A 256-byte table in the binary**, and one indirection between §7.2.3's sets and the predicate
  that answers about them. The table is derived from the predicates in a `const` block rather
  than written out, so there is nothing to keep in step.
- **`is_whitespace` and `is_delimiter` are `const fn`**, which is a widening and breaks nothing.
- **Nothing on memory**: three allocations per path operator become none, and the table is
  static.

## Consequences

- `doc/todo/44` §2's attribution is replaced by this one, and the file records what each of the
  round's three levers and five declines was worth.
- **The lexer is still the largest item and is now 42.84% of a smaller total.** What is under it
  is `read_regular_run`'s scan, `fixed_format_number`'s digit loop and the token's own
  construction — none with an obvious lever left, and the one this round tried made it worse.
- **The next largest is not a defect**: the 141 MiB inflate, at 20.14%, is the work the document
  asks for, done once.
