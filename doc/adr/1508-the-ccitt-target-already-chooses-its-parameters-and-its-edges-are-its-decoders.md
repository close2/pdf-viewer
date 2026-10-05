# 1508 — The `ccitt` target already chooses its parameters, and its edges are its decoder's

Session 1336. Status: accepted; **nothing rebuilt**, by reading and measurement.
Context: ADR 1495 section 3 ("`ccitt` ended where its corpus began (223 edges, 141 M executions): its next
edges need a new seed shape"); ADR 1349 (the decoder). Code read: `fuzz/fuzz_targets/ccitt.rs`,
`fuzz/seed_streams.py` (`ccitt_head`), `crates/pdf-ccitt/src/{lib,bits,codes}.rs`.

## 1. The premise did not hold

The round was briefed that the target might feed raw stream bytes without §7.4.6 Table 11's
parameters, so that every input decoded as the default. It does not: the first six bytes of every
input choose the coding (`/K`'s three cases), `/Columns` up to 4095, `/Rows` up to 255,
`/EndOfLine`, `/EncodedByteAlign`, `/EndOfBlock` and `/DamagedRowsBeforeError`, and
`seed_streams.py ccitt` already prefixes each of the corpus's real streams with its own
`/DecodeParms` read off its dictionary. Rebuilding the target in that shape would rebuild it as it is.

## 2. What 223 is

Source coverage of the disk corpus (3740 inputs) through `cargo fuzz coverage` and `llvm-cov`
(LLVM 22.1.8, the nightly's own): `lib.rs` **93.4% of regions, 91.8% of lines**, `bits.rs` 99.4%,
the target 100%. `codes.rs` shows 0% because its tables are built by `const fn` at compile time and
nothing of it runs. Every missed line of `lib.rs` is one of three kinds: the two `Display` impls and
`Coding::from_k`, which the target does not call; the mixed coding's tag bit read at the end of the
data, which the `exhausted` check before every line makes unreachable; and the early return at the
top of `two_dimensional_line`'s loop, which the check at the bottom of the same loop always reaches
first. So the 223 edges are the decoder's reachable edges, and a target that stops gaining them is a
decoder fully exercised, not a fuzzer stuck at its first branch. A 600 s run behind the lock
(this round's record) started at 223 and ended at 223, with no finding.

## 3. What would reopen it

A change to `pdf-ccitt` that adds a branch — a new coding, a new concealment rule — or a decision to
make the two unreachable returns `debug_assert!`s instead; either is a reason to run the target
again, and neither is a reason to reseed it.
