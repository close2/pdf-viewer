# 1902 — A run the reader salvages is written as the number it draws

Batch 131, slot 2 (archive), four sessions on Opus 5.5. ADR 2638.

**What changed.** A content-stream run ISO 32000-2 §7.3.3 spells as no number and the reader draws
as one (ADR 1004: `33600.272.58` → `33600.272`, a cut stream's `454` and 85h → `454`) is written as
that number under the new loss `content-as-drawn` (`archive/as_drawn.rs`,
`Rewrite::RunWrittenAsDrawn`). The operator row of the content-stream fences moved from the by-name
refusals to a remedy row granted per document; `as-if-printed` and `as-shown` take it with
`discard`, `state-every-loss` with `append-statement`, whose page names each run with its page,
stream and number. At most 65536 runs a document are written.

**509284.** `as-if-printed` converts at 2b and 4 with 58 runs written and pages 4, 9, 14 and 18 as
text; page 2 stays photographed, its one unlisted operator a `)` where no string began, which is no
run: four of the brief's five pages. veraPDF 1.30.3 passes both outputs. `as-shown` refuses on the
`)` as at HEAD.

**The plant.** The 2b output round-tripped through `qpdf` with `33600.272` put back as
`33600.272.58` fails the checker on the operator row (exit 4); the control passes (exit 0). veraPDF
does not flag the planted run, lexing it as no operator. The bound: 10^6 runs, HEAD exit 4 at 28.8
MiB, this tree exit 4 at 27.7 MiB (366.9 MiB before the bound); 65536 runs, HEAD exit 4 at 17.8 MiB,
this tree exit 0 at 41.2 MiB in 0.11 s (`getrusage`).

**Gates.** `cargo test -p pdf-transform --test archive --test profiles` exit 0 (536 and 13 passed, 6
ignored); `cargo clippy --no-deps -p pdf-transform -p pdf-model --all-targets -- -D warnings` exit
101 on one red, `rewrite_dictionary` at 117 of 100 lines, carried past by round 1904's hunk;
`rustfmt --edition 2024 --check` exit 0 on this round's files; `--bin cited` exit 0; `--bin
quotations` exit 0 (3049 quotations, 2851 verbatim, 3 diverging, none this round's); `cargo test -p
conformance --no-fail-fast` exit 101 (469 passed, 1 failed, 69 binaries: `records` on round 1905's
record). Census against HEAD's, no pack, exit 0: all twelve total lines unchanged; the moved lines
are `as-shown` refusal reasons only — `064034` from the fence to this round's sentence (a stream
also holding an odd-digit hexadecimal string), and `073439`, `226387`, `507618` and `509284` out of
the TrueType rows' "refused by name" groups (at 4 also `027613`, `268346`, `477139` and `507676`),
each still refused by the fence; 218 s against HEAD's 183 s. With the pack, exit 0: the same moves
and no other, 273 s against 188 s. `rank.py` exit 0.

**Found, not fixed.** The operator row answering with the fence no longer short-circuits a refused
file's TrueType rows, whose programs are then copied (`decision/content_stream_fences.rs`); a stream
holding a run and an odd-digit hexadecimal string takes neither rewrite (`archive/as_drawn.rs`).
