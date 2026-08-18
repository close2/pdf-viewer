# 589 — The second walk over every number a page states

**Finding:** the largest step of the owner's launch is interpretation, 40% of interpretation is the
lexer, and 85% of that page's tokens are numbers — each of which was walked twice, once to find
where the run ends and once to read its value. Fusing the two is 5.4% of the page; and the same
loop written with a slice iterator instead of an index is **750 M instructions worse**, which is
6.5% of the page hiding in the spelling rather than in the algorithm.

Date: 2026-08-18. ADR: [0424](../adr/0424-the-second-walk-over-every-number-a-page-states.md).

Touched: `crates/pdf-syntax/src/lexer.rs`, `crates/pdf-syntax/src/parser.rs`,
`crates/pdf-model/tests/annotations.rs`, `doc/conformance/ledger.toml`, `doc/performance.md`.

## The measurement was re-taken before anything was changed

ADR 0423's 40.2% and 23.1% both reproduce on this round's own binaries: 11 470 896 205 instructions
to interpret page one of `tmp/Entwurf.pdf`, of which `Lexer::next_token` is 40.13% and zlib-rs
21.41%. The division under those two is the round's result and is in the ADR.

## What is worth remembering, beyond this document

- **A fused loop's spelling can cost more than the fusion saves.** The first version of the change
  measured a *regression*, and the profile put 454 M of it in code with no source line. Removing the
  suspect rather than reading the profile is what found it: capping the slice made it worse, so the
  slice was not the cause, and rewriting the same loop with an index instead of a slice iterator was
  worth 750 M. ADR 0370 recorded the mirror image and this is the second instance.
- **"Is this dependency asking for more work than it needs" is answered by a call count, not by a
  ratio.** zlib-rs's 21% looked like the second-largest lever in the program; it is one pass over the
  stream in 2 409 refills, and there is nothing to memoise. What *was* wasteful sat on either side of
  it — a nine-byte-window search and a stream copied twice — and neither is a decompression.
- **A negative on `memchr` is a decision worth writing down.** It would take the `endstream` search
  to about a tenth of an instruction a byte and it is `unsafe` SIMD fed a hostile file's bytes in the
  crate `CLAUDE.md` most wants `#![forbid(unsafe_code)]` on. Recorded so the next round reading the
  profile does not re-open it by accident.
- **I overwrote a committed instrument with a scratch one of the same name.** `examples/display_list_
  digest` already existed and does exactly what the round needed; `git status` showing ` M` where
  `??` was expected is what caught it. **Before writing an example, `ls` the examples directory** —
  this tree has eighty of them and the one you are about to write is often there.

## The spec-driven half

§12.5.6.11's caret row was `reported` with no `code` and no `test`, and its note said the symbol "is
not stated anywhere". Table 183 states half of it — "P A new paragraph symbol (¶) shall be
associated with the caret", a `shall` and a code point, which is more than §12.5.6.4's seven
mandatory icons get. The refusal is unchanged and its reason is better: `/RD`'s own sentence puts the
pilcrow "along with the caret", so it is additive and the caret is what cannot be derived. The row
now names the code and a test in three shapes.

## Gates

Every gate in `doc/todo/02` §2 was run after the last edit and none moved, plus the strong claim a
lexer change owes: `examples/display_list_digest` over 1187 documents, 1178 first pages, both arms
built in one sitting — the two files `diff` empty. `tools/state.sh` prints every number.
