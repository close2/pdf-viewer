# ADR 0306 — A bound that counts the wrong thing, and a clamp that said nothing

Status: accepted, 2026-08-13. Session 471. Carries out `doc/todo/10` §3's three defects and
corrects the unit ADR 0271, `doc/todo/49`, `doc/performance.md`, `doc/todo/03` and §7.8.2's ledger
row all state. **Decides none of `doc/todo/10` §5**: no deadline, no callback, no streaming lexer,
no boundary message, and the confinement is not shipped. Amends §7.4, §7.4.3, §7.4.4.1, §7.4.4.2,
§7.7.3.3 and §7.8.2's rows.

## The three defects were one sentence apart

`doc/todo/10` §3 lists them separately and they are one rule with three violations:

> **A bound must count what its name says, and a bound that refuses must say so.**

1. `MAX_OPERATIONS` said *operators* and counted lexer *tokens*.
2. The Flate and LZW length guard refused and said nothing, handing back the prefix it had
   decoded as though it were the whole stream.
3. `max_stream_len` bounded one stream against a ceiling twice too small for it, and the total of
   a page's `/Contents` parts was bounded by nothing at all.

Each is fixed below with a measurement rather than a new constant. **One of the three values moves,
and it is the only one whose old value contradicted a number this tree already had.**

## 1. Seven tokens is one operator, and §7.8.2 says so

`Interpreter::run` incremented its counter at the top of `while let Some(token) = lexer.next_token()`
— before the operand/operator distinction is made. §7.8.2 puts the operator last:

> In PDF, all of the operands needed by an operator shall immediately precede that operator.

So `x1 y1 x2 y2 x3 y3 c` is **seven tokens and one operator**, and a page of cubic Béziers spent its
budget about seven times faster than the constant advertised. The increment now sits after the
keyword arm and before the dispatch, which is the only place in the loop where the interpreter knows
it has an operator.

**The value does not move, and that is the finding rather than a decision deferred.**
`doc/todo/10` §6 forbids replacing an arbitrary number with another arbitrary number; correcting the
*unit* is what four million always claimed to mean. What it costs is measured, not estimated —
`crates/pdf-model/examples/content_budget_census.rs` counts both quantities in one pass, with the
operator rule written out again so that the census is not the code under test (trap 8):

| population | pages | past 4 M **tokens** | past 4 M **operators** | tokens per operator |
|---|---|---|---|---|
| 65 967 crawled documents | 926 680 | 48 | **8** | 3.76 |
| pdf.js + the four submodule corpora + `doc/` | 10 809 | 0 | 0 | 3.08 |

Forty pages of nine hundred thousand stop being refused; eight remain, and those are programs of
that length. **The ratio is not a constant** — about 2 for text, about 7 for Bézier artwork — which
is exactly why a token count could not stand in for an operator count at any value.

**Why no test saw it.** `hostile_budgets.rs` built its fixture from `"n\n".repeat(4_000_002)`,
deliberately a zero-operand operator "so this measures the bound rather than the operator". That is
the one input shape where the two numbers are equal. Every fixture there now states operands, and
the control that discriminates — 4.4 million tokens, 1.1 million operators, drawn — is
`a_stream_of_many_tokens_and_few_operators_still_draws`.

### The witness

`tmp/Entwurf.pdf` is the project owner's and is not in this repository, so no test names it and the
rule is pinned by generated fixtures instead. It is 49 679 512 bytes, one page, a hand-drawn
geological cross-section traced to Bézier vectors: 3 185 295 operators in 20 834 587 tokens.

| | before | after |
|---|---|---|
| `pdf-retrieve page … 0` | `LimitReached { limit: "MAX_OPERATIONS" }`, 0.54 s, 380 MB | **`complete: true`, `unsupported: []`**, 1.36 s, 380 MB |
| `render_at … 1 1.0` | 0.62 s, 381 MB, **7.99%** of the raster inked | 1.54–1.59 s (five samples), 381 MB, **34.64%** inked |
| `mutool draw -r 72` | 2.08–2.31 s, 97 MB, 34.88% inked | — |
| `pdftoppm -r 72` | 3.38–3.53 s, 19–20 MB, 34.38% inked | — |

The three renders agree about how much ink the page states to within a quarter of a percentage
point, which is the check that matters: the page now draws *whole*, not merely *more*. We are the
fastest of the three at this resolution and **the least frugal by a factor of four to twenty**, and
that is written down here rather than left out — `doc/todo/10` keeps it.

## 2. `io::Take` returns `Ok` at its limit

```rust
flate2::read::ZlibDecoder::new(data)
    .take(limits.max_stream_len as u64)
    .read_to_end(&mut out)
```

`Take` yields end-of-file at its limit and `read_to_end` reports end-of-file as `Ok`, so a
decompression bomb came back as **a complete decode of its own first two gibibytes**, with nothing
reported. `LZWDecode`'s guard did the same thing explicitly: `(!out.is_empty()).then(…)`.
`ASCII85Decode` and `RunLengthDecode` refused properly, and that inconsistency is what made the
other two findable.

The two cases one code path was serving are **opposite statements about the same bytes**:

- **A damaged stream has given everything it had.** Keeping the prefix is right and stays —
  "a partially-inflated content stream still renders most of a page".
- **A stream past the bound has a great deal more to give**, and this reader declined to take it.
  That is a refusal, and trap 5 says a refusal is loud.

