# 2112 — The walks refuse a run the wall clock decided, and the in-process decode is budgeted in processor time

Session 2112, an instruments round in `pdf-viewer-audit` at `4a66dc26` on round 2108's three leftovers. ADR 3149 is the
walks', ADR 3150 the in-process budget's, ADR 3151 the waiter's start; ADR 3152 and Q934 unused.

**The walks (ADR 3149).** A census of every `--ignored` walk `doc/todo/02` §2 names found thirteen whose verdict an
image decoded in the test's own process can move: `pdf-model`'s `corpus`, `oracle`, `fixed_documents`,
`text_extraction` and `drawn_while_read` (read in pieces, two interpretations compared); `render-raster`'s `corpus`;
`viewer-core`'s two censuses; `pdf-transform`'s `split`, `merge`, `pages` and `optimize` corpus walks and `gate`'s oracle
page. Each reads `pdf_model::image::wall_clock_refusals()` after its figures are printed and refuses the run before any
assertion. Plant: one refusal added to the count, each walk cut to three documents (the oracle and `render-raster` by
their `ONLY` variables) — all thirteen ended at the guard; with the count restored, none did (their own ratchets failed
on the cut populations, as they must).

**The in-process budget (ADR 3150).** Buildable without `unsafe`: the decoding thread names itself through the
`/proc/thread-self` link and the caller reads `/proc/self/task/<tid>/stat`. `clock::Allowance` now serves both paths;
spending the budget is the new `SandboxError::Exhausted` (the image's, not counted), the tenfold ceiling `Overran` (the
machine's). Proof, a release build of one 8192² JBIG2 region (1.1 s of processor), budget 1.5 s, pinned to one
processor at `nice 3` beside a busy loop: before, `Overran { after: 1.5s }` at 1.51 s, counted (twice); after, decoded at
2.57 s and 2.65 s. After, idle at 0.5 s: `Exhausted`, uncounted; its process stopped 17 s: `Overran { after: 15s }`,
counted.

**The waiter (ADR 3151).** `--start` waits for its pid file or its runner's end, read once more after; 240 s is a
ceiling on neither. Two self-test cases with a `setsid` shim (held until `late.go`, six seconds after it is reached; or
exiting at once): the old `start` fails both, the new passes both.

**Trap.** A `pgrep -f` for the proof's binary matched the shell that ran it, and the `SIGSTOP` stopped that; stall a
process by the `$!` that started it (2108's trap, again).

**Gates.** Over `-p pdf-sandbox -p pdf-model -p render-raster -p viewer-core -p pdf-transform -p conformance`: fmt 0;
clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 100 (5488 of 5489:
`records`, this record's gates unwritten; `expected-failures-current.json` lists none, so no difference); rustdoc 101
first (public docs linked the private `clock` and `STALL_FACTOR`, 2108's slip again), 0 after; `fuzz` `cargo check
--bins` 0; `cargo test -p conformance` 101 (`records`), `--test records` 0 after this line. `wait.sh --self-test` 0.

**Left.** `gate`'s two hundred pages are drawn by `quorra-transform` in a child whose count the test cannot read: the
program would have to print it. `foreign_corpus` and `awkward_classes` decode in other processes and judge nothing an
image moves today.
