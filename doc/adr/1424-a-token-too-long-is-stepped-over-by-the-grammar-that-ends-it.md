# 1424 — A token too long is stepped over by the grammar that ends it, and a composite walk asks membership once

Session 1293. Status: accepted and **built**.
Context: ISO 32000-2 §7.2.3, §7.3.4.2, §7.3.4.3, §9.9 Table 124; CLAUDE.md principle 3's time
budgets; `doc/todo/10`. Code: `crates/pdf-model/src/content/reader.rs` (`ContentReader::drop_token`,
`drop_literal_string`, `drop_hexadecimal_string`), `crates/pdf-font/src/sfnt.rs` (`composite_cycle`,
`Walk`). Tests: `crates/pdf-model/tests/content_window.rs`
(`a_string_longer_than_the_window_is_stepped_over_whole`), `crates/pdf-font/src/truetype.rs`
(`a_composite_that_includes_itself_is_named_at_the_glyph_that_closes_the_cycle`,
`the_deepest_composite_chain_is_walked_in_linear_time`), `crates/pdf-sandbox/tests/confinement.rs`
(`a_decode_that_does_not_return_is_ended_at_the_deadline`). Amends ADR 0365's step over a token no
buffer holds; ADR 1411's decision about what a cycle *is* stands.

## 1. The content reader's step over a token past `CEILING`

**Found by the `page` fuzz target**, a 101 514-byte input it had left as `timeout-6af40bfb…` in
`fuzz/artifacts/page/` and that no round had read: 59 s of interpretation in a release build. The
input is a form XObject whose 14 KB of Flate inflate to 11 MB, holding one literal string of about
eight megabytes: `(h(h(h…` for three megabytes, words with spaces for six, and `) ) )…` to close it.

ADR 0365's reader reports a token longer than `CEILING` and steps over it **to the next white-space
byte**, on §7.2.3's ground that white space ends every token. It does not end a string. §7.3.4.2:
"Balanced pairs of parentheses within a string require no special treatment", so the string runs to
the right parenthesis that balances its first, white space and all; §7.3.4.3's hexadecimal string
runs to its `>` and ignores white space inside it. Stepping to white space therefore had two costs:

- **Fidelity.** The rest of the string was read as content. The regression test puts `1 1 m 2 2 l
  S` inside a string one and a half windows long: the old step drew that stroke (3 commands where
  the file states 2).
- **Time.** Every word of the string that begins with `(` opened another string this buffer cannot
  hold, each lexed and widened to `CEILING` before being refused — quadratic in the string's length,
  a minute for the fuzzer's fourteen kilobytes.

**Decision.** `drop_token` looks at the token's first byte. A `(` is stepped over by §7.3.4.2's
grammar — depth counted, a REVERSE SOLIDUS escaping the byte after it, both carried across refills
— to the balancing `)` or the end of the stream; a `<` that is not `<<` to its `>`; anything else to
white space as before. One pass over the string however many windows it spans, and one
`TokenTooLong` report. The input now interprets in 49 ms.

**What it changes on real files: nothing measured.** The census that set `CEILING`
(`examples/token_window_census`) found no token past 390.16 KiB in 39 976 documents, so the step is
never taken on the corpus; the corpus and golden walks were not re-run for that reason.

## 2. The composite walk's membership test

ADR 1411's `composite_cycle` walked the component graph once, but asked "is this glyph on the path"
by scanning the path — `on_path.iter().any(..)` per component reference. The path's depth and the
references per glyph are the file's to choose. A 6 MB program whose 65 534 glyphs form one chain,
each also naming the empty last glyph fifteen times, costs 34.5 s for two walks in the test profile;
the question is asked for every code a page shows that reached no outline.

**Decision.** One state per glyph index — unseen, on the path, finished — in a vector of 65 536, so
each reference is one lookup. The same fixture now takes well under a second. The function is
public beside `repaired_font_program`, for that function's reason: it walks a graph read out of a
document's bytes, and `fuzz/fuzz_targets/sfnt.rs` asks it directly (ADR 1423).

## 3. A JBIG2 stream the codec does not finish, and the budget that ends it

The `jbig2` target stopped after 4 069 executions on an input whose `/JBIG2Globals` symbol
dictionary `hayro-jbig2` spends minutes decoding — every sampled stack is inside its symbol
dictionary's integer and generic-region decoding — 307 s in the instrumented binary, and still past
30 s uninstrumented. A second run in fork mode (`-ignore_timeouts=1`) left six more inputs, all
stopped in the same decode. The codec is a dependency and runs confined, so **the time budget is
the parent's `REQUEST_TIMEOUT`**, and the regression test is that it holds on an input that needs
it: the confined decode is refused as `SandboxError::TimedOut` at the deadline and the replacement
worker answers the next request. The smallest of the seven, 1 624 bytes and still past the deadline
uninstrumented, is the test's inline fixture. `Isolation::InProcess` has no such bound, as its
documentation says; a bound inside the codec would be an upstream change and is not made here.