So the ceiling handed to `take` is **one byte past** the bound: reaching that byte is what
distinguishes the two, and it costs one byte of memory to know. `filter::FilterRefusal` is the
three answers — `Unsupported`, `Corrupt`, `TooLarge { limit }` — `Document::decoded_stream_data_reported`
carries them up as `StreamRefusal`, and `Page::content_with_report` turns the third into
`ContentIssue::TooLarge`. The byte-returning entry points are unchanged and still answer `None`.

**A third guard had the same hole and was found by asserting that all four agree.** `ascii85`'s
length check sat after the group push, and `z` — §7.4.3's stand-in for four zero bytes — reached it
by way of a `continue`. Eight `z` under a bound of eight produced thirty-two bytes and reported
nothing. The check is now at the top of the loop and once after it, which bounds the overshoot at
one input byte's worth of output.

**Both new tests were confirmed to fail with the defect put back**, which is the only thing that
establishes a test guards what it claims: `flate_past_the_bound_is_refused_rather_than_clamped` and
`lzw_past_the_bound_is_refused_rather_than_clamped` each fail on the old expression and pass on the
new one.

## 3. Two gibibytes against a four-gibibyte ceiling

`Limits::max_stream_len` was `1 << 31`. `pdf_sandbox`'s `INTERPRETER_ADDRESS_SPACE_LIMIT` is
`4 << 30`, of which `viewer_core`'s `MAX_PIXELS` × 4 bytes is a gibibyte of raster. **One stream
could therefore command the whole ceiling and leave nothing to draw with**, which is not a policy
disagreement but an arithmetic contradiction between two numbers in one process.

It is now `1 << 30`, and it is bounded from both sides rather than chosen:

- **From above, by the ceiling.** Decoding costs about *twice* the decoded length before the bytes
  are handed over — `read_to_end` grows a `Vec` by doubling and `Arc<[u8]>` is then a copy of it.
  Measured: 3694 MB peak for a 1.9 GB decode. A bound of L costs about 2L, and 2L has to fit in the
  3 GiB the raster leaves: L ≤ 1.5 GiB.
- **From below, by what documents contain.** The largest decoded stream in **5 047 187 streams over
  65 967 crawled documents** is 483.84 MiB, and exactly one stream in five million passes 256 MiB.
  A gibibyte is twice the largest real one.

The two sides do not overlap by much and they do not have to: the number is the largest power of two
that satisfies both.

### And the parts of `/Contents` had no total

`Page::content_with_report` concatenated every part with no aggregate bound, and `/Contents` may
hold `max_array_len` = 2²⁰ entries. **No new number was needed**, because Table 31 says what the
array is:

> If the value is an array, the effect shall be as if all of the streams in the array were
> concatenated with at least one white-space character added between the streams' data, in order,
> to form a single stream.

The array *is* one stream, so the bound one stream gets is the bound the array gets. Passing it
reports `ContentIssue::TooLarge { part: None, limit }` and stops adding parts — the parts already
read still draw, which is the same salvage decision as a truncated inflate and for the same reason.

## What the bombs did

`doc/todo/10` §2's two bombs were rebuilt from its description and came out byte-for-byte the sizes
it records — 389 317 and 1 847 467 bytes, both 1029:1 — which is what makes the comparison a
measurement rather than a memory.

| | before | after |
|---|---|---|
| **Bomb A**, 0.39 MB → 400 MB, 200 M `n` | 0.81 s, **831 MB**, `MAX_OPERATIONS` | 0.71 s, **831 MB**, `MAX_OPERATIONS` |
| **Bomb B**, 1.85 MB → 1.9 GB | 3.26 s, **3694 MB**, `MAX_OPERATIONS` | 1.18 s, **1095 MB**, `TooLarge { part: Some(0), limit: 1073741824 }` |

Bomb A is unchanged and should be: 200 million operators is 200 million operators however they are
counted, and its 400 MB is under the new bound as it was under the old one. **Bomb B loses 70% of
its peak and gains a report that names what happened**, which is the whole of items 2 and 3 in one
line of output.

**Neither number is a claim that the memory question is answered.** 1095 MB is still a gibibyte
commanded by 1.85 MB of file, and §5's road D — streaming the decompression — is the only entry in
that file that removes the allocation rather than surviving it. What this round did is make the
survivable case survivable *inside the ceiling the confined worker actually sets*, and make the
refusal audible.

## What this deliberately does not do

`doc/todo/10` §5 offers four roads and the choice is the project owner's. Nothing here forecloses
one, and the check is worth stating road by road:

- **A (a deadline and a callback)** — no clock entered `pdf-model`; the check point A would use is
  the same one, one line lower.
- **B (ship the confinement)** — `max_stream_len` now *fits* under `RLIMIT_AS` instead of
  contradicting it, which is a precondition of B rather than an obstacle to it.
- **C (a resumable job)** — `Interpreter::run` is untouched in shape; the counter moved within it.
- **D (stream the decompression)** — `FilterRefusal::TooLarge` is exactly the report a window-fed
  decoder needs when its consumer stops, and §5 D's own caveat ("a streaming rewrite that does not
  separate those two is the same bug with better memory behaviour") is the thing this round
  separated. D arrives with the distinction already made.

`interpret` is still a pure function of the document and the view state, which `doc/todo/10` §6
requires and the gates rest on.
